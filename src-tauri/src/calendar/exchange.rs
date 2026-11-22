//! CALENDAR phase 6: moving events in and out as `.ics` files. The file format lives in `ics.rs`;
//! this is the database side and the two file dialogs.
//!
//! Import is two steps: choose a file and see what it holds (nothing is written), then confirm. The
//! parsed file is kept here between the steps, so the page never sends events back to be saved.
use super::ics::{self, ExportDated, ExportEvent, Imported, Parsed};
use super::repeat::{Change, Repeat};
use super::store::{self, Event};
use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use mysql::{prelude::Queryable, Conn};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// The most events the calendar may hold, so an import cannot fill the database.
pub const MAX_EVENTS_TOTAL: u64 = 10_000;
const MAX_SHOWN: usize = 100;
const MAX_EXPORT: usize = 5000;

static PENDING: Mutex<Option<(u64, Parsed, Vec<bool>)>> = Mutex::new(None);

fn sql(what: &str) -> impl Fn(mysql::Error) -> String + '_ {
    move |_| format!("{what} failed. Start the memory database in Settings > Database.")
}

pub fn own_event_id(uid: &str) -> Option<u64> {
    uid.strip_prefix(ics::OWN_EVENT)?.strip_suffix(ics::OWN_DOMAIN)?.parse().ok()
}

/// For each imported entry: is it already in the calendar? An entry matches by its id, and Castle's own
/// exported ids match the event they came from.
pub fn duplicates(c: &mut Conn, items: &[Imported]) -> Result<Vec<bool>, String> {
    let mut known: HashSet<String> = HashSet::new();
    let mut own: HashSet<u64> = HashSet::new();
    for chunk in items.chunks(200) {
        let quoted: Vec<String> = chunk.iter().map(|i| format!("'{}'", i.uid.replace('\\', "\\\\").replace('\'', "''"))).collect();
        if quoted.is_empty() {
            continue;
        }
        let rows: Vec<String> = c.query(format!("SELECT ics_uid FROM calendar_events WHERE ics_uid IN ({})", quoted.join(","))).map_err(sql("Checking the file"))?;
        known.extend(rows);
        let ids: Vec<String> = chunk.iter().filter_map(|i| own_event_id(&i.uid)).map(|n| n.to_string()).collect();
        if !ids.is_empty() {
            let found: Vec<u64> = c.query(format!("SELECT id FROM calendar_events WHERE id IN ({})", ids.join(","))).map_err(sql("Checking the file"))?;
            own.extend(found);
        }
    }
    Ok(items.iter().map(|i| known.contains(&i.uid) || own_event_id(&i.uid).is_some_and(|n| own.contains(&n))).collect())
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub all_day: bool,
    pub repeat: Option<Repeat>,
    pub duplicate: bool,
    pub warning: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: u64,
    pub file_name: String,
    pub total: usize,
    pub fresh: usize,
    pub duplicates: usize,
    pub own_dated: usize,
    pub warnings: Vec<String>,
    pub shown: Vec<PreviewItem>,
    pub more: usize,
}

pub fn preview_of(parsed: &Parsed, dup: &[bool], token: u64, file_name: &str) -> Preview {
    let all: Vec<PreviewItem> = parsed
        .events
        .iter()
        .zip(dup)
        .map(|(i, d)| PreviewItem { title: i.event.title.clone(), starts_at: i.event.starts_at.clone(), ends_at: i.event.ends_at.clone(), all_day: i.event.all_day, repeat: i.event.repeat.clone(), duplicate: *d, warning: i.warning.clone() })
        .collect();
    let duplicates = dup.iter().filter(|d| **d).count();
    let more = all.len().saturating_sub(MAX_SHOWN);
    Preview { token, file_name: file_name.chars().filter(|c| !c.is_control()).take(120).collect(), total: all.len(), fresh: all.len() - duplicates, duplicates, own_dated: parsed.own_dated, warnings: parsed.warnings.clone(), shown: all.into_iter().take(MAX_SHOWN).collect(), more }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub imported: usize,
    pub duplicates: usize,
    pub refused: usize,
}

