//! Member events: checking them, saving them, and reading them back as dated items.
use super::repeat::{self, Change, Exception, Repeat, Series};
use super::Item;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use mysql::{prelude::Queryable, Conn};
use serde::{Deserialize, Serialize};

/// The colours an event may carry: names that the room maps to theme variables, never raw colours.
pub const COLOURS: [&str; 3] = ["accent", "soft", "plain"];
const MAX_NOTE: usize = 4000;
const MAX_TITLE: usize = 160;
const MAX_LENGTH_DAYS: i64 = 400;
const FORMAT: &str = "%Y-%m-%d %H:%M";

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    #[serde(default)]
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub note: String,
    /// "YYYY-MM-DD HH:MM", local time; an all-day event keeps 00:00
    pub starts_at: String,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default)]
    pub repeat: Option<Repeat>,
    #[serde(default)]
    pub remind_before_min: Option<u32>,
    #[serde(default = "accent")]
    pub colour: String,
    /// an automation job this event starts (a job whose trigger is the calendar event)
    #[serde(default)]
    pub automation_job_id: Option<u64>,
    /// minutes before the event that job starts; 0 is at the event time
    #[serde(default)]
    pub automation_lead_min: Option<u32>,
    /// active | proposed | cancelled; read from the database, never accepted from the page
    #[serde(skip_deserializing, default = "active")]
    pub state: String,
    /// member | companion | import; read from the database, never accepted from the page
    #[serde(skip_deserializing, default = "member")]
    pub created_by: String,
}
fn accent() -> String {
    "accent".into()
}
fn active() -> String {
    "active".into()
}
fn member() -> String {
    "member".into()
}

/// An event that passed every check, with its times parsed.
pub struct Checked {
    pub event: Event,
    pub start: NaiveDateTime,
}

fn clean(text: &str) -> String {
    text.chars().filter(|c| !c.is_control() || *c == '\n').collect::<String>().trim().to_string()
}

fn when(text: &str, what: &str) -> Result<NaiveDateTime, String> {
    let t = NaiveDateTime::parse_from_str(text.trim(), FORMAT)
        .or_else(|_| NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
        .map_err(|_| format!("The {what} is not a date and time."))?;
    if !repeat::in_range(t.date()) {
        return Err(format!("The {what} is too far away."));
    }
    Ok(t)
}

pub fn check(mut e: Event) -> Result<Checked, String> {
    e.title = clean(&e.title).replace('\n', " ");
    if e.title.is_empty() {
        return Err("Give the event a title.".into());
    }
    if e.title.chars().count() > MAX_TITLE {
        return Err(format!("Keep the title under {MAX_TITLE} characters."));
    }
    e.note = clean(&e.note);
    if e.note.chars().count() > MAX_NOTE {
        return Err(format!("Keep the note under {MAX_NOTE} characters."));
    }
    if !COLOURS.contains(&e.colour.as_str()) {
        return Err("Choose one of the listed colours.".into());
    }
    if e.remind_before_min.is_some_and(|m| m > 14 * 1440) {
        return Err("A reminder can come at most 14 days before.".into());
    }
    if e.automation_job_id == Some(0) {
        e.automation_job_id = None;
    }
    match e.automation_job_id {
        None => e.automation_lead_min = None,
        Some(_) => {
            let lead = e.automation_lead_min.unwrap_or(0);
            if lead > 14 * 1440 {
                return Err("An automation can start at most 14 days before.".into());
            }
            e.automation_lead_min = Some(lead);
        }
    }
    let mut start = when(&e.starts_at, "start")?;
    let mut end = match e.ends_at.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => Some(when(t, "end")?),
        None => None,
    };
    if e.all_day {
        start = start.date().and_hms_opt(0, 0, 0).unwrap();
        end = end.map(|t| t.date().and_hms_opt(0, 0, 0).unwrap());
        if end == Some(start) {
            end = None;
        }
    }
    if let Some(t) = end {
        if t < start {
            return Err("The event ends before it starts.".into());
        }
        if t - start > Duration::days(MAX_LENGTH_DAYS) {
            return Err("An event can last at most 400 days.".into());
        }
    }
    if let Some(r) = &e.repeat {
        repeat::validate(r, start.date())?;
    }
    e.starts_at = start.format(FORMAT).to_string();
    e.ends_at = end.map(|t| t.format(FORMAT).to_string());
    Ok(Checked { event: e, start })
}

