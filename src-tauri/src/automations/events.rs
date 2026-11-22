//! Phase 6: Castle's own events. A fixed list (adding one is a single `events::emit(...)` line at
//! the place it happens). Two kinds of source:
//! - things this program does itself (a backup, a provider switch) call `emit`;
//! - things written to the database by another process (the companion's `castle-memory` saving a
//!   diary entry, a task ticked, a reminder falling due) are noticed by a quiet poll.
//! An event only queues a line of text; the dispatcher starts the jobs on its own thread, so a
//! source never waits for an automation. Event text is DATA (`payload.text`), like every payload.
use super::*;
use mysql::prelude::Queryable;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// (name, what the member sees). The one list; the editor shows the same names.
pub const EVENTS: [(&str, &str); 6] = [
    ("backup_done", "Memory backup finished"),
    ("backup_failed", "Memory backup failed"),
    ("provider_switched", "Companion provider switched"),
    ("diary_saved", "Diary entry saved"),
    ("task_done", "Project task ticked"),
    ("reminder_due", "Reminder due today"),
];
pub const MAX_JOBS: u64 = 16;
const QUEUE_MAX: usize = 50;
/// Event runs allowed in any ten minutes, all jobs together: a loop is stopped, not obeyed.
const STORM_MAX: usize = 20;
const STORM_WINDOW: Duration = Duration::from_secs(600);
const POLL_EVERY: Duration = Duration::from_secs(15);

static QUEUE: Mutex<VecDeque<(String, String)>> = Mutex::new(VecDeque::new());
static PROBLEM: Mutex<Option<String>> = Mutex::new(None);
pub(super) fn discard_pending(){QUEUE.lock().unwrap_or_else(|p|p.into_inner()).clear();}
pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}

/// `{"event":"backup_failed"}` and nothing else.
pub fn spec(v: &Value) -> Result<String, String> {
    let name = v["event"].as_str().unwrap_or("");
    if v.as_object().is_none_or(|o| o.len() != 1) || !EVENTS.iter().any(|(n, _)| *n == name) {
        return Err("Choose one of Castle's events.".into());
    }
    Ok(name.to_string())
}

/// Combinations that would start themselves again. The storm limit is the second line of defence.
pub fn conflict(event: &str, action: &Value) -> Result<(), String> {
    let act = action["action"].as_str().unwrap_or("");
    if action["kind"] == "castle"
        && ((event == "diary_saved" && act == "save_diary_prompt") || (event == "reminder_due" && act == "add_reminder"))
    {
        return Err("That action would trigger this event again. Choose a different action or event.".into());
    }
    Ok(())
}

/// Called from the place an event happens. Never blocks and never fails the caller.
pub fn emit(name: &str, text: impl Into<String>) {
    if !EVENTS.iter().any(|(n, _)| *n == name) {
        return;
    }
    if let Ok(mut q) = QUEUE.lock() {
        if q.len() >= QUEUE_MAX {
            *PROBLEM.lock().unwrap() = Some("Too many Castle events at once; some were skipped.".into());
            return;
        }
        q.push_back((name.to_string(), text.into().chars().take(500).collect()));
    }
}

#[derive(Default)]
pub struct Storm(Vec<Instant>);
impl Storm {
    pub fn admit(&mut self, now: Instant) -> bool {
        self.0.retain(|t| now.duration_since(*t) < STORM_WINDOW);
        if self.0.len() >= STORM_MAX {
            return false;
        }
        self.0.push(now);
        true
    }
}

pub fn payload(name: &str, text: &str) -> Value {
    json!({"cause":"event","event":name,"text":text,"time":schedule::stamp(chrono::Local::now())})
}

/// Where the database watchers have got to. `None` means "not looked yet": the first look only
/// records the present, so nothing from before the job was switched on ever runs.
#[derive(Default)]
pub struct Marks {
    diary: Option<u64>,
    task: Option<(String, u64)>,
}

fn subscribed(jobs: &[Value], event: &str) -> bool {
    jobs.iter().any(|j| j["trigger"]["event"] == event)
}