/// Write the new entries. All or nothing: a failure part-way leaves the calendar as it was.
pub fn import(c: &mut Conn, parsed: &Parsed, dup: &[bool]) -> Result<Summary, String> {
    let fresh = dup.iter().filter(|d| !**d).count() as u64;
    let have: Option<u64> = c.query_first("SELECT COUNT(*) FROM calendar_events").map_err(sql("Importing"))?;
    if have.unwrap_or(0) + fresh > MAX_EVENTS_TOTAL {
        return Err(format!("The calendar may hold at most {MAX_EVENTS_TOTAL} events; this file would go past that."));
    }
    c.query_drop("START TRANSACTION").map_err(sql("Importing"))?;
    let done = (|| -> Result<Summary, String> {
        let mut out = Summary { imported: 0, duplicates: 0, refused: 0 };
        for (item, is_dup) in parsed.events.iter().zip(dup) {
            if *is_dup {
                out.duplicates += 1;
                continue;
            }
            // the same checks as an event the member types, so nothing odd gets in
            let Ok(ok) = store::check(item.event.clone()) else {
                out.refused += 1;
                continue;
            };
            let id = store::insert_imported(c, &ok, &item.uid)?;
            for d in &item.skips {
                store::add_exception(c, id, *d, None)?;
            }
            for (original, new_start) in &item.moves {
                if crate::calendar::repeat::in_range(new_start.date()) {
                    store::add_exception(c, id, *original, Some(*new_start))?;
                }
            }
            out.imported += 1;
        }
        Ok(out)
    })();
    match done {
        Ok(s) => {
            if c.query_drop("COMMIT").is_err() {
                let _ = c.query_drop("ROLLBACK");
                return Err("The import could not be finished, so nothing was added.".into());
            }
            Ok(s)
        }
        Err(e) => {
            let _ = c.query_drop("ROLLBACK");
            Err(e)
        }
    }
}

/// The text of an export, and how many events it holds.
pub fn export(c: &mut Conn, include_dated: bool, now_utc: NaiveDateTime) -> Result<(String, usize), String> {
    let events: Vec<Event> = store::all_active(c, MAX_EXPORT)?;
    let uids: HashMap<u64, String> = c
        .query::<(u64, Option<String>), _>("SELECT id,ics_uid FROM calendar_events WHERE state='active'")
        .map_err(sql("Exporting"))?
        .into_iter()
        .filter_map(|(id, uid)| uid.map(|u| (id, u)))
        .collect();
    let changes = store::exceptions(c, &events)?;
    let x: Vec<ExportEvent> = events
        .iter()
        .map(|e| {
            let own: Vec<&(u64, NaiveDate, Change)> = changes.iter().filter(|(id, _, _)| *id == e.id).collect();
            ExportEvent {
                uid: uids.get(&e.id).cloned().unwrap_or_else(|| format!("{}{}{}", ics::OWN_EVENT, e.id, ics::OWN_DOMAIN)),
                event: e.clone(),
                skips: own.iter().filter(|(_, _, ch)| matches!(ch, Change::Skip)).map(|(_, d, _)| *d).collect(),
                moves: own.iter().filter_map(|(_, d, ch)| if let Change::Move(t) = ch { Some((*d, *t)) } else { None }).collect(),
            }
        })
        .collect();
    let mut dated: Vec<ExportDated> = Vec::new();
    if include_dated {
        let rows: Vec<(u64, String, String)> = c
            .query("SELECT id,title,DATE_FORMAT(due_date,'%Y-%m-%d') FROM reminders WHERE state='open' AND due_date IS NOT NULL ORDER BY due_date LIMIT 2000")
            .map_err(sql("Exporting"))?;
        for (id, title, day) in rows {
            if let Ok(date) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
                dated.push(ExportDated { uid: format!("{}{}{}", ics::OWN_REMINDER, id, ics::OWN_DOMAIN), title, date, kind: "Reminder" });
            }
        }
        let rows: Vec<(u64, String, String)> = c
            .query("SELECT t.id,t.title,DATE_FORMAT(t.due_date,'%Y-%m-%d') FROM project_tasks t JOIN projects p ON p.id=t.project_id WHERE t.done_at IS NULL AND t.due_date IS NOT NULL AND p.shelf='active' ORDER BY t.due_date LIMIT 2000")
            .map_err(sql("Exporting"))?;
        for (id, title, day) in rows {
            if let Ok(date) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
                dated.push(ExportDated { uid: format!("{}{}{}", ics::OWN_TASK, id, ics::OWN_DOMAIN), title, date, kind: "Task" });
            }
        }
    }
    let count = x.len() + dated.len();
    Ok((ics::write(&x, &dated, now_utc), count))
}