fn sql(what: &str) -> impl Fn(mysql::Error) -> String + '_ {
    move |_| format!("{what} failed. Start the memory database in Settings > Database.")
}

type Row = mysql::Row;
const COLUMNS: &str = "id,title,note,DATE_FORMAT(starts_at,'%Y-%m-%d %H:%i'),DATE_FORMAT(ends_at,'%Y-%m-%d %H:%i'),all_day,CAST(repeat_json AS CHAR),remind_before_min,colour,state,created_by,automation_job_id,automation_lead_min";

fn col<T: mysql::prelude::FromValue>(r: &mut Row, i: usize) -> Result<T, String> {
    r.take_opt(i).ok_or_else(|| "An event could not be read.".to_string())?.map_err(|_| "An event could not be read.".to_string())
}

fn from_row(mut r: Row) -> Result<Event, String> {
    let rule: Option<String> = col(&mut r, 6)?;
    let repeat = match rule {
        Some(text) => Some(serde_json::from_str::<Repeat>(&text).map_err(|_| "A repeat rule could not be read.".to_string())?),
        None => None,
    };
    Ok(Event {
        id: col(&mut r, 0)?,
        title: col(&mut r, 1)?,
        note: col(&mut r, 2)?,
        starts_at: col(&mut r, 3)?,
        ends_at: col(&mut r, 4)?,
        all_day: col(&mut r, 5)?,
        repeat,
        remind_before_min: col(&mut r, 7)?,
        colour: col(&mut r, 8)?,
        automation_job_id: col(&mut r, 11)?,
        automation_lead_min: col(&mut r, 12)?,
        state: col(&mut r, 9)?,
        created_by: col(&mut r, 10)?,
    })
}

pub fn get(c: &mut Conn, id: u64) -> Result<Event, String> {
    let row: Option<Row> = c
        .exec_first(format!("SELECT {COLUMNS} FROM calendar_events WHERE id=? AND state IN ('active','proposed')"), (id,))
        .map_err(sql("Reading the event"))?;
    from_row(row.ok_or("That event no longer exists.")?)
}

/// Save a checked event; returns its id. An edit that changes the first day or the repeat rule
/// forgets the one-off changes made to single days, since they no longer line up.
pub fn save(c: &mut Conn, ok: &Checked) -> Result<u64, String> {
    let e = &ok.event;
    let repeat = e.repeat.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default());
    let end = e.ends_at.clone();
    if e.id > 0 {
        let before: Option<(String, Option<String>)> = c
            .exec_first("SELECT DATE_FORMAT(starts_at,'%Y-%m-%d'),CAST(repeat_json AS CHAR) FROM calendar_events WHERE id=? AND state IN ('active','proposed')", (e.id,))
            .map_err(sql("Saving the event"))?;
        let (day, rule) = before.ok_or("That event no longer exists.")?;
        let same_rule = match (&rule, &repeat) {
            (None, None) => true,
            (Some(a), Some(b)) => serde_json::from_str::<Repeat>(a).ok() == serde_json::from_str::<Repeat>(b).ok(),
            _ => false,
        };
        c.exec_drop(
            "UPDATE calendar_events SET title=?,note=?,starts_at=?,ends_at=?,all_day=?,repeat_json=?,remind_before_min=?,colour=?,automation_job_id=?,automation_lead_min=? WHERE id=? AND state IN ('active','proposed')",
            (&e.title, &e.note, ok.start.format("%Y-%m-%d %H:%M:00").to_string(), end.map(|t| format!("{t}:00")), e.all_day, repeat, e.remind_before_min, &e.colour, e.automation_job_id, e.automation_lead_min, e.id),
        )
        .map_err(sql("Saving the event"))?;
        if day != ok.start.format("%Y-%m-%d").to_string() || !same_rule {
            c.exec_drop("DELETE FROM calendar_exceptions WHERE event_id=?", (e.id,)).map_err(sql("Saving the event"))?;
        }
        Ok(e.id)
    } else {
        c.exec_drop(
            "INSERT INTO calendar_events (title,note,starts_at,ends_at,all_day,repeat_json,remind_before_min,colour,automation_job_id,automation_lead_min,created_by) VALUES (?,?,?,?,?,?,?,?,?,?,'member')",
            (&e.title, &e.note, ok.start.format("%Y-%m-%d %H:%M:00").to_string(), end.map(|t| format!("{t}:00")), e.all_day, repeat, e.remind_before_min, &e.colour, e.automation_job_id, e.automation_lead_min),
        )
        .map_err(sql("Saving the event"))?;
        Ok(c.last_insert_id())
    }
}

