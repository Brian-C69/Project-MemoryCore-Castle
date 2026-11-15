//! Reading the MemoryCore Foundation room from the moment Castle starts - not only while the
//! CHAT tab is open - so every speaker's lines are there when the member looks, and the
//! companion can answer while the member is away. A held read (20 s) while CHAT is shown, a quick
//! look every 30 s while it is hidden or in the tray. New lines go to the page as `room-lines`; a
//! read that fails is said plainly (`room-read-failed`), never swallowed.

use crate::chatroom::{Room, RoomRefusal};
use crate::key;
use crate::settings::SettingsStore;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const KEEP: usize = 500;
/// How many ids before the newest page a fresh start looks back. A read with no starting point
/// gets only the newest 50 lines, so CHAT began each Castle start with a short room.
const BACKFILL: u64 = 300;
/// At most this many extra pages (200 lines each) are read to fill the room back in.
const BACKFILL_PAGES: usize = 3;

#[derive(Default)]
pub struct RoomReader {
    visible: AtomicBool,
    lines: Mutex<Vec<Value>>,
    cursor: Mutex<Option<u64>>,
    /// the room's time cursors for what was cleared / reacted to since (sent back as they came)
    cleared_at: Mutex<Option<String>>,
    reacted_at: Mutex<Option<String>>,
    problem: Mutex<Option<RoomRefusal>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backlog {
    pub lines: Vec<Value>,
    pub problem: Option<RoomRefusal>,
}

#[derive(Clone, Serialize)]
struct Fresh {
    lines: Vec<Value>,
    cleared: Vec<u64>,
    /// lines already shown whose reactions changed, whole (the page replaces them by id)
    updated: Vec<Value>,
}

fn id_of(line: &Value) -> Option<u64> {
    line.get("id").and_then(Value::as_u64)
}

/// Where to read from to fill the room back in: BACKFILL ids before the oldest line the first
/// read gave. None when that line is already the first in the room. Pure, tested.
pub fn backfill_start(oldest: u64) -> Option<u64> {
    (oldest > 1).then(|| oldest.saturating_sub(BACKFILL))
}

/// Lines kept so far + one read's answer -> (all lines kept, the ones that are new). Pure, tested.
/// The reactions that changed since the last read, put on the lines still kept: the changed lines,
/// whole, before and after (a line not kept is left out). Pure, tested.
pub fn apply_reacted(lines: &mut [Value], answer: &Value) -> Vec<(Value, Value)> {
    let mut changed = Vec::new();
    for update in answer.get("reacted").and_then(Value::as_array).cloned().unwrap_or_default() {
        let (Some(id), Some(reactions)) = (id_of(&update), update.get("reactions")) else { continue };
        if let Some(line) = lines.iter_mut().find(|l| id_of(l) == Some(id)) {
            let before = line.clone();
            line["reactions"] = reactions.clone();
            changed.push((before, line.clone()));
        }
    }
    changed
}

pub fn merge(kept: &[Value], answer: &Value) -> (Vec<Value>, Vec<Value>) {
    let cleared: Vec<u64> = answer.get("cleared").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
    let mut all: Vec<Value> = kept.iter().filter(|l| id_of(l).is_none_or(|id| !cleared.contains(&id))).cloned().collect();
    let mut fresh = Vec::new();
    for line in answer.get("lines").and_then(Value::as_array).cloned().unwrap_or_default() {
        if id_of(&line).is_some_and(|id| !all.iter().any(|l| id_of(l) == Some(id))) {
            all.push(line.clone());
            fresh.push(line);
        }
    }
    all.sort_by_key(|l| id_of(l).unwrap_or(0));
    let keep = all.len().saturating_sub(KEEP);
    (all[keep..].to_vec(), fresh)
}

/// After the first read of a start (the newest 50 lines), reads the older lines the server still
/// has, by asking for the lines after an earlier id a page at a time, and merges them in. The page
/// gets them as room-lines, which it keeps once each and in order. A failure just leaves the room
/// as the first read made it.
fn backfill(app: &AppHandle, room: &Room, reader: &RoomReader) {
    let Some(oldest) = reader.lines.lock().unwrap().first().and_then(id_of) else { return };
    let Some(mut after) = backfill_start(oldest) else { return };
    for _ in 0..BACKFILL_PAGES {
        let Ok(answer) = room.lines(Some(after), 0) else { return };
        let kept = reader.lines.lock().unwrap().clone();
        let (all, fresh) = merge(&kept, &answer);
        *reader.lines.lock().unwrap() = all;
        if !fresh.is_empty() {
            let _ = app.emit("room-lines", Fresh { lines: fresh, cleared: Vec::new(), updated: Vec::new() });
        }
        let last = answer.get("lines").and_then(Value::as_array).and_then(|l| l.last()).and_then(id_of);
        let more = answer.get("more").and_then(Value::as_bool).unwrap_or(false);
        match last {
            Some(last) if more && last < oldest => after = last,
            _ => return,
        }
    }
}

/// Starts the reader with Castle.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        let settings = app.state::<SettingsStore>().get();
        let token = key::load("member");
        let (true, Some(token)) = (settings.room_on("chat"), token) else {
            std::thread::sleep(Duration::from_secs(15));
            continue;
        };
        let reader = app.state::<RoomReader>();
        let visible = reader.visible.load(Ordering::Relaxed);
        let cursor = *reader.cursor.lock().unwrap();
        let cleared_at = reader.cleared_at.lock().unwrap().clone();
        let reacted_at = reader.reacted_at.lock().unwrap().clone();
        let room = Room::new(&crate::signin::portfolio(), &token);
        match room.lines_since(cursor, if visible && cursor.is_some() { 20 } else { 0 }, cleared_at.as_deref(), reacted_at.as_deref()) {
            Ok(answer) => {
                let kept = reader.lines.lock().unwrap().clone();
                let (mut all, fresh) = merge(&kept, &answer);
                let reacted = apply_reacted(&mut all, &answer);
                *reader.lines.lock().unwrap() = all;
                if let Some(next) = answer.get("cursor").and_then(Value::as_u64) {
                    *reader.cursor.lock().unwrap() = Some(next);
                }
                for (key, slot) in [("clearedAt", &reader.cleared_at), ("reactedAt", &reader.reacted_at)] {
                    if let Some(at) = answer.get(key).and_then(Value::as_str).filter(|a| !a.is_empty()) {
                        *slot.lock().unwrap() = Some(at.to_string());
                    }
                }
                if cursor.is_none() {
                    backfill(&app, &room, &reader);
                }
                *reader.problem.lock().unwrap() = None;
                let cleared: Vec<u64> = answer.get("cleared").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
                let updated: Vec<Value> = reacted.iter().map(|(_, after)| after.clone()).collect();
                if !fresh.is_empty() || !cleared.is_empty() || !updated.is_empty() {
                    let _ = app.emit("room-lines", Fresh { lines: fresh.clone(), cleared, updated });
                }
                if !fresh.is_empty() && cursor.is_some() {
                    crate::roomturn::heard(&app, &fresh);
                    // read again: a held read can last 20 s, and the member may have left CHAT meanwhile
                    crate::roomalert::new_lines(&app, &fresh, reader.visible.load(Ordering::Relaxed));
                }
                let more = answer.get("more").and_then(Value::as_bool).unwrap_or(false);
                if !more && !reader.visible.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(30));
                }
            }
            Err(refused) => {
                let pause = if refused.kind == "locked" { 60 } else { 10 };
                *reader.problem.lock().unwrap() = Some(refused.clone());
                let _ = app.emit("room-read-failed", refused);
                std::thread::sleep(Duration::from_secs(pause));
            }
        }
    });
}

