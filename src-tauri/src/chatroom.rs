//! The MemoryCore Foundation chat room, from the member's side (the Portfolio's room contract,
//! /api/room/**). Every call goes from Rust with the member's room key from Credential Manager;
//! the key never reaches the page. Lines are plain text, shown as text.

use crate::key;
use crate::settings::SettingsStore;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tauri::State;

const ROOM_RULES: &str = include_str!("../resources/room-rules.md");
const USER_AGENT: &str = concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION"));

/// What the page needs to know when a door says no - in words, with the kind of no.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoomRefusal {
    /// "locked" (not a member / revoked) | "muted" | "needsName" | "tooLarge" | "wrongType" |
    /// "slowDown" | "notSetUp" | "offline" | "other"
    pub kind: String,
    pub message: String,
}

fn refusal(kind: &str, message: impl Into<String>) -> RoomRefusal {
    RoomRefusal { kind: kind.into(), message: message.into() }
}

/// Maps a door's status and its {"error": "..."} body to a plain refusal.
pub fn refusal_for(status: u16, body: &str) -> RoomRefusal {
    let said = serde_json::from_str::<Value>(body).ok().and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string));
    match status {
        403 if said.as_deref().is_some_and(|s| s.to_lowercase().contains("muted")) => refusal("muted", said.unwrap_or_default()),
        403 | 401 => refusal("locked", said.unwrap_or_else(|| "This room is for MemoryCore Foundation members".into())),
        409 => refusal("needsName", "Set your Foundation display name on the Portfolio to speak in the room."),
        413 => refusal("tooLarge", "That message is too long (1000 characters at most)."),
        415 => refusal("wrongType", "That file type is not allowed in the room."),
        429 => refusal("slowDown", "Too many messages at once - wait a moment."),
        _ => refusal("other", said.unwrap_or_else(|| format!("The room answered {status}."))),
    }
}

/// The room's address: the Portfolio address the member saved, plus /api/room.
pub fn room_url(base: &str, path: &str) -> String {
    format!("{}/api/room{}", base.trim_end_matches('/').trim_end_matches("/api"), path)
}

pub struct Room {
    base: String,
    key: String,
}

impl Room {
    pub fn new(base: &str, key: &str) -> Room {
        Room { base: base.to_string(), key: key.to_string() }
    }

    fn client(&self, timeout: Duration) -> Result<reqwest::blocking::Client, RoomRefusal> {
        reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .build()
            .map_err(|_| refusal("other", "The room client could not start."))
    }

