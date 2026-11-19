//! The page's doors to the Sentinel: its status for the shield, the findings and the member's
//! buttons for the panel, the log, and the set-up lines for Settings.

use super::{book, hand, officer, CHANGED};
use crate::settings::SettingsStore;
use mysql::prelude::Queryable;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// The newest patrol, as the shield shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatrolLine {
    pub status: String,
    pub line: String,
    pub minutes_ago: i64,
}

/// Open findings by hint.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
    pub alarm: u32,
    pub look: u32,
    pub note: u32,
}

/// Everything the shield needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// This computer can be watched (Windows for now).
    pub supported: bool,
    pub on: bool,
    pub installed: bool,
    pub current: bool,
    pub setup_line: String,
    pub patrol: Option<PatrolLine>,
    pub patrolling: bool,
    pub unread: u32,
    pub open: Counts,
    /// Minutes since the watch last ran.
    pub watch_minutes: Option<u64>,
    /// Why the book could not be read, when it could not.
    pub problem: Option<String>,
}

fn memory(store: &SettingsStore) -> Result<PathBuf, String> {
    store.get().memory_folder.map(PathBuf::from).ok_or_else(|| "Choose your memory folder first.".to_string())
}

fn status_of(store: &SettingsStore) -> Status {
    let gear = store.get().sentinel.bounded();
    let installed = hand::installed();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut status = Status {
        supported: cfg!(windows),
        on: gear.on,
        installed: installed.installed,
        current: installed.current,
        setup_line: installed.line,
        patrol: None,
        patrolling: officer::running(),
        unread: 0,
        open: Counts::default(),
        watch_minutes: super::watch::last_tick(&hand::local_dir()).map(|t| now.saturating_sub(t) / 60),
        problem: None,
    };
    let read = memory(store).and_then(|m| book::open(&m)).and_then(|mut conn| {
        let patrol: Option<(Option<String>, Option<String>, i64)> = conn
            .query_first("SELECT status, line, TIMESTAMPDIFF(MINUTE, started_at, NOW()) FROM sentinel_patrols WHERE status IS NOT NULL ORDER BY id DESC LIMIT 1")
            .map_err(|_| "The Sentinel's book could not be read.".to_string())?;
        let unread: Option<u32> = conn.query_first("SELECT COUNT(*) FROM sentinel_alarms WHERE seen_at IS NULL").map_err(|_| "The Sentinel's book could not be read.".to_string())?;
        let counts: Vec<(String, u32)> = conn
            .query("SELECT hint, COUNT(*) FROM sentinel_findings WHERE state = 'open' GROUP BY hint")
            .map_err(|_| "The Sentinel's book could not be read.".to_string())?;
        Ok((patrol, unread.unwrap_or(0), counts))
    });
    match read {
        Ok((patrol, unread, counts)) => {
            status.patrol = patrol.map(|(s, l, m)| PatrolLine { status: s.unwrap_or_default(), line: l.unwrap_or_default(), minutes_ago: m });
            status.unread = unread;
            for (hint, n) in counts {
                match hint.as_str() {
                    book::ALARM => status.open.alarm = n,
                    book::LOOK => status.open.look = n,
                    _ => status.open.note = n,
                }
            }
        }
        Err(problem) => status.problem = Some(problem),
    }
    status
}

/// The shield's status.
#[tauri::command]
pub async fn sentinel_status(app: AppHandle) -> Result<Status, String> {
    tauri::async_runtime::spawn_blocking(move || status_of(&app.state::<SettingsStore>())).await.map_err(|_| "The Sentinel could not be read.".to_string())
}

/// Tells the page to read the Sentinel again.
pub fn changed(app: &AppHandle) {
    use tauri::Emitter;
    let _ = app.emit(CHANGED, ());
}