/// The most proposals that may wait at once; more is a sign something is wrong, not a wish.
pub const MAX_PROPOSED: u64 = 20;

/// The companion's suggestion: stored as `proposed`, by `companion`. It makes no notice and shows only as
/// a suggestion until the member approves it in CALENDAR.
pub fn propose(c: &mut Conn, ok: &Checked, why: &str) -> Result<u64, String> {
    let waiting: Option<u64> = c.query_first("SELECT COUNT(*) FROM calendar_events WHERE state='proposed'").map_err(sql("Proposing the event"))?;
    if waiting.unwrap_or(0) >= MAX_PROPOSED {
        return Err("Too many proposals are already waiting. Your person needs to look at CALENDAR first.".into());
    }
    let same: Option<u64> = c
        .exec_first(
            "SELECT COUNT(*) FROM calendar_events WHERE title=? AND starts_at=? AND (state IN ('active','proposed') OR (state='cancelled' AND updated_at > DATE_SUB(NOW(), INTERVAL 30 DAY)))",
            (&ok.event.title, ok.start.format("%Y-%m-%d %H:%M:00").to_string()),
        )
        .map_err(sql("Proposing the event"))?;
    if same.unwrap_or(0) > 0 {
        return Err("That is already on the calendar, already proposed, or was turned down lately.".into());
    }
    let e = &ok.event;
    let repeat = e.repeat.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default());
    let note: String = if why.trim().is_empty() { String::new() } else { format!("Why: {}", clean(why)).chars().take(MAX_NOTE).collect() };
    c.exec_drop(
        "INSERT INTO calendar_events (title,note,starts_at,ends_at,all_day,repeat_json,remind_before_min,colour,state,created_by) VALUES (?,?,?,?,?,?,?,?,'proposed','companion')",
        (&e.title, note, ok.start.format("%Y-%m-%d %H:%M:00").to_string(), e.ends_at.as_ref().map(|t| format!("{t}:00")), e.all_day, repeat, e.remind_before_min, &e.colour),
    )
    .map_err(sql("Proposing the event"))?;
    Ok(c.last_insert_id())
}

/// Proposals waiting for a yes, oldest first.
pub fn proposals(c: &mut Conn) -> Result<Vec<Event>, String> {
    let rows: Vec<Row> = c
        .query(format!("SELECT {COLUMNS} FROM calendar_events WHERE state='proposed' ORDER BY created_at,id LIMIT 50"))
        .map_err(sql("Reading proposals"))?;
    rows.into_iter().map(from_row).collect()
}

/// The member's decision on a proposal: approve it (optionally with their edits) or turn it down.
/// Only a proposal that is still waiting can be decided.
pub fn decide(c: &mut Conn, id: u64, approve: bool, edited: Option<&Checked>) -> Result<(), String> {
    let now: Option<String> = c.exec_first("SELECT state FROM calendar_events WHERE id=?", (id,)).map_err(sql("Deciding"))?;
    if now.as_deref() != Some("proposed") {
        return Err("That proposal is no longer waiting.".into());
    }
    if approve {
        if let Some(ok) = edited {
            save(c, &Checked { event: Event { id, ..ok.event.clone() }, start: ok.start })?;
        }
        c.exec_drop("UPDATE calendar_events SET state='active' WHERE id=? AND state='proposed'", (id,)).map_err(sql("Approving"))?;
    } else {
        c.exec_drop("UPDATE calendar_events SET state='cancelled' WHERE id=? AND state='proposed'", (id,)).map_err(sql("Turning it down"))?;
    }
    Ok(())
}

/// Delete a whole event (its repeat and its changed days go with it).
pub fn delete(c: &mut Conn, id: u64) -> Result<(), String> {
    c.exec_drop("DELETE FROM calendar_events WHERE id=?", (id,)).map_err(sql("Deleting the event"))?;
    if c.affected_rows() == 0 {
        return Err("That event no longer exists.".into());
    }
    Ok(())
}

