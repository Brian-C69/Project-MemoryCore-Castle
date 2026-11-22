//! CALENDAR: one page for everything in Castle that has a date.
//! Phase 1 read reminders, project tasks, diary pages and automation runs where they live.
//! Phase 2 adds the member's own events (with repeats), written only to their own two tables.
use crate::settings::SettingsStore;
use chrono::{NaiveDate, NaiveDateTime};
use mysql::{prelude::Queryable, Conn};
use serde::Serialize;
use tauri::{AppHandle, Manager};

pub mod companion;
pub mod exchange;
#[cfg(test)]
mod exchange_live;
mod ics;
mod link;
#[cfg(test)]
mod link_live;
pub use link::unlink_job;
pub mod notices;
mod quickadd;
mod repeat;
mod store;
pub mod subscriptions;
#[cfg(test)]
mod subscriptions_live;

/// The widest window one request may ask for, so a bad range cannot read the whole database.
pub const MAX_DAYS: i64 = 400;
/// The most items one request may return.
pub const MAX_ITEMS: usize = 2000;
const KINDS: [&str; 6] = ["holiday", "event", "reminder", "task", "automation", "diary"];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// event | reminder | task | automation | diary
    pub kind: String,
    pub id: u64,
    pub title: String,
    /// YYYY-MM-DD, local time
    pub date: String,
    /// HH:MM when the item has a time of day
    pub time: Option<String>,
    /// open | done | planned | ok | failed | active (events)
    pub state: String,
    /// The room that owns this item: clicking opens it. Events are owned by this room.
    pub owner: String,
    /// A second line: the project of a task, the status of a run.
    pub detail: String,
    pub all_day: bool,
    /// HH:MM when a timed event ends the same day it starts
    pub end_time: Option<String>,
    /// the day a repeating event's occurrence would have had without a move
    pub occurrence: Option<String>,
    pub repeating: bool,
    /// a colour name for an event (accent, soft, plain)
    pub colour: Option<String>,
    /// false for the second and later day of a multi-day event, which cannot be dragged alone
    pub first: bool,
}

impl Item {
    fn plain(kind: &str, id: u64, title: &str, date: String, time: Option<String>, state: &str, owner: &str, detail: String) -> Item {
        Item { kind: kind.into(), id, title: clip(title, 160), date, time, state: state.into(), owner: owner.into(), detail, all_day: false, end_time: None, occurrence: None, repeating: false, colour: None, first: true }
    }
}

/// Parse and check a window: both ends real dates, from not after to, at most MAX_DAYS wide.
pub fn window(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let a = NaiveDate::parse_from_str(from, "%Y-%m-%d").map_err(|_| "The start date is not a date.".to_string())?;
    let b = NaiveDate::parse_from_str(to, "%Y-%m-%d").map_err(|_| "The end date is not a date.".to_string())?;
    if b < a {
        return Err("The end date is before the start date.".into());
    }
    if (b - a).num_days() >= MAX_DAYS {
        return Err(format!("Ask for at most {MAX_DAYS} days at a time."));
    }
    Ok((a, b))
}

fn kind_rank(kind: &str) -> usize {
    KINDS.iter().position(|k| *k == kind).unwrap_or(KINDS.len())
}

/// Sort by day, then all-day items before timed ones, then time, kind and title; keep at most `cap`.
pub fn merge(mut items: Vec<Item>, cap: usize) -> Vec<Item> {
    items.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.time.is_some().cmp(&b.time.is_some()))
            .then_with(|| a.time.cmp(&b.time))
            .then_with(|| kind_rank(&a.kind).cmp(&kind_rank(&b.kind)))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
    items.truncate(cap);
    items
}

fn clip(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}

