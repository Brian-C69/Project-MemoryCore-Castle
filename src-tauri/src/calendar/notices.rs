//! CALENDAR phase 3: a reminder before an event. One notice per occurrence, ever: the row that
//! records it is claimed before anything is shown, so a restart, a second tick or a slow database
//! can never repeat one. A reminder missed while Castle was closed is shown once on the next start,
//! at most five of them, and the rest as one line.
use super::repeat::{self, Exception};
use super::store::{self, Event};
use chrono::{Duration, NaiveDateTime};
use mysql::{prelude::Queryable, Conn};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// A reminder whose time passed longer ago than this is never raised: it is stale news.
pub const LOOKBACK_MIN: i64 = 12 * 60;
/// Later than this and a reminder counts as "missed while Castle was closed".
pub const LATE_AFTER_SEC: i64 = 150;
/// How many missed reminders are shown one by one; the rest are one summary line.
pub const MAX_LATE_SHOWN: usize = 5;
/// How many on-time reminders one tick raises; a pile-up beyond this is summarised too.
pub const MAX_ON_TIME: usize = 10;
/// How long a notice stays on the HOME card unless dismissed.
const SHOW_HOURS: i64 = 24;

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub event_id: u64,
    pub title: String,
    pub lead: u32,
    pub start: NaiveDateTime,
    pub all_day: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    pub event_id: u64,
    pub title: String,
    pub lead: u32,
    pub start: NaiveDateTime,
    pub due_at: NaiveDateTime,
    pub late: bool,
    pub all_day: bool,
}

/// Which occurrences have reached their reminder time and are still worth saying.
pub fn pick(candidates: &[Candidate], now: NaiveDateTime) -> Vec<Due> {
    candidates
        .iter()
        .filter_map(|c| {
            let due_at = c.start - Duration::minutes(c.lead as i64);
            if due_at > now || due_at <= now - Duration::minutes(LOOKBACK_MIN) {
                return None;
            }
            Some(Due { event_id: c.event_id, title: c.title.clone(), lead: c.lead, start: c.start, due_at, late: (now - due_at).num_seconds() > LATE_AFTER_SEC, all_day: c.all_day })
        })
        .collect()
}

/// Split into what is shown one by one and what is only counted.
pub fn plan(dues: Vec<Due>) -> (Vec<Due>, Vec<Due>) {
    let (mut late, mut on_time): (Vec<Due>, Vec<Due>) = dues.into_iter().partition(|d| d.late);
    on_time.sort_by_key(|d| (d.due_at, d.event_id));
    late.sort_by_key(|d| std::cmp::Reverse((d.due_at, d.event_id)));
    let mut shown: Vec<Due> = Vec::new();
    let mut counted: Vec<Due> = Vec::new();
    for (n, d) in on_time.into_iter().enumerate() {
        if n < MAX_ON_TIME { shown.push(d) } else { counted.push(d) }
    }
    for (n, d) in late.into_iter().enumerate() {
        if n < MAX_LATE_SHOWN { shown.push(d) } else { counted.push(d) }
    }
    (shown, counted)
}

pub fn lead_text(min: u32) -> String {
    let plural = |n: u32, unit: &str| if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") };
    if min >= 10080 && min % 10080 == 0 {
        plural(min / 10080, "week")
    } else if min >= 1440 && min % 1440 == 0 {
        plural(min / 1440, "day")
    } else if min >= 60 && min % 60 == 0 {
        plural(min / 60, "hour")
    } else {
        plural(min, "minute")
    }
}

fn day_name(t: NaiveDateTime) -> String {
    t.format("%a %-d %b").to_string()
}

/// What the reminder says, as a short line (the title of the event is the member's own words).
pub fn line(d: &Due) -> String {
    let at = if d.all_day { day_name(d.start) } else if d.start.date() == d.due_at.date() { d.start.format("%H:%M").to_string() } else { format!("{} {}", day_name(d.start), d.start.format("%H:%M")) };
    if d.late {
        format!("Missed while Castle was closed: {} ({})", d.title, at)
    } else if d.lead == 0 {
        format!("{} - now ({})", d.title, at)
    } else {
        format!("{} - in {} ({})", d.title, lead_text(d.lead), at)
    }
}

pub fn summary_line(count: usize) -> String {
    if count == 1 { "1 more reminder came while Castle was closed.".into() } else { format!("{count} more reminders came while Castle was closed.") }
}