/// Looks at the database for the three events another process can cause. Returns new event lines.
pub fn poll(c: &mut Conn, marks: &mut Marks, jobs: &[Value], today: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    if subscribed(jobs, "diary_saved") {
        let newest: Option<u64> = c.query_first("SELECT MAX(id) FROM diary_entries").map_err(|_| "Diary could not be read.")?.flatten();
        match marks.diary {
            None => marks.diary = Some(newest.unwrap_or(0)),
            Some(seen) => {
                let rows: Vec<(u64, String)> = c
                    .exec("SELECT id,title FROM diary_entries WHERE id > ? ORDER BY id LIMIT 20", (seen,))
                    .map_err(|_| "Diary could not be read.")?;
                for (id, title) in rows {
                    marks.diary = Some(id);
                    out.push(("diary_saved".into(), format!("Diary entry saved: {title}")));
                }
            }
        }
    } else {
        marks.diary = None;
    }
    if subscribed(jobs, "task_done") {
        match marks.task.clone() {
            None => {
                let newest: Option<(String, u64)> = c
                    .query_first("SELECT CAST(done_at AS CHAR),id FROM project_tasks WHERE done_at IS NOT NULL ORDER BY done_at DESC,id DESC LIMIT 1")
                    .map_err(|_| "Tasks could not be read.")?;
                marks.task = Some(newest.unwrap_or(("1970-01-01 00:00:00".into(), 0)));
            }
            Some((at, id)) => {
                let rows: Vec<(u64, String, String, String)> = c
                    .exec(
                        "SELECT t.id,t.title,p.name,CAST(t.done_at AS CHAR) FROM project_tasks t JOIN projects p ON p.id=t.project_id \
                         WHERE t.done_at IS NOT NULL AND (t.done_at > ? OR (t.done_at = ? AND t.id > ?)) ORDER BY t.done_at,t.id LIMIT 20",
                        (&at, &at, id),
                    )
                    .map_err(|_| "Tasks could not be read.")?;
                for (id, title, project, done) in rows {
                    marks.task = Some((done, id));
                    out.push(("task_done".into(), format!("Task done in {project}: {title}")));
                }
            }
        }
    } else {
        marks.task = None;
    }
    if subscribed(jobs, "reminder_due") {
        let rows: Vec<(u64, String)> = c
            .exec("SELECT id,title FROM reminders WHERE state='open' AND due_date = ? ORDER BY id LIMIT 50", (today,))
            .map_err(|_| "Reminders could not be read.")?;
        for (id, title) in rows {
            // once per reminder per day, also across a restart (the same receipts message triggers use)
            let key = triggers::event_key("event", "reminder_due", &format!("{id}:{today}"));
            c.exec_drop("INSERT IGNORE INTO automation_messages(event_key,also_chat) VALUES (?,0)", (&key,))
                .map_err(|_| "Reminder receipts unavailable.")?;
            if c.affected_rows() == 1 {
                out.push(("reminder_due".into(), format!("Reminder due today: {title}")));
            }
        }
    }
    Ok(out)
}