fn read_items(c: &mut Conn, from: NaiveDate, to: NaiveDate) -> Result<Vec<Item>, String> {
    let (a, b) = (from.to_string(), to.to_string());
    let (start, end) = (format!("{a} 00:00:00"), format!("{b} 23:59:59"));
    let sad = |what: &str| format!("{what} could not be read. Start the memory database in Settings > Database.");
    let mut out: Vec<Item> = Vec::new();
    let rows: Vec<(u64, String, String, String)> = c
        .exec(
            "SELECT id,title,DATE_FORMAT(due_date,'%Y-%m-%d'),state FROM reminders WHERE due_date BETWEEN ? AND ? AND state IN ('open','done') ORDER BY due_date,id LIMIT 500",
            (&a, &b),
        )
        .map_err(|_| sad("Reminders"))?;
    for (id, title, date, state) in rows {
        out.push(Item::plain("reminder", id, &title, date, None, &state, "home", String::new()));
    }
    let rows: Vec<(u64, String, String, String, bool)> = c
        .exec(
            "SELECT t.id,t.title,DATE_FORMAT(t.due_date,'%Y-%m-%d'),p.name,t.done_at IS NOT NULL FROM project_tasks t JOIN projects p ON p.id=t.project_id WHERE t.due_date BETWEEN ? AND ? AND p.shelf='active' ORDER BY t.due_date,t.id LIMIT 500",
            (&a, &b),
        )
        .map_err(|_| sad("Tasks"))?;
    for (id, title, date, project, done) in rows {
        out.push(Item::plain("task", id, &title, date, None, if done { "done" } else { "open" }, "projects", clip(&project, 80)));
    }
    let rows: Vec<(u64, String, String)> = c
        .exec(
            "SELECT id,title,DATE_FORMAT(entry_date,'%Y-%m-%d') FROM diary_entries WHERE entry_date BETWEEN ? AND ? ORDER BY entry_date,session_no LIMIT 500",
            (&a, &b),
        )
        .map_err(|_| sad("Diary"))?;
    for (id, title, date) in rows {
        out.push(Item::plain("diary", id, &title, date, None, "done", "diary", String::new()));
    }
    // The next run of every switched-on job, and every run that already happened.
    let rows: Vec<(u64, String, String, String)> = c
        .exec(
            "SELECT id,name,DATE_FORMAT(next_run_at,'%Y-%m-%d'),DATE_FORMAT(next_run_at,'%H:%i') FROM automation_jobs WHERE state='active' AND enabled=1 AND next_run_at BETWEEN ? AND ? ORDER BY next_run_at LIMIT 300",
            (&start, &end),
        )
        .map_err(|_| sad("Automations"))?;
    for (id, title, date, time) in rows {
        out.push(Item::plain("automation", id, &title, date, Some(time), "planned", "automations", "Next run".into()));
    }
    let rows: Vec<(u64, String, String, String, String)> = c
        .exec(
            "SELECT r.id,j.name,DATE_FORMAT(r.started_at,'%Y-%m-%d'),DATE_FORMAT(r.started_at,'%H:%i'),r.status FROM automation_runs r JOIN automation_jobs j ON j.id=r.job_id WHERE r.started_at BETWEEN ? AND ? ORDER BY r.started_at DESC LIMIT 300",
            (&start, &end),
        )
        .map_err(|_| sad("Automation runs"))?;
    for (id, title, date, time, status) in rows {
        let state = if status == "ok" || status == "success" { "ok" } else if status == "running" { "planned" } else { "failed" };
        out.push(Item::plain("automation", id, &title, date, Some(time), state, "automations", clip(&format!("Run: {status}"), 40)));
    }
    out.extend(store::items(c, from, to)?);
    out.extend(subscriptions::items(c, from, to)?);
    Ok(out)
}

/// A connection for a calendar command: the room must be on and the tables must exist.
fn connect(app: &AppHandle) -> Result<Conn, String> {
    let settings = app.state::<SettingsStore>();
    if !settings.get().room_on("calendar") {
        return Err("CALENDAR is switched off in Settings > Rooms.".to_string());
    }
    let mut c = crate::automations::room_connection(&settings)?;
    let ready: Option<String> = c
        .exec_first("SELECT name FROM schema_migrations WHERE name=?", ("025_calendar_subscriptions",))
        .map_err(|_| "The memory database is preparing. Try again in a moment.".to_string())?;
    ready.ok_or("The calendar is being set up. Restart Castle and try again.".to_string())?;
    Ok(c)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| "The calendar could not be reached.".to_string())?
}

#[tauri::command]
pub async fn calendar_range(app: AppHandle, from: String, to: String) -> Result<Vec<Item>, String> {
    let (a, b) = window(&from, &to)?;
    blocking(move || {
        let mut c = connect(&app)?;
        Ok(merge(read_items(&mut c, a, b)?, MAX_ITEMS))
    })
    .await
}

#[tauri::command]
pub async fn calendar_event_get(app: AppHandle, id: u64) -> Result<store::Event, String> {
    blocking(move || store::get(&mut connect(&app)?, id)).await
}

#[tauri::command]
pub async fn calendar_event_save(app: AppHandle, event: store::Event) -> Result<u64, String> {
    let ok = store::check(event)?;
    blocking(move || {
        let mut c = connect(&app)?;
        link::check_link(&mut c, &ok.event)?;
        let id = store::save(&mut c, &ok)?;
        // a reminder or an automation start that has already passed is the member's own news: no toast, no late run
        let now = chrono::Local::now().naive_local();
        let _ = notices::settle(&mut c, id, now);
        let _ = link::settle(&mut c, id, now);
        Ok(id)
    })
    .await
}

#[tauri::command]
pub async fn calendar_event_delete(app: AppHandle, id: u64) -> Result<(), String> {
    blocking(move || store::delete(&mut connect(&app)?, id)).await
}