/// What a tick found and newly claimed.
#[derive(Debug, Default)]
pub struct Delivery {
    pub shown: Vec<Due>,
    pub counted: usize,
}

/// Claim one occurrence's notice. Returns true only for the caller that made the row.
fn claim(c: &mut Conn, d: &Due, hidden: bool, silent: bool) -> Result<bool, String> {
    c.exec_drop(
        "INSERT IGNORE INTO calendar_notices (event_id,occurrence_start,due_at,title,lead_min,late,hidden,dismissed_at) VALUES (?,?,?,?,?,?,?,IF(?,NOW(),NULL))",
        (d.event_id, d.start.format("%Y-%m-%d %H:%M:00").to_string(), d.due_at.format("%Y-%m-%d %H:%M:%S").to_string(), d.title.chars().take(160).collect::<String>(), d.lead, d.late, hidden, silent),
    )
    .map_err(|_| "A reminder could not be recorded.".to_string())?;
    Ok(c.affected_rows() == 1)
}

pub(super) fn candidates(c: &mut Conn, events: &[Event], now: NaiveDateTime, lead_of: fn(&Event) -> Option<u32>) -> Result<Vec<Candidate>, String> {
    let changes = store::exceptions(c, events)?;
    let mut out = Vec::new();
    for e in events {
        let (Some(lead), Ok(series)) = (lead_of(e), store::series_of(e)) else { continue };
        let own: Vec<Exception> = changes.iter().filter(|(id, _, _)| *id == e.id).map(|(_, d, ch)| Exception { date: *d, change: ch.clone() }).collect();
        let from = (now - Duration::minutes(LOOKBACK_MIN)).date();
        let to = (now + Duration::minutes(lead as i64)).date();
        for o in repeat::occurrences(&series, &own, from, to) {
            out.push(Candidate { event_id: e.id, title: e.title.clone(), lead, start: o.start, all_day: e.all_day });
        }
    }
    Ok(out)
}

/// One pass: find what is due, claim it, and return what is new. No toast is made here.
pub fn collect(c: &mut Conn, now: NaiveDateTime) -> Result<Delivery, String> {
    let events = store::with_reminders(c)?;
    let due = pick(&candidates(c, &events, now, |e| e.remind_before_min)?, now);
    let (shown, counted) = plan(due);
    let mut out = Delivery::default();
    for d in shown {
        if claim(c, &d, false, false)? {
            out.shown.push(d);
        }
    }
    for d in counted {
        if claim(c, &d, true, false)? {
            out.counted += 1;
        }
    }
    Ok(out)
}

/// After the member saves or moves an event: reminders already past are theirs to know about, so
/// they are recorded as seen and never raised.
pub fn settle(c: &mut Conn, event_id: u64, now: NaiveDateTime) -> Result<(), String> {
    let events: Vec<Event> = store::with_reminders(c)?.into_iter().filter(|e| e.id == event_id).collect();
    for d in pick(&candidates(c, &events, now, |e| e.remind_before_min)?, now) {
        claim(c, &d, true, true)?;
    }
    Ok(())
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: u64,
    pub event_id: u64,
    pub title: String,
    pub start: String,
    pub late: bool,
    pub text: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub notices: Vec<Notice>,
    /// reminders that were only counted (not shown one by one) and not yet dismissed
    pub more: u64,
    /// the next few events, so the card is never an empty box
    pub next: Vec<super::Item>,
    /// the companion's suggestions waiting for a yes
    pub proposed: u64,
    pub problem: Option<String>,
}

