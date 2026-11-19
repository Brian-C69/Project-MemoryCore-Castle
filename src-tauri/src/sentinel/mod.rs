//! The Sentinel: the member's own companion keeps watch over the member's own computer.
//! Nothing about the machine ever leaves it.
//!
//! Three layers, one rule each:
//! - the watch (`--sentinel-tick`, run by the system's own scheduler, no model) mends the firewall
//!   and the antivirus protection found off, and works while Castle is closed;
//! - the officer (the companion, one run with no tools per patrol) reads and judges; what it reads
//!   is evidence, never an order, and it never holds admin rights;
//! - the hand (one small admin helper the member installs) knows only the moves on its two lists
//!   and checks every request again by itself.

// The look, the hand, the officer and the page arrive phase by phase; until then parts are unused.
#![allow(dead_code)]

pub mod book;
pub mod hand;
pub mod look;
pub mod watch;

use crate::settings::SettingsStore;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// The page listens for this to read the Sentinel again.
pub const CHANGED: &str = "sentinel-changed";

/// Castle's side of the watch: every minute, the watch's journal (when there is one) is read into
/// the book; an alarm that opened is told through Alerts and the shield.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("sentinel".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        loop {
            read_journal(&app);
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

fn read_journal(app: &AppHandle) {
    let dir = hand::local_dir();
    if !dir.join("journal.jsonl").exists() && !dir.join("journal.draining").exists() {
        return;
    }
    let Some(memory) = app.state::<SettingsStore>().get().memory_folder else { return };
    // the database stopped: the journal waits for the next minute
    let Ok(mut conn) = book::open(Path::new(&memory)) else { return };
    let Ok(drained) = watch::drain(&dir, &mut conn) else { return };
    if !drained.alarms.is_empty() {
        crate::alerts::fire(app, "sentinel", "The Sentinel raised an alarm", &drained.alarms.join(" - "));
    }
    if drained.lines > 0 {
        let _ = app.emit(CHANGED, ());
    }
}