pub(super) fn series_of(e: &Event) -> Result<Series, String> {
    let start = NaiveDateTime::parse_from_str(&e.starts_at, FORMAT).map_err(|_| "The event's start could not be read.".to_string())?;
    let end = e.ends_at.as_deref().and_then(|t| NaiveDateTime::parse_from_str(t, FORMAT).ok());
    Ok(Series { start, end, repeat: e.repeat.clone() })
}

/// Skip one day of a repeating event, or move it; `new_start` is None for a skip.
pub fn change_day(c: &mut Conn, id: u64, day: NaiveDate, new_start: Option<NaiveDateTime>) -> Result<(), String> {
    let e = get(c, id)?;
    if e.state != "active" {
        return Err("Approve the proposal first.".into());
    }
    let s = series_of(&e)?;
    let r = s.repeat.as_ref().ok_or("Only a repeating event has single days to change.")?;
    if !repeat::dates(s.start.date(), r, day).contains(&day) {
        return Err("That day is not part of the repeat.".into());
    }
    if let Some(t) = new_start {
        if !repeat::in_range(t.date()) {
            return Err("That day is too far away.".into());
        }
    }
    c.exec_drop(
        "INSERT INTO calendar_exceptions (event_id,occurrence_date,kind,new_starts_at) VALUES (?,?,?,?) ON DUPLICATE KEY UPDATE kind=VALUES(kind),new_starts_at=VALUES(new_starts_at)",
        (id, day.to_string(), if new_start.is_some() { "move" } else { "skip" }, new_start.map(|t| t.format("%Y-%m-%d %H:%M:00").to_string())),
    )
    .map_err(sql("Changing the day"))?;
    Ok(())
}

/// Undo a skip or a move of one day.
pub fn restore_day(c: &mut Conn, id: u64, day: NaiveDate) -> Result<(), String> {
    c.exec_drop("DELETE FROM calendar_exceptions WHERE event_id=? AND occurrence_date=?", (id, day.to_string())).map_err(sql("Changing the day"))?;
    Ok(())
}

/// Move a one-off event to a new start, keeping its length.
pub fn move_whole(c: &mut Conn, id: u64, new_start: NaiveDateTime) -> Result<(), String> {
    let mut e = get(c, id)?;
    if e.state != "active" {
        return Err("Approve the proposal first.".into());
    }
    if e.repeat.is_some() {
        return Err("Move one day of a repeating event, or edit the whole event.".into());
    }
    let s = series_of(&e)?;
    let length = s.end.map(|t| t - s.start);
    e.starts_at = new_start.format(FORMAT).to_string();
    e.ends_at = length.map(|l| (new_start + l).format(FORMAT).to_string());
    let ok = check(e)?;
    save(c, &ok)?;
    Ok(())
}

fn hm(t: NaiveDateTime) -> String {
    t.format("%H:%M").to_string()
}

/// An entry read from a calendar file: made by `import`, keeping the id it came with so a second import
/// finds it. It has no reminder and no automation link, whatever the file said.
pub fn insert_imported(c: &mut Conn, ok: &Checked, uid: &str) -> Result<u64, String> {
    let e = &ok.event;
    let repeat = e.repeat.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default());
    c.exec_drop(
        "INSERT INTO calendar_events (title,note,starts_at,ends_at,all_day,repeat_json,colour,created_by,ics_uid) VALUES (?,?,?,?,?,?,'accent','import',?)",
        (&e.title, &e.note, ok.start.format("%Y-%m-%d %H:%M:00").to_string(), e.ends_at.as_ref().map(|t| format!("{t}:00")), e.all_day, repeat, uid),
    )
    .map_err(sql("Importing"))?;
    Ok(c.last_insert_id())
}

/// Record a skipped day (no `new_start`) or a moved day of a repeating event.
pub fn add_exception(c: &mut Conn, id: u64, day: NaiveDate, new_start: Option<NaiveDateTime>) -> Result<(), String> {
    c.exec_drop(
        "INSERT IGNORE INTO calendar_exceptions (event_id,occurrence_date,kind,new_starts_at) VALUES (?,?,?,?)",
        (id, day.to_string(), if new_start.is_some() { "move" } else { "skip" }, new_start.map(|t| t.format("%Y-%m-%d %H:%M:00").to_string())),
    )
    .map_err(sql("Importing"))?;
    Ok(())
}