pub fn home(c: &mut Conn, now: NaiveDateTime) -> Result<Home, String> {
    let rows: Vec<(u64, u64, String, String, String, u32, bool)> = c
        .exec(
            "SELECT id,event_id,title,DATE_FORMAT(occurrence_start,'%Y-%m-%d %H:%i'),DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s'),lead_min,late FROM calendar_notices WHERE hidden=0 AND dismissed_at IS NULL AND sent_at > ? ORDER BY sent_at DESC,id DESC LIMIT 5",
            ((now - Duration::hours(SHOW_HOURS)).format("%Y-%m-%d %H:%M:%S").to_string(),),
        )
        .map_err(|_| "Reminders could not be read.".to_string())?;
    let notices = rows
        .into_iter()
        .map(|(id, event_id, title, start, due, lead, late)| {
            let s = NaiveDateTime::parse_from_str(&start, "%Y-%m-%d %H:%M").unwrap_or(now);
            let d = NaiveDateTime::parse_from_str(&due, "%Y-%m-%d %H:%M:%S").unwrap_or(now);
            let text = line(&Due { event_id, title: title.clone(), lead, start: s, due_at: d, late, all_day: s.format("%H:%M").to_string() == "00:00" });
            Notice { id, event_id, title, start, late, text }
        })
        .collect();
    let more: Option<u64> = c
        .exec_first(
            "SELECT COUNT(*) FROM calendar_notices WHERE hidden=1 AND dismissed_at IS NULL AND sent_at > ?",
            ((now - Duration::hours(SHOW_HOURS)).format("%Y-%m-%d %H:%M:%S").to_string(),),
        )
        .map_err(|_| "Reminders could not be read.".to_string())?;
    let today = now.date();
    let mut next: Vec<super::Item> = store::items(c, today, today + Duration::days(7))?
        .into_iter()
        .filter(|i| i.date.as_str() > today.to_string().as_str() || i.time.as_deref().is_none_or(|t| t >= now.format("%H:%M").to_string().as_str()))
        .collect();
    next.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.time.cmp(&b.time)));
    next.truncate(3);
    let proposed: Option<u64> = c.query_first("SELECT COUNT(*) FROM calendar_events WHERE state='proposed'").map_err(|_| "Proposals could not be read.".to_string())?;
    Ok(Home { notices, more: more.unwrap_or(0), next, proposed: proposed.unwrap_or(0), problem: problem() })
}

pub fn dismiss(c: &mut Conn, id: Option<u64>) -> Result<(), String> {
    let done = match id {
        Some(id) => c.exec_drop("UPDATE calendar_notices SET dismissed_at=NOW() WHERE id=? AND dismissed_at IS NULL", (id,)),
        None => c.query_drop("UPDATE calendar_notices SET dismissed_at=NOW() WHERE dismissed_at IS NULL"),
    };
    done.map_err(|_| "The reminder could not be dismissed.".to_string())
}

static BUSY: AtomicBool = AtomicBool::new(false);
static TICKS: AtomicU64 = AtomicU64::new(0);
static PROBLEM: Mutex<Option<String>> = Mutex::new(None);

pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}
fn set_problem(p: Option<String>) {
    if let Ok(mut slot) = PROBLEM.lock() {
        *slot = p;
    }
}

fn run(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<crate::settings::SettingsStore>();
    if !settings.get().room_on("calendar") {
        return Ok(());
    }
    let mut c = crate::automations::room_connection(&settings)?;
    let ready: Option<String> = c.exec_first("SELECT name FROM schema_migrations WHERE name=?", ("025_calendar_subscriptions",)).map_err(|_| "The memory database is preparing.".to_string())?;
    if ready.is_none() {
        return Ok(());
    }
    let now = chrono::Local::now().naive_local();
    // an event that starts an automation does it on this same tick, whether or not it also reminds
    super::link::run(app, &mut c, now);
    let found = collect(&mut c, now)?;
    for d in &found.shown {
        crate::roomalert::toast(app, "Calendar", &line(d), "calendar");
    }
    if found.counted > 0 {
        crate::roomalert::toast(app, "Calendar", &summary_line(found.counted), "calendar");
    }
    if !found.shown.is_empty() || found.counted > 0 {
        let _ = app.emit("castle://calendar", serde_json::json!({"notices": found.shown.len() + found.counted}));
    }
    // once an hour, notices older than 30 days go
    if TICKS.fetch_add(1, Ordering::Relaxed) % 240 == 0 {
        let _ = c.query_drop("DELETE FROM calendar_notices WHERE sent_at < DATE_SUB(NOW(), INTERVAL 30 DAY)");
        // a suggestion turned down stays 30 days (so it is not suggested again), then goes
        let _ = c.query_drop("DELETE FROM calendar_events WHERE state='cancelled' AND updated_at < DATE_SUB(NOW(), INTERVAL 30 DAY)");
    }
    Ok(())
}

