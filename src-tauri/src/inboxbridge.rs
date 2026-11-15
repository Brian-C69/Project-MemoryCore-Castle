//! The Mira Inbox bridge, for self-hosters who run the private Mira Inbox web app on their own
//! computer: Castle copies the chat room (the lines CHAT shows) to it, so the room can be read from
//! a phone, and posts what the member wrote there to the room as them.
//! Off until the member switches it on in Settings > Companion. Castle only ever calls out, to the
//! address saved in Settings (this computer's own by default), with a long token kept in Windows
//! Credential Manager; the Inbox's bridge door answers that computer alone.
//! The copy uses the lines the room reader already keeps, which it reads from the moment Castle
//! starts, so it works while Castle is hidden in the tray.

use crate::settings::SettingsStore;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// Where the Inbox lives when the member has not said otherwise: this computer's own web server.
pub const DEFAULT_URL: &str = "http://localhost/mira-inbox";
/// The key's name in Credential Manager (key.rs).
const ACCOUNT: &str = "inbox-bridge";
const TICK: Duration = Duration::from_secs(3);

/// What Settings shows: whether the Inbox answered lately, and why not.
#[derive(Default)]
pub struct BridgeState {
    connected: Mutex<bool>,
    problem: Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    on: bool,
    connected: bool,
    has_token: bool,
    problem: Option<String>,
}

/// The door's address for one action. Pure, tested.
pub fn bridge_url(base: &str, action: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let base = if base.is_empty() { DEFAULT_URL } else { base };
    format!("{base}/bridge.php?a={action}")
}

/// One room line as the Inbox stores it: who said it, what kind of speaker, whether it is the
/// member's own, and the words (a picture or a voice line is told in words). Pure, tested.
/// `mine` is the ids of lines the member posted from Castle, `my_name` their name in the room.
pub fn pushed(line: &Value, mine: &[u64], my_name: Option<&str>) -> Option<Value> {
    let id = line.get("id").and_then(Value::as_u64)?;
    let kind = line.pointer("/author/kind").and_then(Value::as_str).unwrap_or("");
    let name = line.pointer("/author/name").and_then(Value::as_str).map(str::trim).unwrap_or("");
    let own = mine.contains(&id) || (kind == "member" && !name.is_empty() && my_name.is_some_and(|m| m == name));
    let role = if kind == "system" {
        "system"
    } else if own {
        "me"
    } else if kind == "assistant" || kind == "member_assistant" {
        "ai"
    } else {
        "other"
    };
    let mut text = line.get("text").and_then(Value::as_str).unwrap_or("").trim().to_string();
    for (field, told) in [("picture", "[a picture]"), ("voice", "[a voice message]")] {
        if line.get(field).is_some_and(|v| !v.is_null()) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(told);
        }
    }
    if text.is_empty() {
        return None;
    }
    Some(json!({ "key": format!("room-{id}"), "role": role, "author": name, "kind": kind, "mine": own, "text": text, "at": line.get("at") }))
}

struct Door {
    client: reqwest::blocking::Client,
    base: String,
    token: String,
}

impl Door {
    fn new(base: &str, token: &str) -> Result<Door, String> {
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(8)).build().map_err(|_| "The Inbox call could not start.".to_string())?;
        Ok(Door { client, base: base.to_string(), token: token.to_string() })
    }

    fn call(&self, action: &str, body: Option<&Value>) -> Result<Value, String> {
        let url = bridge_url(&self.base, action);
        let request = match body {
            Some(body) => self.client.post(url).json(body),
            None => self.client.get(url),
        };
        let response = request.bearer_auth(&self.token).send().map_err(|_| "The Inbox did not answer. Is it running?".to_string())?;
        match response.status().as_u16() {
            200..=299 => response.json().map_err(|_| "The Inbox sent something unreadable.".to_string()),
            404 => Err("The Inbox refused the token or the address. Check both.".to_string()),
            _ => Err("The Inbox could not take that.".to_string()),
        }
    }
}

/// Copies the room lines the Inbox has not got yet (those newer than `sent`, oldest first).
fn mirror(app: &AppHandle, door: &Door, sent: &mut u64) -> Result<(), String> {
    let mut lines = app.state::<crate::roomread::RoomReader>().snapshot();
    lines.sort_by_key(|l| l.get("id").and_then(Value::as_u64).unwrap_or(0));
    let store = app.state::<SettingsStore>();
    let mine = crate::chatroom::my_line_ids(&store);
    let name = crate::roomalert::room_name();
    let fresh: Vec<&Value> = lines.iter().filter(|l| l.get("id").and_then(Value::as_u64).is_some_and(|id| id > *sent)).collect();
    let Some(newest) = fresh.last().and_then(|l| l.get("id").and_then(Value::as_u64)) else { return Ok(()) };
    let payload: Vec<Value> = fresh.iter().filter_map(|l| pushed(l, &mine, name.as_deref())).collect();
    for chunk in payload.chunks(100) {
        door.call("push", Some(&json!({ "entries": chunk })))?;
    }
    *sent = newest;
    Ok(())
}