/// Every active event, for an export.
pub(super) fn all_active(c: &mut Conn, limit: usize) -> Result<Vec<Event>, String> {
    let rows: Vec<Row> = c
        .exec(format!("SELECT {COLUMNS} FROM calendar_events WHERE state='active' ORDER BY id LIMIT ?"), (limit,))
        .map_err(sql("Exporting"))?;
    rows.into_iter().map(from_row).collect()
}

/// Every active event that starts an automation.
pub(super) fn with_automation(c: &mut Conn) -> Result<Vec<Event>, String> {
    let rows: Vec<Row> = c
        .query(format!("SELECT {COLUMNS} FROM calendar_events WHERE state='active' AND automation_job_id IS NOT NULL LIMIT 1000"))
        .map_err(sql("Reading events"))?;
    rows.into_iter().map(from_row).collect()
}

/// The names of the jobs these events start, for the line shown under an event.
fn job_names(c: &mut Conn, events: &[Event]) -> std::collections::HashMap<u64, String> {
    let ids: Vec<String> = events.iter().filter_map(|e| e.automation_job_id).map(|i| i.to_string()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    if ids.is_empty() {
        return Default::default();
    }
    let rows: Vec<(u64, String)> = c.query(format!("SELECT id,name FROM automation_jobs WHERE id IN ({})", ids.join(","))).unwrap_or_default();
    rows.into_iter().collect()
}

fn link_detail(e: &Event, names: &std::collections::HashMap<u64, String>) -> String {
    let Some(job) = e.automation_job_id else { return String::new() };
    let name = names.get(&job).map(String::as_str).unwrap_or("an automation that no longer exists");
    match e.automation_lead_min.unwrap_or(0) {
        0 => format!("Runs \"{name}\" at the time"),
        m => format!("Runs \"{name}\" {} before", super::notices::lead_text(m)),
    }
}

/// The single-day changes of these events: (event id, day, skip or move).
pub(super) fn exceptions(c: &mut Conn, events: &[Event]) -> Result<Vec<(u64, NaiveDate, Change)>, String> {
    let mut changes: Vec<(u64, NaiveDate, Change)> = Vec::new();
    if events.is_empty() {
        return Ok(changes);
    }
    let ids = events.iter().map(|e| e.id.to_string()).collect::<Vec<_>>().join(",");
    let rows: Vec<(u64, String, String, Option<String>)> = c
        .query(format!("SELECT event_id,DATE_FORMAT(occurrence_date,'%Y-%m-%d'),kind,DATE_FORMAT(new_starts_at,'%Y-%m-%d %H:%i') FROM calendar_exceptions WHERE event_id IN ({ids}) LIMIT 20000"))
        .map_err(sql("Reading events"))?;
    for (id, day, kind, new) in rows {
        let Ok(day) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") else { continue };
        match (kind.as_str(), new.and_then(|t| NaiveDateTime::parse_from_str(&t, FORMAT).ok())) {
            ("move", Some(t)) => changes.push((id, day, Change::Move(t))),
            ("skip", _) => changes.push((id, day, Change::Skip)),
            _ => {}
        }
    }
    Ok(changes)
}

/// Every active event that asks for a reminder.
pub(super) fn with_reminders(c: &mut Conn) -> Result<Vec<Event>, String> {
    let rows: Vec<Row> = c
        .query(format!("SELECT {COLUMNS} FROM calendar_events WHERE state='active' AND remind_before_min IS NOT NULL LIMIT 1000"))
        .map_err(sql("Reading events"))?;
    rows.into_iter().map(from_row).collect()
}

/// The event items inside a window: each occurrence, one item per day it covers.
pub fn items(c: &mut Conn, from: NaiveDate, to: NaiveDate) -> Result<Vec<Item>, String> {
    let rows: Vec<Row> = c
        .exec(
            format!("SELECT {COLUMNS} FROM calendar_events WHERE state IN ('active','proposed') AND starts_at <= ? AND (repeat_json IS NOT NULL OR COALESCE(ends_at,starts_at) >= ?) ORDER BY starts_at LIMIT 1000"),
            (format!("{to} 23:59:59"), format!("{from} 00:00:00")),
        )
        .map_err(sql("Reading events"))?;
    let events = rows.into_iter().map(from_row).collect::<Result<Vec<_>, _>>()?;
    let changes = exceptions(c, &events)?;
    let names = job_names(c, &events);
    let mut out = Vec::new();
    for e in &events {
        let Ok(s) = series_of(e) else { continue };
        let own: Vec<Exception> = changes.iter().filter(|(id, _, _)| *id == e.id).map(|(_, d, ch)| Exception { date: *d, change: ch.clone() }).collect();
        for o in repeat::occurrences(&s, &own, from, to) {
            let last = o.last_day();
            let mut day = o.start.date();
            let mut first = true;
            while day <= last {
                if day >= from && day <= to {
                    let same_day_end = o.end.filter(|t| t.date() == o.start.date() && !e.all_day);
                    out.push(Item {
                        kind: "event".into(),
                        id: e.id,
                        title: e.title.clone(),
                        date: day.to_string(),
                        time: (first && !e.all_day).then(|| hm(o.start)),
                        state: e.state.clone(),
                        owner: "calendar".into(),
                        detail: link_detail(e, &names),
                        all_day: e.all_day,
                        end_time: if first { same_day_end.map(hm) } else { None },
                        occurrence: s.repeat.is_some().then(|| o.original.to_string()),
                        repeating: s.repeat.is_some(),
                        colour: Some(e.colour.clone()),
                        first,
                    });
                }
                first = false;
                day += Duration::days(1);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> Event {
        Event { id: 0, title: "Dentist".into(), note: String::new(), starts_at: "2026-11-27 15:00".into(), ends_at: None, all_day: false, repeat: None, remind_before_min: None, colour: "accent".into(), automation_job_id: None, automation_lead_min: None, state: "active".into(), created_by: "member".into() }
    }
    #[test]
    fn a_plain_event_is_accepted_and_normalised() {
        let ok = check(Event { title: "  Dentist \n".into(), starts_at: "2026-11-27 15:00".into(), ..event() }).unwrap();
        assert_eq!(ok.event.title, "Dentist");
        assert_eq!(ok.start.format(FORMAT).to_string(), "2026-11-27 15:00");
    }
    #[test]
    fn a_date_alone_is_midnight() {
        let ok = check(Event { starts_at: "2026-11-27".into(), ..event() }).unwrap();
        assert_eq!(ok.event.starts_at, "2026-11-27 00:00");
    }
    #[test]
    fn titles_are_required_cleaned_and_bounded() {
        assert!(check(Event { title: "   ".into(), ..event() }).is_err());
        assert!(check(Event { title: "\u{0}\u{7}".into(), ..event() }).is_err());
        assert!(check(Event { title: "x".repeat(161), ..event() }).is_err());
        assert!(check(Event { title: "x".repeat(160), ..event() }).is_ok());
        assert_eq!(check(Event { title: "a\u{0}b\nc".into(), ..event() }).unwrap().event.title, "ab c");
    }
    #[test]
    fn notes_are_bounded() {
        assert!(check(Event { note: "x".repeat(4001), ..event() }).is_err());
        assert!(check(Event { note: "x".repeat(4000), ..event() }).is_ok());
    }
    #[test]
    fn times_must_be_real_and_in_range() {
        assert!(check(Event { starts_at: "tomorrow".into(), ..event() }).is_err());
        assert!(check(Event { starts_at: "2026-02-30 10:00".into(), ..event() }).is_err());
        assert!(check(Event { starts_at: "2026-11-27 25:00".into(), ..event() }).is_err());
        assert!(check(Event { starts_at: "1900-01-01 10:00".into(), ..event() }).is_err());
        assert!(check(Event { starts_at: "2300-01-01 10:00".into(), ..event() }).is_err());
    }
    #[test]
    fn an_event_cannot_end_before_it_starts_or_last_over_400_days() {
        assert!(check(Event { ends_at: Some("2026-11-27 14:00".into()), ..event() }).is_err());
        assert!(check(Event { ends_at: Some("2026-11-27 16:00".into()), ..event() }).is_ok());
        assert!(check(Event { ends_at: Some("2028-01-01 16:00".into()), ..event() }).is_err());
        assert!(check(Event { ends_at: Some("junk".into()), ..event() }).is_err());
        assert!(check(Event { ends_at: Some("".into()), ..event() }).unwrap().event.ends_at.is_none());
    }
    #[test]
    fn an_all_day_event_drops_its_times() {
        let ok = check(Event { all_day: true, starts_at: "2026-12-24 13:00".into(), ends_at: Some("2026-12-26 09:00".into()), ..event() }).unwrap();
        assert_eq!(ok.event.starts_at, "2026-12-24 00:00");
        assert_eq!(ok.event.ends_at.as_deref(), Some("2026-12-26 00:00"));
        let same = check(Event { all_day: true, starts_at: "2026-12-24 13:00".into(), ends_at: Some("2026-12-24 15:00".into()), ..event() }).unwrap();
        assert_eq!(same.event.ends_at, None, "one day needs no end");
    }
    #[test]
    fn colours_are_names_from_the_list_only() {
        assert!(check(Event { colour: "soft".into(), ..event() }).is_ok());
        assert!(check(Event { colour: "#ff0000".into(), ..event() }).is_err());
        assert!(check(Event { colour: "red; background:url(x)".into(), ..event() }).is_err());
    }
    #[test]
    fn a_bad_repeat_is_refused_here_too() {
        let bad = Repeat { every: "day".into(), interval: 0, weekdays: vec![], until: None, count: None };
        assert!(check(Event { repeat: Some(bad), ..event() }).is_err());
        let ok = Repeat { every: "week".into(), interval: 1, weekdays: vec![1, 3], until: None, count: Some(10) };
        assert!(check(Event { repeat: Some(ok), ..event() }).is_ok());
    }
    #[test]
    fn a_reminder_lead_is_bounded() {
        assert!(check(Event { remind_before_min: Some(30), ..event() }).is_ok());
        assert!(check(Event { remind_before_min: Some(14 * 1440 + 1), ..event() }).is_err());
    }
    /// Opt-in isolated database: the generated test database and user are created and dropped here.
    /// `CASTLE_TEST_DB=port:admin:password`, or the installed database's port and stored root key.
    #[test]
    #[ignore]
    fn live_database_saves_repeats_skips_moves_and_deletes() {
        let config = std::env::var("CASTLE_TEST_DB").unwrap_or_else(|_| {
            let folder = std::env::var("APPDATA").unwrap() + "/com.memorycore.castle/settings.json";
            let port = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(folder).unwrap()).unwrap()["dbPort"].as_u64().unwrap();
            format!("{port}:root:{}", crate::key::db_root().unwrap_or_default())
        });
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let (user, pass) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &user, &pass, &format!("Calendar Test {}", std::process::id())).unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        let seen: Option<String> = c.query_first("SELECT name FROM schema_migrations WHERE name='021_calendar'").unwrap();
        assert!(seen.is_some(), "the calendar migration is recorded");
        let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let t = |s: &str| NaiveDateTime::parse_from_str(s, FORMAT).unwrap();

        // a weekly event, every Monday at 09:00 for 1h30
        let weekly = Event { title: "Standup".into(), starts_at: "2026-11-02 09:00".into(), ends_at: Some("2026-11-02 10:30".into()), repeat: Some(Repeat { every: "week".into(), interval: 1, weekdays: vec![], until: None, count: None }), ..event() };
        let id = save(&mut c, &check(weekly).unwrap()).unwrap();
        assert!(id > 0);
        let mondays = |c: &mut Conn| items(c, d("2026-11-01"), d("2026-11-30")).unwrap().iter().filter(|i| i.id == id).map(|i| i.date.clone()).collect::<Vec<_>>();
        assert_eq!(mondays(&mut c), vec!["2026-11-02", "2026-11-09", "2026-11-16", "2026-11-23", "2026-11-30"]);
        let first = &items(&mut c, d("2026-11-02"), d("2026-11-02")).unwrap()[0];
        assert_eq!((first.time.as_deref(), first.end_time.as_deref(), first.repeating), (Some("09:00"), Some("10:30"), true));
        assert_eq!(first.occurrence.as_deref(), Some("2026-11-02"));

        // skip one Monday, move another, and refuse a day that is not in the series
        change_day(&mut c, id, d("2026-11-09"), None).unwrap();
        change_day(&mut c, id, d("2026-11-16"), Some(t("2026-11-18 14:00"))).unwrap();
        assert!(change_day(&mut c, id, d("2026-11-10"), None).is_err());
        assert_eq!(mondays(&mut c), vec!["2026-11-02", "2026-11-18", "2026-11-23", "2026-11-30"]);
        let moved = items(&mut c, d("2026-11-18"), d("2026-11-18")).unwrap();
        assert_eq!((moved[0].time.as_deref(), moved[0].end_time.as_deref(), moved[0].occurrence.as_deref()), (Some("14:00"), Some("15:30"), Some("2026-11-16")));
        // skipping the same day twice or moving it again just changes the one row
        change_day(&mut c, id, d("2026-11-16"), None).unwrap();
        assert_eq!(mondays(&mut c), vec!["2026-11-02", "2026-11-23", "2026-11-30"]);
        restore_day(&mut c, id, d("2026-11-09")).unwrap();
        assert_eq!(mondays(&mut c), vec!["2026-11-02", "2026-11-09", "2026-11-23", "2026-11-30"]);

        // a whole-series edit that keeps the first day and the rule keeps the single-day changes
        let mut again = get(&mut c, id).unwrap();
        again.title = "Daily standup".into();
        save(&mut c, &check(again.clone()).unwrap()).unwrap();
        assert_eq!(mondays(&mut c).len(), 4);
        // ...but moving the first day forgets them, since they no longer line up
        again.starts_at = "2026-11-03 09:00".into();
        again.ends_at = Some("2026-11-03 10:30".into());
        save(&mut c, &check(again).unwrap()).unwrap();
        let left: Option<u64> = c.exec_first("SELECT COUNT(*) FROM calendar_exceptions WHERE event_id=?", (id,)).unwrap();
        assert_eq!(left, Some(0));
        assert_eq!(mondays(&mut c).len(), 4, "Tuesdays 3, 10, 17, 24 now");

        // a one-off, a multi-day one and an all-day one
        let once = save(&mut c, &check(Event { title: "Dentist".into(), starts_at: "2026-11-27 15:00".into(), ..event() }).unwrap()).unwrap();
        let trip = save(&mut c, &check(Event { title: "Trip".into(), all_day: true, starts_at: "2026-11-28".into(), ends_at: Some("2026-11-30".into()), ..event() }).unwrap()).unwrap();
        let all = items(&mut c, d("2026-11-01"), d("2026-11-30")).unwrap();
        assert_eq!(all.iter().filter(|i| i.id == once).count(), 1);
        let days: Vec<&str> = all.iter().filter(|i| i.id == trip).map(|i| i.date.as_str()).collect();
        assert_eq!(days, vec!["2026-11-28", "2026-11-29", "2026-11-30"]);
        assert!(all.iter().filter(|i| i.id == trip).all(|i| i.all_day && i.time.is_none()));
        assert!(items(&mut c, d("2026-12-05"), d("2026-12-06")).unwrap().iter().all(|i| i.id != once && i.id != trip), "outside the window nothing shows");

        // moving a one-off keeps its length; a repeating one cannot be moved whole
        move_whole(&mut c, once, t("2026-11-30 10:00")).unwrap();
        assert_eq!(get(&mut c, once).unwrap().starts_at, "2026-11-30 10:00");
        assert!(move_whole(&mut c, id, t("2026-11-30 10:00")).is_err());
        assert!(change_day(&mut c, once, d("2026-11-30"), None).is_err(), "a one-off has no single days");

        // deleting removes the event and its changed days; a second delete says so
        change_day(&mut c, id, d("2026-11-10"), None).unwrap();
        delete(&mut c, id).unwrap();
        let gone: Option<u64> = c.query_first("SELECT COUNT(*) FROM calendar_exceptions").unwrap();
        assert_eq!(gone, Some(0));
        assert!(delete(&mut c, id).is_err());
        assert!(get(&mut c, id).is_err());

        c.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        let mut root = Conn::new(mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(user.as_str())).pass(Some(pass.as_str()))).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user)).unwrap();
        }
    }

    #[test]
    fn an_unknown_field_from_the_page_is_refused() {
        let text = r#"{"title":"x","startsAt":"2026-11-27 15:00","state":"proposed"}"#;
        assert!(serde_json::from_str::<Event>(text).is_err(), "the page cannot set state or created_by");
        let ok = r#"{"title":"x","startsAt":"2026-11-27 15:00"}"#;
        let e: Event = serde_json::from_str(ok).unwrap();
        assert_eq!(e.colour, "accent");
    }
}