/// The page says whether CHAT is on screen (held reads only then).
#[tauri::command]
pub fn room_visible(on: bool, reader: State<RoomReader>) {
    reader.visible.store(on, Ordering::Relaxed);
}

/// Everything read so far, for a CHAT tab that opens later.
#[tauri::command]
pub fn room_backlog(reader: State<RoomReader>) -> Backlog {
    Backlog { lines: reader.lines.lock().unwrap().clone(), problem: reader.problem.lock().unwrap().clone() }
}

/// After a sign-in or sign-out the reader starts over.
#[tauri::command]
pub fn room_reset(reader: State<RoomReader>) {
    reader.lines.lock().unwrap().clear();
    *reader.cursor.lock().unwrap() = None;
    *reader.cleared_at.lock().unwrap() = None;
    *reader.reacted_at.lock().unwrap() = None;
    *reader.problem.lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_fresh_start_looks_back_from_before_the_oldest_line_it_got() {
        assert_eq!(backfill_start(308), Some(8));
        assert_eq!(backfill_start(150), Some(0));
        assert_eq!(backfill_start(2), Some(0));
        assert_eq!(backfill_start(1), None, "the first line of the room has nothing before it");
        assert_eq!(backfill_start(0), None);
    }

    #[test]
    fn older_lines_merge_in_before_the_newest_without_being_counted_twice() {
        let newest = vec![json!({"id": 308, "text": "n"}), json!({"id": 309, "text": "m"})];
        let older = json!({"lines": [{"id": 8, "text": "a"}, {"id": 308, "text": "n"}], "more": false});
        let (all, fresh) = merge(&newest, &older);
        assert_eq!(all.iter().map(|l| l["id"].as_u64().unwrap()).collect::<Vec<_>>(), vec![8, 308, 309]);
        assert_eq!(fresh.len(), 1);
    }

    #[test]
    fn every_speakers_lines_are_kept_once_in_order_and_cleared_ones_go() {
        let kept = vec![json!({"id": 1, "text": "hi", "author": {"name": "Kiyo", "kind": "host"}}), json!({"id": 2, "text": "x"})];
        let answer = json!({"cursor": 4, "lines": [
            {"id": 4, "text": "hello", "author": {"name": "Rin's Mika", "kind": "member_assistant"}},
            {"id": 3, "text": "hey", "author": {"name": "Rin", "kind": "member"}},
            {"id": 1, "text": "hi again"}
        ], "cleared": [2]});
        let (all, fresh) = merge(&kept, &answer);
        assert_eq!(all.iter().map(|l| l["id"].as_u64().unwrap()).collect::<Vec<_>>(), vec![1, 3, 4]);
        assert_eq!(fresh.len(), 2, "a line already kept is not new again");
        assert_eq!(all[0]["text"], "hi");
    }

    #[test]
    fn a_reaction_changes_a_kept_line_in_place_without_counting_it_again() {
        let mut lines = vec![json!({"id": 3, "text": "hi"}), json!({"id": 4, "text": "yo"})];
        let answer = json!({"lines": [], "reacted": [{"id": 3, "reactions": [{"emoji": "👍", "count": 2, "mine": true}]}, {"id": 99, "reactions": []}]});
        let changed = apply_reacted(&mut lines, &answer);
        assert_eq!(changed.len(), 1, "a line not kept is left out");
        assert!(changed[0].0.get("reactions").is_none());
        assert_eq!(lines[0]["reactions"][0]["count"], 2);
        assert_eq!(lines.len(), 2);
        assert!(apply_reacted(&mut lines, &json!({"lines": []})).is_empty(), "a room without reactions changes nothing");
    }
}
