//! CALENDAR phase 5: an event can start one automation job, at its time or some minutes before.
//!
//! Only the calendar can start a job this way, and only the one job the event names: the job's trigger
//! must be "Calendar event reaches its time", and it still goes through the same single launch path as
//! every other trigger (switched on, not paused, unchanged, within the concurrency and hourly limits).
//! Each occurrence is claimed in `calendar_triggers` before anything starts, so a restart or a second
//! tick never starts it twice; one that is more than a few minutes late (Castle was closed or paused)
//! is recorded as skipped and never replayed: a briefing for a meeting that has begun is no use.
use super::notices::{self, line, Due};
use super::store::{self, Event};
use chrono::NaiveDateTime;
use mysql::{prelude::Queryable, Conn};
use serde::Serialize;
use tauri::AppHandle;

/// How late a start may be and still run; a tick is 15 seconds, so this covers a short pause or a slow tick.
pub const RUN_GRACE_SEC: i64 = 300;
/// The trigger a job must have to be linked.
pub const TRIGGER: &str = "calendar_event";

/// What to do with a due start: run it, or record that it was too late.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    Run,
    TooLate,
}

pub fn verdict(due: &Due, now: NaiveDateTime) -> Verdict {
    if (now - due.due_at).num_seconds() > RUN_GRACE_SEC { Verdict::TooLate } else { Verdict::Run }
}

#[derive(Debug)]
pub struct Claimed {
    pub row: u64,
    pub job: u64,
    pub due: Due,
}

fn claim(c: &mut Conn, job: u64, d: &Due, status: &str, note: &str) -> Result<Option<u64>, String> {
    c.exec_drop(
        "INSERT IGNORE INTO calendar_triggers (event_id,occurrence_start,job_id,due_at,status,note) VALUES (?,?,?,?,?,?)",
        (d.event_id, d.start.format("%Y-%m-%d %H:%M:00").to_string(), job, d.due_at.format("%Y-%m-%d %H:%M:%S").to_string(), status, note),
    )
    .map_err(|_| "An automation start could not be recorded.".to_string())?;
    Ok((c.affected_rows() == 1).then(|| c.last_insert_id()))
}

/// Find what is due, claim it, and return the starts to make now. Late ones are recorded and dropped.
pub fn collect(c: &mut Conn, now: NaiveDateTime) -> Result<Vec<Claimed>, String> {
    let events: Vec<Event> = store::with_automation(c)?;
    let jobs: std::collections::HashMap<u64, u64> = events.iter().filter_map(|e| e.automation_job_id.map(|j| (e.id, j))).collect();
    let due = notices::pick(&notices::candidates(c, &events, now, |e| e.automation_job_id.map(|_| e.automation_lead_min.unwrap_or(0)))?, now);
    let mut out = Vec::new();
    for d in due {
        let Some(job) = jobs.get(&d.event_id).copied() else { continue };
        match verdict(&d, now) {
            Verdict::Run => {
                if let Some(row) = claim(c, job, &d, "claimed", "")? {
                    out.push(Claimed { row, job, due: d });
                }
            }
            Verdict::TooLate => {
                claim(c, job, &d, "skipped", "Castle was not running at the time, so it was not started late.")?;
            }
        }
    }
    Ok(out)
}

fn record(c: &mut Conn, row: u64, status: &str, run: Option<u64>, note: &str) {
    let note: String = note.chars().filter(|ch| !ch.is_control()).take(400).collect();
    let _ = c.exec_drop("UPDATE calendar_triggers SET status=?,run_id=?,note=? WHERE id=?", (status, run, note, row));
}

/// After the member saves or moves an event: a start whose time has already passed is not made now.
pub fn settle(c: &mut Conn, event_id: u64, now: NaiveDateTime) -> Result<(), String> {
    let events: Vec<Event> = store::with_automation(c)?.into_iter().filter(|e| e.id == event_id).collect();
    let Some(job) = events.first().and_then(|e| e.automation_job_id) else { return Ok(()) };
    for d in notices::pick(&notices::candidates(c, &events, now, |e| e.automation_job_id.map(|_| e.automation_lead_min.unwrap_or(0)))?, now) {
        claim(c, job, &d, "skipped", "Already past when the event was saved.")?;
    }
    Ok(())
}