fn event_jobs(app: &AppHandle) -> Result<Vec<Value>,String> {
    let store = app.state::<SettingsStore>();
    connection(&store)
        .and_then(|mut c| store::list(&mut c))
        .map(|jobs| jobs.into_iter().filter(|j| j["state"] == "active" && j["enabled"] == 1 && j["triggerKind"] == "event").collect())

}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut storm = Storm::default();
        let mut marks = Marks::default();
        let mut polled = Instant::now();
        let mut generation=health::generation();let mut last_tick=chrono::Utc::now().timestamp_millis();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now=chrono::Utc::now().timestamp_millis();
            if generation!=health::generation()||now<last_tick||now-last_tick>45000{marks=Marks::default();discard_pending();polled=Instant::now();generation=health::generation();}
            last_tick=now;
            let settings = app.state::<SettingsStore>();
            if !allowed(&settings.get()) {
                QUEUE.lock().unwrap().clear();
                marks = Marks::default();
                *PROBLEM.lock().unwrap() = None;
                continue;
            }
            let queued = !QUEUE.lock().unwrap().is_empty();
            let poll_due = polled.elapsed() >= POLL_EVERY;
            if !queued && !poll_due {
                continue;
            }
            let jobs=match event_jobs(&app){Ok(jobs)=>jobs,Err(e)=>{QUEUE.lock().unwrap().clear();marks=Marks::default();polled=Instant::now();*PROBLEM.lock().unwrap()=Some(e);continue;}};
            if jobs.is_empty() {
                *PROBLEM.lock().unwrap()=None;
                QUEUE.lock().unwrap().clear();
                marks = Marks::default();
                polled = Instant::now();
                continue;
            }
            if poll_due {
                polled = Instant::now();
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                match connection(&settings).and_then(|mut c| poll(&mut c, &mut marks, &jobs, &today)) {
                    Ok(found) => {
                        for (name, text) in found {
                            emit(&name, text);
                        }
                    }
                    Err(e) => *PROBLEM.lock().unwrap() = Some(e),
                }
            }
            let batch: Vec<(String, String)> = QUEUE.lock().unwrap().drain(..).collect();
            for (name, text) in batch {
                for j in jobs.iter().filter(|j| j["trigger"]["event"] == name.as_str()) {
                    let Some(id) = j["id"].as_u64() else { continue };
                    if !allowed(&settings.get()) {
                        break;
                    }
                    if !storm.admit(Instant::now()) {
                        *PROBLEM.lock().unwrap() = Some("Event jobs paused briefly: too many ran in ten minutes.".into());
                        continue;
                    }
                    *PROBLEM.lock().unwrap() = None;
                    if let Err(e)=runner::launch_payload(&app,id,"event",payload(&name,&text),Some(j)){*PROBLEM.lock().unwrap()=Some(e);}
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_listed_event_with_no_other_field_is_a_trigger() {
        for (name, _) in EVENTS {
            assert_eq!(spec(&json!({"event": name})).unwrap(), name);
        }
        for bad in [json!({}), json!({"event":"database_stopped"}), json!({"event":"backup_done","x":1}), json!({"event":1}), json!(null), json!("backup_done")] {
            assert!(spec(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_names_match_the_editor_list() {
        // src/components/automations/automations.js keeps the same six, in the same order
        let names: Vec<&str> = EVENTS.iter().map(|e| e.0).collect();
        assert_eq!(names, ["backup_done", "backup_failed", "provider_switched", "diary_saved", "task_done", "reminder_due"]);
    }

    #[test]
    fn an_action_that_causes_its_own_event_is_refused() {
        let a = |name: &str| json!({"kind":"castle","action":name});
        assert!(conflict("diary_saved", &a("save_diary_prompt")).is_err());
        assert!(conflict("reminder_due", &a("add_reminder")).is_err());
        assert!(conflict("backup_failed", &a("save_diary_prompt")).is_ok());
        assert!(conflict("task_done", &a("add_task")).is_ok());
        assert!(conflict("diary_saved", &json!({"mode":"think"})).is_ok());
    }

    #[test]
    fn unknown_events_are_ignored_and_the_queue_is_bounded() {
        QUEUE.lock().unwrap().clear();
        emit("not_an_event", "x");
        assert!(QUEUE.lock().unwrap().is_empty());
        for n in 0..QUEUE_MAX + 10 {
            emit("backup_done", format!("{n}"));
        }
        assert_eq!(QUEUE.lock().unwrap().len(), QUEUE_MAX);
        assert!(problem().is_some());
        emit("backup_failed", "x".repeat(2000));
        QUEUE.lock().unwrap().clear();
        *PROBLEM.lock().unwrap() = None;
    }

    #[test]
    fn a_runaway_is_stopped_by_the_storm_limit_and_recovers() {
        let t = Instant::now();
        let mut s = Storm::default();
        for _ in 0..STORM_MAX {
            assert!(s.admit(t));
        }
        assert!(!s.admit(t));
        assert!(s.admit(t + STORM_WINDOW + Duration::from_secs(1)));
    }

    #[test]
    fn the_payload_is_plain_data() {
        let p = payload("task_done", "Task done in X: ; calc & {{payload.text}}");
        assert_eq!(p["cause"], "event");
        assert_eq!(p["event"], "task_done");
        assert_eq!(p["text"], "Task done in X: ; calc & {{payload.text}}");
    }

    #[test]
    fn event_jobs_may_use_session_actions_but_never_with_payload_text() {
        let mut j = json!({"name":"After backup","triggerKind":"event","trigger":{"event":"backup_done"},"action":{"kind":"castle","action":"notify","text":"Backed up. {{payload.text}}"},"delivery":[],"policy":{}});
        assert!(validate(&j).is_ok());
        j["action"] = json!({"kind":"castle","action":"save_diary_prompt","allow_tools":true});
        assert!(validate(&j).is_ok());
        j["action"] = json!({"kind":"castle","action":"save_diary_prompt","allow_tools":true,"text":"{{payload.text}}"});
        assert!(validate(&j).is_err());
        j["trigger"] = json!({"event":"diary_saved"});
        j["action"] = json!({"kind":"castle","action":"save_diary_prompt","allow_tools":true});
        assert!(validate(&j).is_err());
        j["action"] = json!({"mode":"think","prompt":"Summarise: {{payload.text}}"});
        assert!(validate(&j).is_ok());
    }

    /// Opt-in isolated database: the generated test database and user are created and dropped here.
    /// `CASTLE_TEST_DB=port:admin:password`, or the installed database's port and stored root key.
    #[test]
    #[ignore]
    fn live_database_events_baseline_once_and_never_repeat() {
        let config = std::env::var("CASTLE_TEST_DB").unwrap_or_else(|_| {
            let folder = std::env::var("APPDATA").unwrap() + "/com.memorycore.castle/settings.json";
            let port = serde_json::from_str::<Value>(&std::fs::read_to_string(folder).unwrap()).unwrap()["dbPort"].as_u64().unwrap();
            format!("{port}:root:{}", crate::key::db_root().unwrap_or_default())
        });
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let (user, pass) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &user, &pass, &format!("Events Test {}", std::process::id())).unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        c.query_drop("INSERT INTO projects(name,shelf) VALUES ('Admin','active')").unwrap();
        let jobs = vec![json!({"trigger":{"event":"diary_saved"}}), json!({"trigger":{"event":"task_done"}}), json!({"trigger":{"event":"reminder_due"}})];
        let mut marks = Marks::default();
        // old rows exist before anyone listens
        c.query_drop("INSERT INTO diary_entries(entry_date,title,body) VALUES ('2026-01-01','Old page','x')").unwrap();
        c.query_drop("INSERT INTO project_tasks(project_id,title,done_at) SELECT id,'Old task',NOW() FROM projects LIMIT 1").unwrap();
        c.query_drop("INSERT INTO reminders(title,due_date,detail) VALUES ('Old','2026-01-01','')").unwrap();
        assert!(poll(&mut c, &mut marks, &jobs, "2026-11-21").unwrap().is_empty(), "the first look only records the present");
        assert!(poll(&mut c, &mut marks, &jobs, "2026-11-21").unwrap().is_empty());
        c.query_drop("INSERT INTO diary_entries(entry_date,title,body) VALUES ('2026-11-21','The calm Saturday','x')").unwrap();
        c.query_drop("INSERT INTO project_tasks(project_id,title,done_at) SELECT id,'Ship it; calc & {{x}}',NOW() FROM projects LIMIT 1").unwrap();
        c.query_drop("INSERT INTO project_tasks(project_id,title) SELECT id,'Not done yet' FROM projects LIMIT 1").unwrap();
        c.query_drop("INSERT INTO reminders(title,due_date,detail) VALUES ('Pay rent','2026-11-21',''),('Later','2026-12-01',''),('Done','2026-11-21','')").unwrap();
        c.query_drop("UPDATE reminders SET state='done' WHERE title='Done'").unwrap();
        let found = poll(&mut c, &mut marks, &jobs, "2026-11-21").unwrap();
        let names: Vec<&str> = found.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, vec!["diary_saved", "task_done", "reminder_due"], "{found:?}");
        assert!(found[0].1.contains("The calm Saturday"));
        assert!(found[1].1.contains("Ship it; calc & {{x}}") && found[1].1.contains("Admin"));
        assert!(found[2].1.contains("Pay rent"));
        // nothing repeats, also with fresh watermarks after a restart (the reminder has a receipt)
        assert!(poll(&mut c, &mut marks, &jobs, "2026-11-21").unwrap().is_empty());
        let mut restarted = Marks::default();
        poll(&mut c, &mut restarted, &jobs, "2026-11-21").unwrap();
        assert!(poll(&mut c, &mut restarted, &jobs, "2026-11-21").unwrap().is_empty());
        // a task ticked later is noticed, and an unsubscribed event is not watched at all
        c.query_drop("UPDATE project_tasks SET done_at=NOW() WHERE title='Not done yet'").unwrap();
        let later = poll(&mut c, &mut marks, &jobs[1..2], "2026-11-21").unwrap();
        assert_eq!(later.len(), 1);
        assert!(later[0].1.contains("Not done yet"));
        assert!(marks.diary.is_none());
        c.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        let mut root = Conn::new(mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(user.as_str())).pass(Some(pass.as_str()))).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user)).unwrap();
        }
    }
}
