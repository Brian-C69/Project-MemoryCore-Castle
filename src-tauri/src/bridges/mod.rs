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
    loop {
        if Instant::now() >= deadline {
            return Err("Your companion took too long to answer.".into());
        }
        std::thread::sleep(Duration::from_millis(500));
        let Ok(entries) = follower.read_new() else { continue };
        let reply: String = entries.iter().filter(|e| e.role == "companion" && !e.text.trim().is_empty()).map(|e| e.text.trim()).collect::<Vec<_>>().join("\n");
        if !reply.is_empty() {
            return Ok(reply);
        }
    }
}