fn to_local(t: NaiveDateTime) -> NaiveDateTime {
    Local.from_utc_datetime(&t).naive_local()
}

fn token() -> u64 {
    Utc::now().timestamp_nanos_opt().unwrap_or(1) as u64
}

/// Choose a file and read it; nothing is written yet. None when the picker was closed.
#[tauri::command]
pub async fn calendar_ics_preview(app: AppHandle) -> Result<Option<Preview>, String> {
    super::blocking(move || {
        let mut c = super::connect(&app)?;
        let Some(path) = app.dialog().file().set_title("Choose a calendar file").add_filter("Calendar files", &["ics"]).blocking_pick_file().and_then(|p| p.into_path().ok()) else {
            return Ok(None);
        };
        let size = std::fs::metadata(&path).map_err(|_| "That file could not be read.".to_string())?.len();
        if size as usize > ics::MAX_BYTES {
            return Err("That file is bigger than 2 MB, which is more than a calendar needs.".into());
        }
        let bytes = std::fs::read(&path).map_err(|_| "That file could not be read.".to_string())?;
        let text = String::from_utf8_lossy(&bytes);
        let parsed = ics::parse(text.trim_start_matches('\u{feff}'), &to_local);
        if parsed.events.is_empty() && parsed.own_dated == 0 {
            return Err(if parsed.warnings.is_empty() { "No events were found in that file.".into() } else { format!("No events could be read from that file. {}", parsed.warnings.join(" ")) });
        }
        let dup = duplicates(&mut c, &parsed.events)?;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let t = token();
        let preview = preview_of(&parsed, &dup, t, &name);
        *PENDING.lock().unwrap_or_else(|p| p.into_inner()) = Some((t, parsed, dup));
        Ok(Some(preview))
    })
    .await
}

/// Add what the preview showed. The preview must be the latest one.
#[tauri::command]
pub async fn calendar_ics_import(app: AppHandle, token: u64) -> Result<Summary, String> {
    let (parsed, _) = {
        let mut slot = PENDING.lock().unwrap_or_else(|p| p.into_inner());
        match slot.take() {
            Some((t, parsed, dup)) if t == token => (parsed, dup),
            other => {
                *slot = other;
                return Err("That preview has expired. Choose the file again.".into());
            }
        }
    };
    super::blocking(move || {
        let mut c = super::connect(&app)?;
        // look again: something may have been added since the preview
        let dup = duplicates(&mut c, &parsed.events)?;
        // imported entries carry no reminder and no automation link, so there is nothing to settle
        import(&mut c, &parsed, &dup)
    })
    .await
}

/// Forget a preview that was not confirmed.
#[tauri::command]
pub fn calendar_ics_discard() {
    *PENDING.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Exported {
    pub path: String,
    pub count: usize,
}

/// Save the member's events (and, if asked, their dated reminders and tasks) where they choose.
#[tauri::command]
pub async fn calendar_ics_export(app: AppHandle, include_dated: bool) -> Result<Option<Exported>, String> {
    super::blocking(move || {
        let mut c = super::connect(&app)?;
        let name = format!("castle-calendar-{}.ics", Local::now().format("%Y-%m-%d"));
        let Some(path) = app.dialog().file().set_title("Save the calendar").set_file_name(&name).add_filter("Calendar files", &["ics"]).blocking_save_file().and_then(|p| p.into_path().ok()) else {
            return Ok(None);
        };
        let (text, count) = export(&mut c, include_dated, Utc::now().naive_utc())?;
        std::fs::write(&path, text).map_err(|_| "The file could not be saved there.".to_string())?;
        Ok(Some(Exported { path: path.to_string_lossy().to_string(), count }))
    })
    .await
}