/// What a job is told: the event, in plain words. Data, like every payload.
pub fn payload_text(d: &Due) -> String {
    let mut on_time = d.clone();
    on_time.late = false;
    line(&on_time)
}

/// One tick: start what is due. Never fails the caller; a refusal is recorded against the occurrence.
pub fn run(app: &AppHandle, c: &mut Conn, now: NaiveDateTime) {
    let Ok(found) = collect(c, now) else { return };
    for f in found {
        match crate::automations::events::fire_linked(app, f.job, &payload_text(&f.due)) {
            Ok(run) => record(c, f.row, "started", Some(run), ""),
            Err(e) => record(c, f.row, "refused", None, &e),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobChoice {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
}

/// The jobs an event may start: approved, and waiting for the calendar event.
pub fn eligible(c: &mut Conn) -> Result<Vec<JobChoice>, String> {
    let rows: Vec<(u64, String, bool)> = c
        .exec(
            "SELECT id,name,enabled FROM automation_jobs WHERE state='active' AND trigger_kind='event' AND JSON_UNQUOTE(JSON_EXTRACT(trigger_json,'$.event'))=? ORDER BY name LIMIT 100",
            (TRIGGER,),
        )
        .map_err(|_| "Automations could not be read.".to_string())?;
    Ok(rows.into_iter().map(|(id, name, enabled)| JobChoice { id, name, enabled }).collect())
}

/// A job may be linked only if it is one of those.
pub fn check_link(c: &mut Conn, event: &Event) -> Result<(), String> {
    let Some(job) = event.automation_job_id else { return Ok(()) };
    if eligible(c)?.iter().any(|j| j.id == job) {
        Ok(())
    } else {
        Err("Choose an automation that waits for a calendar event (make one in AUTOMATIONS first).".into())
    }
}

/// Free the events that started a job that has been archived or deleted.
pub fn unlink_job(c: &mut Conn, job: u64) {
    let _ = c.exec_drop("UPDATE calendar_events SET automation_job_id=NULL,automation_lead_min=NULL WHERE automation_job_id=?", (job,));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }
    fn due(start: &str, lead: u32, now: &str) -> Due {
        let c = notices::Candidate { event_id: 1, title: "Standup".into(), lead, start: t(start), all_day: false };
        notices::pick(&[c], t(now)).remove(0)
    }

    #[test]
    fn a_start_a_little_late_runs_and_a_long_late_one_is_skipped() {
        let on = due("2026-11-27 09:00:00", 30, "2026-11-27 08:30:10");
        assert_eq!(verdict(&on, t("2026-11-27 08:30:10")), Verdict::Run);
        let slow = due("2026-11-27 09:00:00", 30, "2026-11-27 08:35:00");
        assert_eq!(verdict(&slow, t("2026-11-27 08:35:00")), Verdict::Run, "exactly five minutes late still runs");
        let late = due("2026-11-27 09:00:00", 30, "2026-11-27 08:35:01");
        assert_eq!(verdict(&late, t("2026-11-27 08:35:01")), Verdict::TooLate);
        let away = due("2026-11-27 09:00:00", 30, "2026-11-27 11:00:00");
        assert_eq!(verdict(&away, t("2026-11-27 11:00:00")), Verdict::TooLate);
    }

    #[test]
    fn the_job_is_told_the_event_in_plain_words_never_as_missed() {
        let late = due("2026-11-27 15:00:00", 30, "2026-11-27 14:34:00");
        assert!(late.late, "it is late for a reminder, but the job text does not say so");
        assert_eq!(payload_text(&late), "Standup - in 30 minutes (15:00)");
        let now = due("2026-11-27 15:00:00", 0, "2026-11-27 15:00:04");
        assert_eq!(payload_text(&now), "Standup - now (15:00)");
    }

    #[test]
    fn the_trigger_name_is_the_one_the_automations_list_has() {
        assert!(crate::automations::events::EVENTS.iter().any(|(n, _)| *n == TRIGGER));
    }
}
