//! Health belongs to the backend process, never to a window or page timer.
use super::*;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
static GENERATION: AtomicU64 = AtomicU64::new(0);
pub(super) fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}
pub fn interrupt(app: &AppHandle) {
    app.state::<Health>().request_interruption();
    super::events::discard_pending();
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    started_at: i64,
    last_heartbeat: Option<i64>,
    last_scan: Option<i64>,
    last_database_check: Option<i64>,
    database_ready: Option<bool>,
    problem: Option<String>,
    last_interruption: Option<String>,
    last_interruption_at: Option<i64>,
}
#[derive(Default)]
pub struct Health {
    data: Mutex<Data>,
    pending: AtomicUsize,
    interrupted: Mutex<Option<&'static str>>,
}
impl Health {
    fn request_interruption(&self) {
        *self.interrupted.lock().unwrap_or_else(|p| p.into_inner()) =
            Some("Automations were off or paused");
        GENERATION.fetch_add(1, Ordering::AcqRel);
    }
    pub(super) fn take_interruption(&self) -> Option<&'static str> {
        self.interrupted
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
    }

    fn change(&self, f: impl FnOnce(&mut Data)) {
        f(&mut self.data.lock().unwrap_or_else(|p| p.into_inner()));
    }
    pub fn beat(&self, now: i64) {
        self.change(|d| {
            if d.started_at == 0 {
                d.started_at = now;
            }
            d.last_heartbeat = Some(now);
        });
    }
    pub fn database(&self, ready: bool, problem: Option<String>) {
        self.change(|d| {
            d.last_database_check = Some(chrono::Utc::now().timestamp_millis());
            d.database_ready = Some(ready);
            d.problem = problem;
        });
    }
    pub fn scan(&self, problem: Option<String>) {
        self.change(|d| {
            d.last_scan = Some(chrono::Utc::now().timestamp_millis());
            d.problem = problem;
        });
    }
    pub fn interruption(&self, reason: &str) {
        self.change(|d| {
            d.last_interruption = Some(reason.into());
            d.last_interruption_at = Some(chrono::Utc::now().timestamp_millis());
        });
    }
    pub fn snapshot(
        &self,
        s: &crate::settings::Settings,
        active: Option<usize>,
        now: i64,
    ) -> Value {
        let d = self.data.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let pending = self.pending.load(Ordering::Acquire);
        let mode = if !s.room_on("automations") {
            "off"
        } else if s.automations_paused {
            "paused"
        } else if d.last_heartbeat.is_none() {
            "starting"
        } else if d.last_heartbeat.is_some_and(|t| now < t || now - t > 45000) {
            "stale"
        } else if d.database_ready == Some(false) {
            "database_unavailable"
        } else if d.problem.is_some() {
            "attention"
        } else if d.last_scan.is_none() {
            "starting"
        } else if pending > 0 {
            "saving"
        } else {
            "running"
        };
        json!({"mode":mode,"lastHeartbeat":d.last_heartbeat,"lastScan":d.last_scan,"lastDatabaseCheck":d.last_database_check,"databaseReady":d.database_ready,"problem":d.problem,"lastInterruption":d.last_interruption,"lastInterruptionAt":d.last_interruption_at,"activeRuns":active,"pendingResults":pending,"tickSeconds":15,"startWithSystem":s.start_with_windows})
    }
}
pub struct Pending(tauri::AppHandle);
impl Pending {
    pub fn new(app: &AppHandle) -> Self {
        app.state::<Health>().pending.fetch_add(1, Ordering::AcqRel);
        Self(app.clone())
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.0
            .state::<Health>()
            .pending
            .fetch_sub(1, Ordering::AcqRel);
    }
}
#[tauri::command]
pub fn automation_health(app: AppHandle) -> Value {
    let store = app.state::<SettingsStore>();
    let active = app.state::<Running>().0.try_lock().ok().map(|r| r.len());
    let mut result =
        app.state::<Health>()
            .snapshot(&store.get(), active, chrono::Utc::now().timestamp_millis());
    result["startupProblem"] = json!(store.startup_note.lock().ok().and_then(|p| p.clone()));
    result
}
/// A successful scan consumes the interruption once. Failed scans keep it pending.
pub struct Continuity {
    last: Option<i64>,
    reason: Option<&'static str>,
}
impl Default for Continuity {
    fn default() -> Self {
        Self {
            last: None,
            reason: Some("Castle started"),
        }
    }
}
impl Continuity {
    pub fn tick(&mut self, now: i64) -> Option<&'static str> {
        if self
            .last
            .is_some_and(|last| now < last || now - last > 45000)
            && self.reason.is_none()
        {
            self.reason = Some("Execution or clock gap");
        }
        self.last = Some(now);
        self.reason
    }
    pub fn interrupted(&mut self, reason: &'static str) {
        self.reason = Some(reason);
    }
    pub fn scanned(&mut self) {
        self.reason = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_pause_database_and_resume_consume_once() {
        let mut c = Continuity::default();
        assert_eq!(c.tick(0), Some("Castle started"));
        c.scanned();
        assert!(c.tick(15000).is_none());
        assert!(c.tick(90000).is_some());
        c.scanned();
        assert!(c.tick(105000).is_none());
        c.interrupted("Database was unavailable");
        assert!(c.tick(120000).is_some());
        assert!(c.tick(135000).is_some());
        c.scanned();
        c.interrupted("Automations were paused");
        assert!(c.tick(150000).is_some());
        c.scanned();
        assert!(c.tick(100).is_some());
    }
    #[test]
    fn health_remains_readable_without_database_and_detects_stall() {
        let h = Health::default();
        let mut s = crate::settings::Settings::default();
        s.rooms.insert("automations".into(), true);
        assert_eq!(h.snapshot(&s, Some(0), 0)["mode"], "starting");
        h.beat(1000);
        h.database(false, Some("The database is unavailable.".into()));
        assert_eq!(
            h.snapshot(&s, Some(1), 2000)["mode"],
            "database_unavailable"
        );
        assert_eq!(h.snapshot(&s, None, 50000)["mode"], "stale");
        s.automations_paused = true;
        assert_eq!(h.snapshot(&s, Some(1), 50000)["mode"], "paused");
        s.rooms.insert("automations".into(), false);
        assert_eq!(h.snapshot(&s, Some(0), 50000)["mode"], "off");
    }
}
#[cfg(test)]
mod connection_tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn stalled_database_handshake_is_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = stream.write_all(b"");
        });
        let folder =
            std::env::temp_dir().join(format!("castle-phase8-connection-{}", std::process::id()));
        assert!(
            !folder.exists(),
            "owned fixture already exists; inspect before retrying"
        );
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join(".env"),format!("MEMORY_DB_PORT={port}\nMEMORY_DB_NAME=fixture\nMEMORY_DB_USER=fixture\nMEMORY_DB_PASSWORD=fixture\n")).unwrap();
        let started = std::time::Instant::now();
        assert!(
            castle_actions::bounded_connection(&folder, std::time::Duration::from_millis(150))
                .is_err()
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        server.join().unwrap();
        std::fs::remove_file(folder.join(".env")).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
#[cfg(test)]
mod transition_tests {
    use super::*;
    #[test]
    fn brief_pause_is_not_lost_between_runner_ticks() {
        let h = Health::default();
        h.request_interruption();
        h.request_interruption();
        assert_eq!(
            h.take_interruption(),
            Some("Automations were off or paused")
        );
        assert!(h.take_interruption().is_none());
        h.beat(1000);
        h.database(true, None);
        h.scan(None);
        let mut s = crate::settings::Settings::default();
        s.rooms.insert("automations".into(), true);
        h.pending.store(2, Ordering::Release);
        assert_eq!(h.snapshot(&s, Some(2), 2000)["mode"], "saving");
    }
}
