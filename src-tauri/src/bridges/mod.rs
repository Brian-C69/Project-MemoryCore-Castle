//! MemoryCore in your pocket: lets a platform (Telegram today, more later) talk to the member's
//! own companion - the real one, with its own memory and tools, not a sandboxed reply. Never the
//! shared Foundation chat room (see roomturn.rs/inboxbridge.rs for that, a different thing).
//! Each adapter only needs to turn its own transport's messages into calls to `send_and_await`.

pub mod discord;
pub mod telegram;
pub mod whatsapp;

use crate::session::{MainHost, Room};
use crate::settings::SettingsStore;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// Types the member's words into the companion's real session (waking it first if it is asleep)
/// and waits for its reply, by following the session's own transcript - the same record the DEV
/// tab's Chat reads, just with this call's own independent follower so the two never race.
pub fn send_and_await(app: &AppHandle, store: &SettingsStore, text: &str, timeout: Duration) -> Result<String, String> {
    let host = &app.state::<MainHost>().0;
    let fresh = !host.is_running();
    if fresh {
        // A session that is not running yet can take the first message as its own launch
        // argument - no PTY write race against a CLI that has not started reading stdin yet.
        crate::session::wake_with_prompt(app, store, host, Room::Main, 120, 40, Some(text))?;
    }
    let (_, started) = host.started().ok_or("Your companion's session did not start.")?;
    let settings = store.get();
    let folder = Room::Main.folder(&settings).ok_or("No MemoryCore folder is chosen yet.")?;
    let cli = host.provider().unwrap_or(settings.cli);
    // A session that just woke takes a moment to write its first record; a running one already
    // has one. Either way, wait rather than fail the whole message over a one-second gap.
    let found_at = Instant::now() + Duration::from_secs(15).min(timeout);
    let (path, follow) = loop {
        if let Some(found) = crate::chat::newest_record(cli, &folder, started) {
            break found;
        }
        if Instant::now() >= found_at {
            return Err("Your companion's session has no record yet. Try again in a moment.".into());
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    let mut follower = follow(path, started);
    if !fresh {
        // Already running: anything already said is not this message's reply, so it is skipped
        // before the word is typed in - never after, or a fast answer could be skipped too.
        let _ = follower.read_new();
        host.write(text.as_bytes())?;
        std::thread::sleep(Duration::from_millis(150));
        host.write(b"\r")?;
    }
    let deadline = Instant::now() + timeout;
    // A companion often says something first ("let me check...") and works before the real answer,
    // so the first words are not the reply. It is taken as done once it has been quiet for a while;
    // the answer is the last thing it said.
    let mut last_said: Option<String> = None;
    let mut quiet_since = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if let Ok(entries) = follower.read_new() {
            if !entries.is_empty() {
                quiet_since = Instant::now();
            }
            if let Some(said) = entries.iter().rev().find(|e| e.role == "companion" && !e.text.trim().is_empty()) {
                last_said = Some(said.text.trim().to_string());
            }
        }
        if let Some(said) = &last_said {
            if quiet_since.elapsed() >= QUIET_AFTER {
                return Ok(said.clone());
            }
        }
        if Instant::now() >= deadline {
            return match last_said {
                Some(said) => Ok(said),
                None => Err("Your companion took too long to answer.".into()),
            };
        }
    }
}

/// How long the companion must stay silent before what it last said counts as its answer.
const QUIET_AFTER: Duration = Duration::from_secs(12);