fn day_of(text: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| "That is not a day.".to_string())
}

/// Move an event. A one-off event moves whole; for a repeating one, `occurrence` names the single
/// day that moves and the others stay.
#[tauri::command]
pub async fn calendar_event_move(app: AppHandle, id: u64, occurrence: Option<String>, new_starts_at: String) -> Result<(), String> {
    let new = NaiveDateTime::parse_from_str(new_starts_at.trim(), "%Y-%m-%d %H:%M").map_err(|_| "That is not a date and time.".to_string())?;
    let day = occurrence.as_deref().map(day_of).transpose()?;
    blocking(move || {
        let mut c = connect(&app)?;
        match day {
            Some(d) => store::change_day(&mut c, id, d, Some(new)),
            None => store::move_whole(&mut c, id, new),
        }?;
        let now = chrono::Local::now().naive_local();
        let _ = notices::settle(&mut c, id, now);
        let _ = link::settle(&mut c, id, now);
        Ok(())
    })
    .await
}

/// The jobs an event may start: approved, and waiting for a calendar event.
#[tauri::command]
pub async fn calendar_link_jobs(app: AppHandle) -> Result<Vec<link::JobChoice>, String> {
    blocking(move || link::eligible(&mut connect(&app)?)).await
}

/// Skip one day of a repeating event (`undo` brings it, or a moved day, back to the rule).
#[tauri::command]
pub async fn calendar_event_skip(app: AppHandle, id: u64, occurrence: String, undo: bool) -> Result<(), String> {
    let day = day_of(&occurrence)?;
    blocking(move || {
        let mut c = connect(&app)?;
        if undo { store::restore_day(&mut c, id, day) } else { store::change_day(&mut c, id, day, None) }
    })
    .await
}

/// The companion's suggestions waiting for a yes.
#[tauri::command]
pub async fn calendar_proposals(app: AppHandle) -> Result<Vec<store::Event>, String> {
    blocking(move || store::proposals(&mut connect(&app)?)).await
}

/// Approve a suggestion (with the member's own edits, if any) or turn it down. Only the member can: the
/// companion has no verb for this.
#[tauri::command]
pub async fn calendar_proposal_decide(app: AppHandle, id: u64, approve: bool, event: Option<store::Event>) -> Result<(), String> {
    let edited = match event {
        Some(e) if approve => Some(store::check(e)?),
        _ => None,
    };
    blocking(move || {
        let mut c = connect(&app)?;
        if let Some(ok) = &edited {
            link::check_link(&mut c, &ok.event)?;
        }
        store::decide(&mut c, id, approve, edited.as_ref())?;
        if approve {
            let now = chrono::Local::now().naive_local();
            let _ = notices::settle(&mut c, id, now);
            let _ = link::settle(&mut c, id, now);
        }
        Ok(())
    })
    .await
}

/// HOME's CALENDAR card: the reminders raised lately and the next few events.
#[tauri::command]
pub async fn calendar_home(app: AppHandle) -> Result<notices::Home, String> {
    blocking(move || notices::home(&mut connect(&app)?, chrono::Local::now().naive_local())).await
}

/// Dismiss one reminder from the card, or all of them (`id` empty).
#[tauri::command]
pub async fn calendar_notice_dismiss(app: AppHandle, id: Option<u64>) -> Result<(), String> {
    blocking(move || notices::dismiss(&mut connect(&app)?, id)).await
}

#[derive(Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsInfo {
    pub subs: Vec<subscriptions::Sub>,
    pub presets: Vec<Preset>,
    pub states: Vec<&'static str>,
}

fn subs_info(c: &mut Conn) -> Result<SubsInfo, String> {
    Ok(SubsInfo { subs: subscriptions::list(c)?, presets: subscriptions::PRESETS.iter().map(|(id, name, _)| Preset { id, name }).collect(), states: subscriptions::MALAYSIA_STATES.to_vec() })
}

/// The calendars the member subscribes to (holidays), and the ones on offer.
#[tauri::command]
pub async fn calendar_subs(app: AppHandle) -> Result<SubsInfo, String> {
    blocking(move || subs_info(&mut connect(&app)?)).await
}

/// Subscribe to a calendar: a preset (`malaysia`) or a name and a web address. It is fetched once at
/// once; if that fails the calendar stays added with a quiet note, and Castle tries again later.
#[tauri::command]
pub async fn calendar_sub_add(app: AppHandle, preset: Option<String>, name: Option<String>, url: Option<String>) -> Result<SubsInfo, String> {
    blocking(move || {
        let mut c = connect(&app)?;
        let (name, url, tag) = match preset.as_deref() {
            Some(p) => {
                let (id, name, url) = subscriptions::PRESETS.iter().find(|(id, _, _)| *id == p).ok_or("That calendar is not on offer.")?;
                (name.to_string(), url.to_string(), id.to_string())
            }
            None => (name.unwrap_or_default(), url.unwrap_or_default(), String::new()),
        };
        let id = subscriptions::add(&mut c, &name, &url, &tag)?;
        let now = chrono::Local::now().naive_local();
        let _ = subscriptions::refresh(&mut c, id, now.date(), now);
        subs_info(&mut c)
    })
    .await
}