/// Called from the automations runner's 15 second tick. It runs on its own short thread, one at a
/// time, so a slow database never delays the runner, and it does not depend on AUTOMATIONS being
/// on or paused: a reminder the member asked for is not an automation.
pub fn tick(app: &AppHandle) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        set_problem(run(&app).err());
        BUSY.store(false, Ordering::Release);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }
    fn cand(id: u64, start: &str, lead: u32) -> Candidate {
        Candidate { event_id: id, title: format!("Event {id}"), lead, start: t(start), all_day: false }
    }

    #[test]
    fn a_reminder_is_due_from_its_time_and_not_before() {
        let c = [cand(1, "2026-11-27 15:00:00", 30)];
        assert!(pick(&c, t("2026-11-27 14:29:59")).is_empty());
        let got = pick(&c, t("2026-11-27 14:30:00"));
        assert_eq!(got.len(), 1);
        assert!(!got[0].late);
        assert_eq!(got[0].due_at, t("2026-11-27 14:30:00"));
    }

    #[test]
    fn a_reminder_a_few_seconds_late_is_still_on_time_and_minutes_late_is_missed() {
        let c = [cand(1, "2026-11-27 15:00:00", 30)];
        assert!(!pick(&c, t("2026-11-27 14:32:00"))[0].late, "two minutes: the tick was a little slow");
        assert!(pick(&c, t("2026-11-27 14:33:00"))[0].late, "three minutes is missed");
        assert!(pick(&c, t("2026-11-27 16:00:00"))[0].late);
    }

    #[test]
    fn a_reminder_older_than_twelve_hours_is_never_raised() {
        let c = [cand(1, "2026-11-27 15:00:00", 0)];
        assert_eq!(pick(&c, t("2026-11-28 02:59:59")).len(), 1);
        assert!(pick(&c, t("2026-11-28 03:00:00")).is_empty());
    }

    #[test]
    fn a_zero_lead_fires_at_the_start() {
        let c = [cand(1, "2026-11-27 15:00:00", 0)];
        assert!(pick(&c, t("2026-11-27 14:59:59")).is_empty());
        assert_eq!(pick(&c, t("2026-11-27 15:00:00")).len(), 1);
    }

    #[test]
    fn plan_shows_five_missed_newest_first_and_counts_the_rest() {
        let c: Vec<Candidate> = (1..=8).map(|n| cand(n, &format!("2026-11-27 0{n}:00:00"), 0)).collect();
        let dues = pick(&c, t("2026-11-27 09:00:00"));
        assert!(dues.iter().all(|d| d.late));
        let (shown, counted) = plan(dues);
        assert_eq!(shown.iter().map(|d| d.event_id).collect::<Vec<_>>(), vec![8, 7, 6, 5, 4]);
        assert_eq!(counted.len(), 3);
    }

    #[test]
    fn plan_keeps_on_time_ones_first_and_caps_a_pile_up() {
        let mut c: Vec<Candidate> = (1..=12).map(|n| cand(n, "2026-11-27 09:00:00", 0)).collect();
        c.push(cand(99, "2026-11-27 06:00:00", 0));
        let dues = pick(&c, t("2026-11-27 09:00:10"));
        let (shown, counted) = plan(dues);
        assert_eq!(shown.len(), MAX_ON_TIME + 1, "ten on time and the one missed");
        assert_eq!(counted.len(), 2);
        assert_eq!(shown.last().unwrap().event_id, 99, "missed ones come after the on-time ones");
    }

    #[test]
    fn lead_words() {
        assert_eq!(lead_text(1), "1 minute");
        assert_eq!(lead_text(30), "30 minutes");
        assert_eq!(lead_text(60), "1 hour");
        assert_eq!(lead_text(90), "90 minutes");
        assert_eq!(lead_text(120), "2 hours");
        assert_eq!(lead_text(1440), "1 day");
        assert_eq!(lead_text(2880), "2 days");
        assert_eq!(lead_text(10080), "1 week");
    }

    #[test]
    fn the_lines_say_what_and_when() {
        let on = pick(&[cand(1, "2026-11-27 15:00:00", 30)], t("2026-11-27 14:30:05")).remove(0);
        assert_eq!(line(&on), "Event 1 - in 30 minutes (15:00)");
        let now = pick(&[cand(1, "2026-11-27 15:00:00", 0)], t("2026-11-27 15:00:05")).remove(0);
        assert_eq!(line(&now), "Event 1 - now (15:00)");
        let missed = pick(&[cand(1, "2026-11-27 15:00:00", 30)], t("2026-11-27 16:00:00")).remove(0);
        assert_eq!(line(&missed), "Missed while Castle was closed: Event 1 (15:00)");
        let day_before = pick(&[cand(1, "2026-11-28 09:00:00", 1440)], t("2026-11-27 09:00:05")).remove(0);
        assert_eq!(line(&day_before), "Event 1 - in 1 day (Sat 28 Nov 09:00)");
        let all_day = Candidate { all_day: true, ..cand(2, "2026-11-28 00:00:00", 1440) };
        let d = pick(&[all_day], t("2026-11-27 00:00:05")).remove(0);
        assert_eq!(line(&d), "Event 2 - in 1 day (Sat 28 Nov)");
        assert_eq!(summary_line(1), "1 more reminder came while Castle was closed.");
        assert_eq!(summary_line(3), "3 more reminders came while Castle was closed.");
    }

    /// Opt-in isolated database: the generated test database and user are created and dropped here.
    /// `CASTLE_TEST_DB=port:admin:password`, or the installed database's port and stored root key.
    #[test]
    #[ignore]
    fn live_database_notices_once_catch_up_settle_dismiss_and_cascade() {
        let config = std::env::var("CASTLE_TEST_DB").unwrap_or_else(|_| {
            let folder = std::env::var("APPDATA").unwrap() + "/com.memorycore.castle/settings.json";
            let port = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(folder).unwrap()).unwrap()["dbPort"].as_u64().unwrap();
            format!("{port}:root:{}", crate::key::db_root().unwrap_or_default())
        });
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let (user, pass) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &user, &pass, &format!("Notices Test {}", std::process::id())).unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        let seen: Option<String> = c.query_first("SELECT name FROM schema_migrations WHERE name='022_calendar_notices'").unwrap();
        assert!(seen.is_some());
        let make = |c: &mut Conn, title: &str, start: &str, lead: u32, repeat: Option<super::super::repeat::Repeat>| {
            let e = Event { id: 0, title: title.into(), note: String::new(), starts_at: start.into(), ends_at: None, all_day: false, repeat, remind_before_min: Some(lead), colour: "accent".into(), automation_job_id: None, automation_lead_min: None, state: "active".into(), created_by: "member".into() };
            store::save(c, &store::check(e).unwrap()).unwrap()
        };

        // one reminder: nothing before its time, one notice at it, never a second, also after a "restart"
        let dentist = make(&mut c, "Dentist", "2026-11-27 15:00", 30, None);
        assert!(collect(&mut c, t("2026-11-27 14:29:00")).unwrap().shown.is_empty());
        let first = collect(&mut c, t("2026-11-27 14:30:03")).unwrap();
        assert_eq!((first.shown.len(), first.counted), (1, 0));
        assert_eq!(line(&first.shown[0]), "Dentist - in 30 minutes (15:00)");
        assert!(collect(&mut c, t("2026-11-27 14:30:18")).unwrap().shown.is_empty(), "the next tick says nothing");
        drop(c);
        let mut c = crate::memdb::open(&access).unwrap();
        assert!(collect(&mut c, t("2026-11-27 14:45:00")).unwrap().shown.is_empty(), "a new connection and a later time still say nothing");

        // an event with no reminder never speaks
        let plain = Event { id: 0, title: "Quiet".into(), note: String::new(), starts_at: "2026-11-27 15:00".into(), ends_at: None, all_day: false, repeat: None, remind_before_min: None, colour: "accent".into(), automation_job_id: None, automation_lead_min: None, state: "active".into(), created_by: "member".into() };
        store::save(&mut c, &store::check(plain).unwrap()).unwrap();
        assert!(collect(&mut c, t("2026-11-27 15:00:00")).unwrap().shown.iter().all(|d| d.title != "Quiet"));

        // a repeating event: each Monday has its own notice; a skipped Monday has none
        let weekly = super::super::repeat::Repeat { every: "week".into(), interval: 1, weekdays: vec![], until: None, count: None };
        let gym = make(&mut c, "Gym", "2026-11-02 18:30", 15, Some(weekly));
        store::change_day(&mut c, gym, chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(), None).unwrap();
        assert_eq!(collect(&mut c, t("2026-11-02 18:15:05")).unwrap().shown.len(), 1);
        assert!(collect(&mut c, t("2026-11-16 18:15:05")).unwrap().shown.is_empty(), "skipped");
        assert_eq!(collect(&mut c, t("2026-11-23 18:15:05")).unwrap().shown.len(), 1, "the next Monday is a new occurrence");
        // a moved Monday reminds at its new time
        store::change_day(&mut c, gym, chrono::NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(), Some(t("2026-12-01 07:00:00"))).unwrap();
        assert!(collect(&mut c, t("2026-11-30 18:15:05")).unwrap().shown.is_empty());
        assert_eq!(collect(&mut c, t("2026-12-01 06:45:05")).unwrap().shown.len(), 1);

        // catch-up after Castle was closed: eight reminders, five shown, three counted, once only
        for n in 1..=8 {
            make(&mut c, &format!("Away {n}"), &format!("2026-12-10 0{n}:00"), 0, None);
        }
        let away = collect(&mut c, t("2026-12-10 09:30:00")).unwrap();
        assert_eq!((away.shown.len(), away.counted), (5, 3));
        assert!(away.shown.iter().all(|d| d.late));
        let again = collect(&mut c, t("2026-12-10 09:30:15")).unwrap();
        assert_eq!((again.shown.len(), again.counted), (0, 0));

        // the HOME card sees the visible ones and the count of the rest, and dismissing clears them
        let real = chrono::Local::now().naive_local();
        let home1 = home(&mut c, real).unwrap();
        assert_eq!(home1.notices.len(), 5);
        assert_eq!(home1.more, 3);
        assert!(home1.notices.iter().all(|n| n.late && n.text.starts_with("Missed while Castle was closed")));
        let gone = home1.notices[0].id;
        dismiss(&mut c, Some(gone)).unwrap();
        let rest = home(&mut c, real).unwrap();
        assert!(rest.notices.iter().all(|n| n.id != gone), "the dismissed one is gone; an older one takes its place on the card");
        dismiss(&mut c, None).unwrap();
        let cleared = home(&mut c, real).unwrap();
        assert_eq!((cleared.notices.len(), cleared.more), (0, 0));

        // saving an event whose reminder time has already passed does not ring: it is settled as seen
        let soon = make(&mut c, "Soon", "2026-12-20 10:00", 60, None);
        settle(&mut c, soon, t("2026-12-20 09:50:00")).unwrap();
        assert!(collect(&mut c, t("2026-12-20 09:50:15")).unwrap().shown.is_empty());
        assert_eq!(home(&mut c, real).unwrap().notices.len(), 0);
        // the upcoming list shows the next events, soonest first, at most three
        let soonest = (real + Duration::days(2)).format("%Y-%m-%d 10:00").to_string();
        make(&mut c, "Next A", &soonest, 0, None);
        let later = (real + Duration::days(3)).format("%Y-%m-%d 10:00").to_string();
        make(&mut c, "Next B", &later, 0, None);
        let far = (real + Duration::days(20)).format("%Y-%m-%d 10:00").to_string();
        make(&mut c, "Far away", &far, 0, None);
        let next = home(&mut c, real).unwrap().next;
        let names: Vec<&str> = next.iter().map(|i| i.title.as_str()).collect();
        assert!(names.len() <= 3 && names.contains(&"Next A") && names.contains(&"Next B"), "{names:?}");
        assert!(!names.contains(&"Far away"), "twenty days away is not next");
        assert!(names.iter().position(|n| *n == "Next A") < names.iter().position(|n| *n == "Next B"), "soonest first");

        // a reminder older than twelve hours is not raised on a late start
        make(&mut c, "Old", "2026-12-25 08:00", 0, None);
        assert!(collect(&mut c, t("2026-12-25 21:00:00")).unwrap().shown.is_empty());

        // deleting an event takes its notices with it
        let before: Option<u64> = c.exec_first("SELECT COUNT(*) FROM calendar_notices WHERE event_id=?", (dentist,)).unwrap();
        assert_eq!(before, Some(1));
        store::delete(&mut c, dentist).unwrap();
        let after: Option<u64> = c.exec_first("SELECT COUNT(*) FROM calendar_notices WHERE event_id=?", (dentist,)).unwrap();
        assert_eq!(after, Some(0));

        c.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        let mut root = Conn::new(mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(user.as_str())).pass(Some(pass.as_str()))).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user)).unwrap();
        }
    }
}