/// Posts what the phone wrote to the room, as the member, oldest first, and tells the Inbox which
/// went out. One the room cannot take right now stays waiting for the next round; one that is too
/// long for the room is dropped (the phone page keeps messages within the limit).
fn collect(app: &AppHandle, door: &Door) -> Result<(), String> {
    let waiting = door.call("outbox", None)?;
    let store = app.state::<SettingsStore>();
    let mut done: Vec<u64> = Vec::new();
    for item in waiting.get("items").and_then(Value::as_array).into_iter().flatten() {
        let (Some(id), Some(body)) = (item.get("id").and_then(Value::as_u64), item.get("body").and_then(Value::as_str)) else { continue };
        match crate::chatroom::post_as_member(&store, body) {
            Ok(_) => done.push(id),
            Err(refused) if refused.kind == "tooLarge" => done.push(id),
            Err(_) => break,
        }
    }
    if !done.is_empty() {
        door.call("ack", Some(&json!({ "ids": done })))?;
    }
    Ok(())
}

/// Starts the bridge with Castle; it does nothing while the switch in Settings is off.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut sent: u64 = 0;
        loop {
            std::thread::sleep(TICK);
            let state = app.state::<BridgeState>();
            let settings = app.state::<SettingsStore>().get();
            let token = crate::key::load(ACCOUNT);
            let (true, Some(token)) = (settings.inbox_bridge_on, token) else {
                sent = 0; // switched on again: copy the kept lines afresh (the Inbox ignores repeats)
                *state.connected.lock().unwrap() = false;
                *state.problem.lock().unwrap() = None;
                continue;
            };
            let outcome = Door::new(settings.inbox_bridge_url.as_deref().unwrap_or(""), &token).and_then(|door| {
                mirror(&app, &door, &mut sent)?;
                collect(&app, &door)
            });
            *state.connected.lock().unwrap() = outcome.is_ok();
            *state.problem.lock().unwrap() = outcome.err();
        }
    });
}

#[tauri::command]
pub fn inbox_bridge_status(store: State<SettingsStore>, state: State<BridgeState>) -> BridgeStatus {
    BridgeStatus {
        on: store.get().inbox_bridge_on,
        connected: *state.connected.lock().unwrap(),
        has_token: crate::key::load(ACCOUNT).is_some(),
        problem: state.problem.lock().unwrap().clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: u64, name: &str, kind: &str, text: &str) -> Value {
        json!({ "id": id, "text": text, "at": "2026-11-15T13:30:00Z", "author": { "name": name, "kind": kind } })
    }

    #[test]
    fn the_door_address_follows_the_setting_or_this_computers_own() {
        assert_eq!(bridge_url("", "push"), "http://localhost/mira-inbox/bridge.php?a=push");
        assert_eq!(bridge_url("  https://example.test/inbox/ ", "outbox"), "https://example.test/inbox/bridge.php?a=outbox");
    }

    #[test]
    fn a_line_says_who_spoke_and_whether_it_is_the_members_own() {
        let alice = pushed(&line(7, "Alice", "assistant", " welcome "), &[], Some("Bernard")).unwrap();
        assert_eq!((alice["role"].as_str(), alice["author"].as_str(), alice["mine"].as_bool()), (Some("ai"), Some("Alice"), Some(false)));
        assert_eq!(alice["key"], "room-7");
        assert_eq!(alice["text"], "welcome");
        let other = pushed(&line(8, "Kiyo", "host", "hi"), &[], Some("Bernard")).unwrap();
        assert_eq!(other["role"], "other");
        // by id (posted from Castle or the phone) and by name in the room
        assert_eq!(pushed(&line(9, "Bernard", "member", "yo"), &[9], None).unwrap()["role"], "me");
        assert_eq!(pushed(&line(10, "Bernard", "member", "yo"), &[], Some("Bernard")).unwrap()["mine"], true);
        assert_eq!(pushed(&line(11, "Mira", "member_assistant", "hm"), &[], Some("Bernard")).unwrap()["role"], "ai");
        assert_eq!(pushed(&line(12, "Room", "system", "someone joined"), &[], None).unwrap()["role"], "system");
    }

    #[test]
    fn a_picture_or_a_voice_line_is_told_in_words_and_an_empty_line_is_left_out() {
        let mut shown = line(3, "Alice", "assistant", "look");
        shown["picture"] = json!({ "fileId": "abc" });
        assert_eq!(pushed(&shown, &[], None).unwrap()["text"], "look\n[a picture]");
        let mut said = line(4, "Alice", "assistant", "");
        said["voice"] = json!({ "fileId": "def" });
        assert_eq!(pushed(&said, &[], None).unwrap()["text"], "[a voice message]");
        assert!(pushed(&line(5, "Alice", "assistant", "  "), &[], None).is_none());
        assert!(pushed(&json!({ "text": "no id" }), &[], None).is_none());
    }
}