    fn answer(response: reqwest::Result<reqwest::blocking::Response>) -> Result<reqwest::blocking::Response, RoomRefusal> {
        let response = response.map_err(|e| {
            if e.is_timeout() {
                refusal("offline", "The room did not answer in time.")
            } else {
                refusal("offline", "The room could not be reached. Check the address and the internet connection.")
            }
        })?;
        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            Ok(response)
        } else {
            Err(refusal_for(status, &response.text().unwrap_or_default()))
        }
    }

    fn get_json(&self, path: &str, timeout: Duration) -> Result<Value, RoomRefusal> {
        let response = self.client(timeout)?.get(room_url(&self.base, path)).bearer_auth(&self.key).send();
        Self::answer(response)?.json().map_err(|_| refusal("other", "The room sent something unreadable."))
    }

    pub fn me(&self) -> Result<Value, RoomRefusal> {
        self.get_json("/me", Duration::from_secs(15))
    }

    /// The lines after `after` (the newest 50 without it); `wait` holds the read up to 25 s.
    pub fn lines(&self, after: Option<u64>, wait: u64) -> Result<Value, RoomRefusal> {
        let wait = wait.min(25);
        let path = match after {
            Some(id) => format!("/lines?after={id}&wait={wait}"),
            None => format!("/lines?wait={wait}"),
        };
        // the HTTP timeout stays well above the held read
        self.get_json(&path, Duration::from_secs(wait + 10))
    }

    pub fn presence(&self) -> Result<Value, RoomRefusal> {
        self.get_json("/presence", Duration::from_secs(15))
    }

    pub fn post(&self, text: &str) -> Result<Value, RoomRefusal> {
        let text = text.trim();
        if text.is_empty() {
            return Err(refusal("other", "Write something first."));
        }
        if text.chars().count() > 1000 {
            return Err(refusal("tooLarge", "That message is too long (1000 characters at most)."));
        }
        let response = self
            .client(Duration::from_secs(20))?
            .post(room_url(&self.base, "/lines"))
            .bearer_auth(&self.key)
            .json(&serde_json::json!({ "text": text }))
            .send();
        Self::answer(response)?.json().map_err(|_| refusal("other", "The room sent something unreadable."))
    }

    /// A room picture or voice as a data URL (fetched here, so the page never calls the Portfolio).
    pub fn file(&self, id: &str) -> Result<String, RoomRefusal> {
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(refusal("other", "That is not a room file."));
        }
        let response = self.client(Duration::from_secs(30))?.get(room_url(&self.base, &format!("/files/{id}"))).bearer_auth(&self.key).send();
        let response = Self::answer(response)?;
        let kind = response.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
        if !(kind.starts_with("image/") || kind.starts_with("audio/")) {
            return Err(refusal("wrongType", "That room file is not a picture or a voice."));
        }
        let bytes = response.bytes().map_err(|_| refusal("offline", "The file could not be read."))?;
        Ok(format!("data:{kind};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }
}

/// The member's room (who: "me" for their own key, "companion" for their companion's key).
fn room_for(store: &SettingsStore, who: &str) -> Result<Room, RoomRefusal> {
    if !store.get().room_on("chat") {
        return Err(refusal("off", crate::rooms::off_line("chat")));
    }
    let base = store.get().room_base_url.ok_or_else(|| refusal("notSetUp", "Add the room address in the CHAT room first."))?;
    let account = if who == "companion" { "room-companion" } else { "room" };
    let key = key::load(account).ok_or_else(|| {
        refusal("notSetUp", if who == "companion" { "Add your companion's room key to let it speak in the room." } else { "Add your room key to enter." })
    })?;
    Ok(Room::new(&base, &key))
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, RoomRefusal> + Send + 'static) -> Result<T, RoomRefusal> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| refusal("other", "The room call stopped."))?
}

#[tauri::command]
pub async fn room_me(store: State<'_, SettingsStore>) -> Result<Value, RoomRefusal> {
    let room = room_for(&store, "me")?;
    blocking(move || room.me()).await
}

#[tauri::command]
pub async fn room_lines(after: Option<u64>, wait: u64, store: State<'_, SettingsStore>) -> Result<Value, RoomRefusal> {
    let room = room_for(&store, "me")?;
    blocking(move || room.lines(after, wait)).await
}

#[tauri::command]
pub async fn room_presence(store: State<'_, SettingsStore>) -> Result<Value, RoomRefusal> {
    let room = room_for(&store, "me")?;
    blocking(move || room.presence()).await
}

/// Posts a line as the member ("me") or as their companion ("companion", its own room key).
#[tauri::command]
pub async fn room_post(text: String, who: Option<String>, store: State<'_, SettingsStore>) -> Result<Value, RoomRefusal> {
    let room = room_for(&store, who.as_deref().unwrap_or("me"))?;
    blocking(move || room.post(&text)).await
}

#[tauri::command]
pub async fn room_file(id: String, store: State<'_, SettingsStore>) -> Result<String, RoomRefusal> {
    let room = room_for(&store, "me")?;
    blocking(move || room.file(&id)).await
}

/// The rules the companion follows in the room, with the names filled in.
#[tauri::command]
pub fn room_rules(store: State<SettingsStore>) -> String {
    let s = store.get();
    ROOM_RULES.replace("{{AI_NAME}}", &s.companion_label()).replace("{{USER_NAME}}", s.member_name.as_deref().unwrap_or("the member"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A one-request mock room: answers `status` with `body`, and returns the request it saw.
    fn mock(status: u16, body: &'static str, kind: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let reply = format!("HTTP/1.1 {status} X\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(reply.as_bytes()).unwrap();
            request
        });
        (base, handle)
    }

    #[test]
    fn the_room_address_is_built_from_the_portfolio_address() {
        assert_eq!(room_url("https://example.test/", "/me"), "https://example.test/api/room/me");
        assert_eq!(room_url("https://example.test/api", "/lines"), "https://example.test/api/room/lines");
    }

    #[test]
    fn refusals_are_told_apart() {
        assert_eq!(refusal_for(403, r#"{"error":"This room is for MemoryCore Foundation members"}"#).kind, "locked");
        assert_eq!(refusal_for(403, r#"{"error":"You are muted in this room for now"}"#).kind, "muted");
        assert_eq!(refusal_for(409, "{}").kind, "needsName");
        assert_eq!(refusal_for(413, "{}").kind, "tooLarge");
        assert_eq!(refusal_for(415, "{}").kind, "wrongType");
        assert_eq!(refusal_for(429, "{}").kind, "slowDown");
    }

    #[test]
    fn a_held_read_sends_the_key_the_cursor_and_a_named_agent() {
        let (base, seen) = mock(200, r#"{"cursor":12,"lines":[{"id":12,"text":"hi","author":{"name":"Rin","kind":"member"}}],"cleared":[],"more":false}"#, "application/json");
        let answer = Room::new(&base, "mcr_test").lines(Some(11), 2).unwrap();
        assert_eq!(answer["cursor"], 12);
        let request = seen.join().unwrap();
        assert!(request.starts_with("GET /api/room/lines?after=11&wait=2 "));
        assert!(request.to_lowercase().contains("authorization: bearer mcr_test"));
        assert!(request.contains("MemoryCoreCastle/"));
    }

    #[test]
    fn a_locked_door_and_a_long_message_are_refused_plainly() {
        let (base, seen) = mock(403, r#"{"error":"This room is for MemoryCore Foundation members"}"#, "application/json");
        let refused = Room::new(&base, "mcr_old").me().unwrap_err();
        assert_eq!(refused.kind, "locked");
        let _ = seen.join();
        assert_eq!(Room::new("http://127.0.0.1:9", "k").post(&"x".repeat(1001)).unwrap_err().kind, "tooLarge");
        assert!(Room::new("http://127.0.0.1:9", "k").file("../etc").is_err());
    }

    #[test]
    fn a_room_picture_comes_back_as_a_data_url() {
        let (base, seen) = mock(200, "PNG", "image/png");
        let url = Room::new(&base, "mcr_test").file("abc123").unwrap();
        assert_eq!(url, "data:image/png;base64,UE5H");
        let _ = seen.join();
    }
}
