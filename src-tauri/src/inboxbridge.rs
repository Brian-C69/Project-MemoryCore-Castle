//! The Mira Inbox bridge, for self-hosters who run the private Mira Inbox web app on their own
//! computer: Castle copies the main chat (the member's words and the companion's replies) to it, so
//! it can be read from a phone, and types what the phone sent into the companion's session.
//! Off until the member switches it on in Settings > Companion. Castle only ever calls out, to the
//! address saved in Settings (this computer's own by default), with a long token kept in Windows
//! Credential Manager; the Inbox's bridge door answers that computer alone.
//! The mirror reads the session's record with its own reader, so it works while Castle is hidden
//! in the tray and never takes lines from the Chat on screen.

use crate::chat::newest_record;
use crate::session::{MainHost, Room, STORY_FOLDER};
use crate::settings::SettingsStore;
use crate::transcript::{ChatEntry, Follower, FIRST_READ_LIMIT};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// Where the Inbox lives when the member has not said otherwise: this computer's own web server.
pub const DEFAULT_URL: &str = "http://localhost/mira-inbox";
/// The key's name in Credential Manager (key.rs).
const ACCOUNT: &str = "inbox-bridge";
const TICK: Duration = Duration::from_secs(3);
/// Lines waiting to be sent while the Inbox is out of reach; older ones are dropped.
const BACKLOG: usize = 600;

/// What Settings shows: whether the bridge is on, whether the Inbox answered lately, and why not.
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

/// One chat entry as the Inbox stores it: only the member's words and the companion's replies (not
/// tool work, plans, pictures or voices), with a key that stays the same for the same line. Pure, tested.
pub fn pushed(entry: &ChatEntry, provider: &str, session: u64) -> Option<Value> {
    let role = match entry.role.as_str() {
        "member" => "me",
        "companion" => "companion",
        _ => return None,
    };
    if crate::providers::castle_wrote(entry) {
        return None;
    }
    let mut text = entry.text.trim().to_string();
    if !entry.pictures.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(if entry.pictures.len() == 1 { "[a picture was shared]" } else { "[pictures were shared]" });
    }
    if text.is_empty() {
        return None;
    }
    let digest = Sha256::digest(format!("{provider}-{session}-{}", entry.id).as_bytes());
    let key: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    Some(json!({ "key": key, "role": role, "text": text, "at": entry.at }))
}