/// Switch a calendar on or off, and choose the state whose own holidays show too.
#[tauri::command]
pub async fn calendar_sub_set(app: AppHandle, id: u64, enabled: bool, region: String) -> Result<SubsInfo, String> {
    blocking(move || {
        let mut c = connect(&app)?;
        subscriptions::set(&mut c, id, enabled, &region)?;
        subs_info(&mut c)
    })
    .await
}

#[tauri::command]
pub async fn calendar_sub_remove(app: AppHandle, id: u64) -> Result<SubsInfo, String> {
    blocking(move || {
        let mut c = connect(&app)?;
        subscriptions::remove(&mut c, id)?;
        subs_info(&mut c)
    })
    .await
}

/// Fetch a calendar again now. A failure is also kept on the calendar as its quiet note.
#[tauri::command]
pub async fn calendar_sub_refresh(app: AppHandle, id: u64) -> Result<SubsInfo, String> {
    blocking(move || {
        let mut c = connect(&app)?;
        let now = chrono::Local::now().naive_local();
        let _ = subscriptions::refresh(&mut c, id, now.date(), now);
        subs_info(&mut c)
    })
    .await
}

/// What a quick-add line means, shown before anything is saved. Pure: no database.
#[tauri::command]
pub fn calendar_quickadd_preview(text: String) -> quickadd::Quick {
    let short: String = text.chars().take(300).collect();
    quickadd::parse(&short, chrono::Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(kind: &str, id: u64, title: &str, date: &str, time: Option<&str>) -> Item {
        Item::plain(kind, id, title, date.into(), time.map(String::from), "open", "home", String::new())
    }
    #[test]
    fn a_window_must_be_real_ordered_and_bounded() {
        assert!(window("2026-11-01", "2026-11-30").is_ok());
        assert!(window("2026-11-01", "2026-11-01").is_ok());
        assert!(window("2026-11-30", "2026-11-01").is_err());
        assert!(window("2026-13-01", "2026-11-01").is_err());
        assert!(window("tomorrow", "2026-11-01").is_err());
        assert!(window("2026-02-30", "2026-03-01").is_err());
        assert!(window("2026-01-01", "2027-02-05").is_err(), "400 days or more is refused");
        assert!(window("2026-01-01", "2027-02-03").is_ok(), "399 days is allowed");
    }
    #[test]
    fn merging_sorts_by_day_then_all_day_first_then_time_then_kind() {
        let got = merge(
            vec![
                item("automation", 1, "Briefing", "2026-11-22", Some("09:00")),
                item("diary", 2, "Page", "2026-11-22", None),
                item("reminder", 3, "Call", "2026-11-21", None),
                item("task", 4, "Write", "2026-11-22", None),
                item("automation", 5, "Early", "2026-11-22", Some("07:30")),
                item("event", 6, "Dentist", "2026-11-22", None),
            ],
            100,
        );
        let order: Vec<u64> = got.iter().map(|i| i.id).collect();
        assert_eq!(order, vec![3, 6, 4, 2, 5, 1], "an event leads the all-day items of its day");
    }
    #[test]
    fn merging_keeps_at_most_the_cap_from_the_start_of_the_window() {
        let items = (0..10).map(|n| item("reminder", n, "x", &format!("2026-11-{:02}", n + 1), None)).collect();
        let got = merge(items, 3);
        assert_eq!(got.len(), 3);
        assert_eq!(got[2].date, "2026-11-03");
    }
    #[test]
    fn titles_lose_control_characters_and_are_cut() {
        assert_eq!(clip("a\u{0}b\nc", 10), "abc");
        assert_eq!(clip(&"x".repeat(500), 160).len(), 160);
    }
    #[test]
    fn the_migration_has_the_two_tables_and_no_link_that_could_block_a_drop() {
        let sql = include_str!("../../resources/schema/021_calendar.sql");
        let found = crate::memdb::statements(sql);
        assert_eq!(found.len(), 2);
        assert!(found[0].starts_with("CREATE TABLE calendar_events"));
        assert!(found[1].starts_with("CREATE TABLE calendar_exceptions"));
        assert!(!found[0].contains("FOREIGN KEY"), "the automation link is a plain column");
        assert!(!sql.contains("GENERATED"));
    }
}