/// A message from the phone, as typed into the session: it says where it came from.
pub fn phone_text(text: &str) -> String {
    format!("[Sent from my phone] {}", text.trim())
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

/// The reader of the main session's record, and the lines the Inbox has not taken yet.
#[derive(Default)]
struct Mirror {
    at: Option<(u64, PathBuf)>,
    follower: Option<Follower>,
    backlog: Vec<Value>,
}

impl Mirror {
    fn read(&mut self, app: &AppHandle) {
        let host = &app.state::<MainHost>().0;
        let Some((session, started)) = host.started() else { return };
        let settings = app.state::<SettingsStore>().get();
        let provider = host.provider().unwrap_or(settings.cli);
        let Some(folder) = Room::Main.folder(&settings) else { return };
        let Some((newest, follow)) = newest_record(provider, &folder, started) else { return };
        let current = Some((session, newest.clone()));
        let fresh = self.at != current;
        if fresh {
            self.follower = Some(follow(newest, started).skipping(&folder.join(STORY_FOLDER)));
            self.at = current;
        }
        let Some(follower) = self.follower.as_mut() else { return };
        let Ok(mut entries) = follower.read_new() else { return };
        if fresh && entries.len() > FIRST_READ_LIMIT {
            entries = entries.split_off(entries.len() - FIRST_READ_LIMIT);
        }
        self.backlog.extend(entries.iter().filter_map(|e| pushed(e, provider.command(), session)));
        if self.backlog.len() > BACKLOG {
            let drop = self.backlog.len() - BACKLOG;
            self.backlog.drain(..drop);
        }
    }

    fn send(&mut self, door: &Door) -> Result<(), String> {
        for chunk in self.backlog.chunks(100) {
            door.call("push", Some(&json!({ "entries": chunk })))?;
        }
        self.backlog.clear();
        Ok(())
    }
}

/// Types what the phone sent into the session, oldest first, and tells the Inbox which went in. A
/// message the session cannot take yet (asleep, switching) stays waiting for the next round.
fn collect(app: &AppHandle, door: &Door) -> Result<(), String> {
    let waiting = door.call("outbox", None)?;
    let mut done: Vec<u64> = Vec::new();
    for item in waiting.get("items").and_then(Value::as_array).into_iter().flatten() {
        let (Some(id), Some(body)) = (item.get("id").and_then(Value::as_u64), item.get("body").and_then(Value::as_str)) else { continue };
        if crate::messages::send_message(phone_text(body), None, app.clone()).is_err() {
            break;
        }
        done.push(id);
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
        let mut mirror = Mirror::default();
        loop {
            std::thread::sleep(TICK);
            let state = app.state::<BridgeState>();
            let settings = app.state::<SettingsStore>().get();
            let token = crate::key::load(ACCOUNT);
            let (true, Some(token)) = (settings.inbox_bridge_on, token) else {
                mirror = Mirror::default();
                *state.connected.lock().unwrap() = false;
                *state.problem.lock().unwrap() = None;
                continue;
            };
            let outcome = Door::new(settings.inbox_bridge_url.as_deref().unwrap_or(""), &token).and_then(|door| {
                mirror.read(&app);
                mirror.send(&door)?;
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

    fn entry(id: &str, role: &str, text: &str) -> ChatEntry {
        ChatEntry { id: id.into(), role: role.into(), text: text.into(), at: Some("2026-11-15T22:00:00Z".into()), path: None, pictures: Vec::new(), state: None }
    }

    #[test]
    fn the_door_address_follows_the_setting_or_this_computers_own() {
        assert_eq!(bridge_url("", "push"), "http://localhost/mira-inbox/bridge.php?a=push");
        assert_eq!(bridge_url("  https://example.test/inbox/ ", "outbox"), "https://example.test/inbox/bridge.php?a=outbox");
    }

    #[test]
    fn only_the_members_words_and_the_companions_replies_are_sent() {
        let said = pushed(&entry("1", "member", " hello "), "claude", 3).unwrap();
        assert_eq!(said["role"], "me");
        assert_eq!(said["text"], "hello");
        assert_eq!(pushed(&entry("2", "companion", "hi"), "claude", 3).unwrap()["role"], "companion");
        for role in ["work", "plan", "receipt", "picture", "voice"] {
            assert!(pushed(&entry("3", role, "x"), "claude", 3).is_none(), "{role}");
        }
        assert!(pushed(&entry("4", "member", "   "), "claude", 3).is_none());
    }

    #[test]
    fn a_line_keeps_its_key_and_two_sessions_never_share_one() {
        let a = pushed(&entry("1", "member", "x"), "claude", 3).unwrap();
        assert_eq!(a["key"], pushed(&entry("1", "member", "x"), "claude", 3).unwrap()["key"]);
        assert_ne!(a["key"], pushed(&entry("1", "member", "x"), "claude", 4).unwrap()["key"]);
        assert_ne!(a["key"], pushed(&entry("1", "member", "x"), "codex", 3).unwrap()["key"]);
        assert_eq!(a["key"].as_str().unwrap().len(), 32);
    }

    #[test]
    fn a_shared_picture_is_told_in_words() {
        let mut shared = entry("5", "member", "look");
        shared.pictures = vec!["C:\\a.png".into()];
        assert_eq!(pushed(&shared, "claude", 1).unwrap()["text"], "look\n[a picture was shared]");
        let mut only = entry("6", "member", "");
        only.pictures = vec!["C:\\a.png".into(), "C:\\b.png".into()];
        assert_eq!(pushed(&only, "claude", 1).unwrap()["text"], "[pictures were shared]");
    }

    #[test]
    fn a_phone_message_says_where_it_came_from() {
        assert_eq!(phone_text("  are you there? "), "[Sent from my phone] are you there?");
    }
}
