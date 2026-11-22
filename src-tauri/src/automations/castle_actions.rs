//! Fixed Castle moves. No payload/output chooses an action or becomes session input.
use super::*;
use mysql::prelude::Queryable;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub kind: String,
    pub action: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub due: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub allow_tools: bool,
}
pub fn spec(value: &Value) -> Result<Spec, String> {
    serde_json::from_value(value.clone()).map_err(|_| "Check the Castle action fields.".into())
}
pub fn validate(s: &Spec, trigger: &str) -> Result<(), String> {
    if s.kind != "castle" {
        return Err("Choose a Castle action.".into());
    }
    if ![
        "backup_memory",
        "export_markdown",
        "add_reminder",
        "add_task",
        "check_database",
        "wake_companion",
        "save_diary_prompt",
        "notify",
    ]
    .contains(&s.action.as_str())
    {
        return Err("Choose a supported Castle action.".into());
    }
    if [&s.title, &s.due, &s.project, &s.text, &s.folder]
        .iter()
        .any(|s| s.contains('\0'))
    {
        return Err("Action fields cannot contain null characters.".into());
    }
    if ["add_reminder", "add_task"].contains(&s.action.as_str())
        && (s.title.trim().is_empty() || s.title.chars().count() > 255)
    {
        return Err("Use a title of 1 to 255 characters.".into());
    }
    if !s.due.is_empty() && chrono::NaiveDate::parse_from_str(&s.due, "%Y-%m-%d").is_err() {
        return Err("Use a date like 2026-12-01.".into());
    }
    if s.action == "add_task" && (s.project.trim().is_empty() || s.project.chars().count() > 255) {
        return Err("Choose an active project.".into());
    }
    if s.text.len() > 32000 {
        return Err("Keep the notification under 32,000 bytes.".into());
    }
    if s.action == "notify" && s.text.trim().is_empty() {
        return Err("Write a notification.".into());
    }
    if ["backup_memory", "export_markdown"].contains(&s.action.as_str())
        && (!Path::new(&s.folder).is_absolute() || !Path::new(&s.folder).is_dir())
    {
        return Err("Choose an existing output folder.".into());
    }
    if ["wake_companion", "save_diary_prompt"].contains(&s.action.as_str())
        && !matches!(trigger, "schedule" | "manual" | "event")
    {
        return Err("Companion session actions only accept trusted schedules, manual runs or Castle events.".into());
    }
    // a session action's words are fixed: nothing an event or a message carried is ever put in them
    if ["wake_companion", "save_diary_prompt"].contains(&s.action.as_str())
        && [&s.title, &s.text].iter().any(|t| t.contains("{{payload"))
    {
        return Err("Companion session actions cannot use text from an event or message.".into());
    }
    if s.action == "save_diary_prompt" && !s.allow_tools {
        return Err("Confirm that this diary request lets your companion use its tools.".into());
    }
    Ok(())
}
fn output_parent(s: &Spec, memory: &Path) -> Result<PathBuf, String> {
    let parent =
        std::fs::canonicalize(&s.folder).map_err(|_| "The output folder is unavailable.")?;
    let memory = std::fs::canonicalize(memory).map_err(|_| "The memory folder is unavailable.")?;
    #[cfg(windows)]
    if parent
        .to_string_lossy()
        .to_ascii_uppercase()
        .starts_with(r"\\?\UNC\")
    {
        return Err("Choose a local private output folder.".into());
    }
    if parent.starts_with(memory) {
        return Err("Choose an output folder outside the active memory folder.".into());
    }
    Ok(parent)
}
pub fn preview(s: &Spec, conn: &mut Conn, memory: &Path) -> Result<String, String> {
    if ["backup_memory", "export_markdown"].contains(&s.action.as_str()) {
        output_parent(s, memory)?;
    }
    if s.action == "add_task" {
        active_project(conn, &s.project)?;
    }
    Ok(format!(
        "Test passed: {} is valid. No Castle changes or deliveries were made.",
        s.action
    ))
}
fn active_project(conn: &mut Conn, name: &str) -> Result<u64, String> {
    let id = crate::project_tree::need(conn, name)?;
    let state: Option<String> = conn
        .exec_first("SELECT shelf FROM projects WHERE id=?", (id,))
        .map_err(|_| "The project could not be checked.")?;
    if state.as_deref() != Some("active") {
        return Err("Choose a project on the active shelf.".into());
    }
    Ok(id)
}
pub fn database_action(s: &Spec, c: &mut Conn, memory: &Path, run: u64) -> Result<String, String> {
    match s.action.as_str() {
        "check_database" => {
            let value: Option<u8> = c
                .query_first("SELECT 1")
                .map_err(|_| "The database check failed.")?;
            if value != Some(1) {
                return Err("The database check failed.".into());
            }
            Ok("The memory database is reachable.".into())
        }
        "add_reminder" => {
            let mut flags = BTreeMap::from([
                ("title".into(), s.title.trim().into()),
                ("detail".into(), s.text.clone()),
            ]);
            if !s.due.is_empty() {
                flags.insert("due".into(), s.due.clone());
            }
            crate::door::run(
                c,
                &crate::door::Ask {
                    words: vec!["reminder".into(), "add".into()],
                    flags,
                },
                memory,
            )
        }
        "add_task" => {
            let project = active_project(c, &s.project)?;
            let id = crate::project_tree::task_add(
                c,
                project,
                &s.title,
                (!s.due.is_empty()).then_some(s.due.as_str()),
                false,
            )?;
            Ok(format!("Task #{id} added to {}: {}", s.project, s.title))
        }
        "backup_memory" | "export_markdown" => {
            if s.action == "export_markdown" {
                let paths: Vec<String> = c
                    .query("SELECT path FROM imported_files UNION SELECT path FROM memory_files UNION SELECT file FROM session_sections UNION SELECT source_file FROM projects WHERE source_file IS NOT NULL")
                    .map_err(|_| "Export paths could not be checked.")?;
                if paths.iter().any(|p| !export_path_ok(p)) {
                    return Err(
                        "The stored export contains an unsafe path; nothing was exported.".into(),
                    );
                }
            }
            let parent = output_parent(s, memory)?;
            let leaf = format!(
                "automation-{run}-{}",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            );
            let destination = parent.join(&leaf);
            if destination.exists() {
                return Err("The output folder already exists; nothing was overwritten.".into());
            }
            let to = parent.join(format!("{leaf}.partial"));
            std::fs::create_dir(&to).map_err(|_| {
                "A new output folder could not be created; nothing was overwritten."
            })?;
            c.query_drop("START TRANSACTION WITH CONSISTENT SNAPSHOT, READ ONLY")
                .map_err(|_| "The database snapshot could not start.")?;
            let result = if s.action == "backup_memory" {
                crate::door::dump(c, &to.join("memory.sql.partial"))
            } else {
                crate::door::run(
                    c,
                    &crate::door::Ask {
                        words: vec!["export".into()],
                        flags: BTreeMap::from([("to".into(), to.to_string_lossy().into_owned())]),
                    },
                    memory,
                )
            };
            if result.is_err() {
                let _ = c.query_drop("ROLLBACK");
                return result;
            }
            c.query_drop("COMMIT")
                .map_err(|_| "The database snapshot could not finish.")?;
            if s.action == "backup_memory" {
                std::fs::rename(to.join("memory.sql.partial"), to.join("memory.sql"))
                    .map_err(|_| "The backup could not be finalized.")?;
                std::fs::rename(&to, &destination)
                    .map_err(|_| "The backup folder could not be finalized.")?;
                Ok(format!("Local memory backup saved in {leaf}."))
            } else {
                std::fs::rename(&to, &destination)
                    .map_err(|_| "The export folder could not be finalized.")?;
                Ok(format!("Markdown export saved in {leaf}."))
            }
        }
        _ => Err("This action needs the running Castle session.".into()),
    }
}
pub(super) fn with_payload(mut s: Spec, payload: &Value) -> Result<Spec, String> {
    let context = BTreeMap::from([(
        "payload.text".into(),
        payload["text"].as_str().unwrap_or("").to_string(),
    )]);
    // Only these literal text fields accept data; never action, folder, project or date.
    s.title = actions::render(&s.title, &context)
        .chars()
        .take(255)
        .collect();
    s.text = actions::render(&s.text, &context);
    if s.text.len() > 32000 {
        return Err("Expanded message text is too long.".into());
    }
    Ok(s)
}
pub fn execute(app: &AppHandle, j: &Value, run: u64, payload: &Value) -> Result<String, String> {
    let mut s = spec(&j["action"])?;
    validate(&s, j["triggerKind"].as_str().unwrap_or("schedule"))?;
    s = with_payload(s, payload)?;
    validate(&s, j["triggerKind"].as_str().unwrap_or("schedule"))?;
    let store = app.state::<SettingsStore>();
    let memory = store
        .get()
        .memory_folder
        .ok_or("Choose a MemoryCore folder.")?;
    let limit = Duration::from_secs(j["policy"]["timeout_s"].as_u64().unwrap_or(120));
    let mut c = bounded_connection(Path::new(&memory), limit)?;
    if payload["cause"] == "test" {
        return preview(&s, &mut c, Path::new(&memory));
    }
    match s.action.as_str() {
        "notify" => Ok(s.text),
        "wake_companion" => {
            let host = &app.state::<crate::session::MainHost>().0;
            if host.is_running() {
                Ok("Your companion is already awake.".into())
            } else {
                crate::session::wake(app, &store, host, crate::session::Room::Main, 120, 40)?;
                Ok("Your companion was awakened.".into())
            }
        }
        "save_diary_prompt" => {
            if app
                .state::<crate::providers::Images>()
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|j| j.holds_session())
            {
                return Err(
                    "An image job is using the session; the diary request was not sent.".into(),
                );
            }
            let switching = app.state::<crate::providers::Control>();
            if switching.0.lock().unwrap().phase == "switching" {
                return Err("The provider is switching; the diary request was not sent.".into());
            }
            let reply = crate::bridges::send_and_await(
                app,
                &store,
                "save diary",
                Duration::from_secs(j["policy"]["timeout_s"].as_u64().unwrap_or(120)),
            )?;
            Ok(format!("Diary request returned:\n{reply}"))
        }
        _ => {
            let mutable = ["add_reminder", "add_task"].contains(&s.action.as_str());
            if mutable {
                c.query_drop("START TRANSACTION")
                    .map_err(|_| "The Castle change could not start.")?;
            }
            let result = database_action(&s, &mut c, Path::new(&memory), run);
            if mutable {
                if result.is_ok() {
                    c.query_drop("COMMIT").map_err(|_| {
                        "The change did not confirm completion; it was not retried."
                    })?;
                } else {
                    let _ = c.query_drop("ROLLBACK");
                }
            }
            result
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn action(name: &str) -> Spec {
        spec(&json!({"kind":"castle","action":name})).unwrap()
    }
    #[test]
    fn only_fixed_actions_are_accepted() {
        assert!(validate(&action("check_database"), "schedule").is_ok());
        assert!(validate(&action("run_shell"), "schedule").is_err());
    }
    #[test]
    fn session_actions_require_trusted_triggers_and_tool_ack() {
        let mut s = action("save_diary_prompt");
        assert!(validate(&s, "schedule").is_err());
        s.allow_tools = true;
        for t in ["message", "webhook", "file"] {
            assert!(validate(&s, t).is_err());
        }
        for t in ["schedule", "manual", "event"] {
            assert!(validate(&s, t).is_ok());
        }
    }
    #[test]
    fn invalid_dates_and_empty_titles_are_refused() {
        let mut s = action("add_reminder");
        assert!(validate(&s, "schedule").is_err());
        s.title = "Review".into();
        s.due = "2026-02-30".into();
        assert!(validate(&s, "schedule").is_err());
        s.due = "2026-12-01".into();
        assert!(validate(&s, "schedule").is_ok());
    }
}

fn export_path_ok(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.contains(['\\', ':', '\0', '*', '?', '<', '>', '|', '"'])
        && s.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && !p.ends_with('.')
                && !p.ends_with(' ')
                && !reserved_component(p)
        })
}

fn reserved_component(p: &str) -> bool {
    let base = p.split('.').next().unwrap_or("").to_ascii_uppercase();
    ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        || base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit()
            && base.as_bytes()[3] != b'0'
}
#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn export_paths_cannot_escape_or_name_devices() {
        for bad in [
            "../outside.md",
            "a/../../outside.md",
            "/absolute.md",
            "C:/file.md",
            "a\\file.md",
            "aux.md",
            "a/NUL.txt",
            "bad?file.md",
        ] {
            assert!(!export_path_ok(bad), "{bad}");
        }
        assert!(export_path_ok("main/main-memory.md"));
    }
}
#[cfg(test)]
mod live_tests {
    use super::*;
    /// Explicit opt-in local DB. Exact generated DB/user and owned temporary artifacts only.
    #[test]
    #[ignore]
    fn live_fixed_actions_and_snapshot() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("CASTLE_TEST_DB=port:admin:password");
        let mut bits = spec.splitn(3, ':');
        let port = bits.next().unwrap().parse().unwrap();
        let admin = bits.next().unwrap();
        let password = bits.next().unwrap_or("");
        let access = crate::memdb::provision(
            port,
            admin,
            password,
            &format!("Phase3 Test {}", std::process::id()),
        )
        .unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        let base =
            std::env::temp_dir().join(format!("castle-phase3-fixture-{}", std::process::id()));
        assert!(
            !base.exists(),
            "owned fixture exists; inspect it before retrying"
        );
        std::fs::create_dir(&base).unwrap();
        let memory = base.join("memory");
        let out = base.join("output");
        std::fs::create_dir(&memory).unwrap();
        std::fs::create_dir(&out).unwrap();
        let mut s = super::tests::action("check_database");
        assert!(database_action(&s, &mut c, &memory, 1)
            .unwrap()
            .contains("reachable"));
        s = super::tests::action("add_reminder");
        s.title = "Literal ; calc reminder".into();
        s.due = "2026-12-01".into();
        preview(&s, &mut c, &memory).unwrap();
        let before: Option<u64> = c.query_first("SELECT COUNT(*) FROM reminders").unwrap();
        assert_eq!(before, Some(0));
        database_action(&s, &mut c, &memory, 2).unwrap();
        let title: Option<String> = c.query_first("SELECT title FROM reminders").unwrap();
        assert_eq!(title.as_deref(), Some("Literal ; calc reminder"));
        c.query_drop("INSERT INTO projects (name,summary,body) VALUES ('Fixture project','Owned test','Fixture notes')").unwrap();
        s = super::tests::action("add_task");
        s.project = "Fixture project".into();
        s.title = "Task <script> is text".into();
        database_action(&s, &mut c, &memory, 3).unwrap();
        let tasks: Option<u64> = c.query_first("SELECT COUNT(*) FROM project_tasks").unwrap();
        assert_eq!(tasks, Some(1));
        c.query_drop("UPDATE projects SET shelf='paused'").unwrap();
        assert!(preview(&s, &mut c, &memory).is_err());
        c.query_drop("UPDATE projects SET shelf='active'").unwrap();
        s = super::tests::action("backup_memory");
        s.folder = memory.to_string_lossy().into_owned();
        assert!(preview(&s, &mut c, &memory).is_err());
        s.folder = out.to_string_lossy().into_owned();
        database_action(&s, &mut c, &memory, 4).unwrap();
        let dump = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().path().join("memory.sql"))
            .find(|p| p.is_file())
            .unwrap();
        let sql = std::fs::read_to_string(dump).unwrap();
        assert!(sql.contains("CREATE TABLE") && sql.contains("Literal ; calc reminder"));
        s.action = "export_markdown".into();
        database_action(&s, &mut c, &memory, 5).unwrap();
        let count = std::fs::read_dir(&out).unwrap().count();
        assert_eq!(count, 2);
        c.query_drop(
            "INSERT INTO memory_files (path,body) VALUES ('../outside.md','malicious fixture')",
        )
        .unwrap();
        assert!(database_action(&s, &mut c, &memory, 6).is_err());
        assert!(!base.join("outside.md").exists());
        drop(c);
        let opts = mysql::OptsBuilder::new()
            .ip_or_hostname(Some("127.0.0.1"))
            .tcp_port(port)
            .user(Some(admin))
            .pass(Some(password));
        let mut root = Conn::new(opts).unwrap();
        root.query_drop(format!("DROP DATABASE `{}`", access.name))
            .unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user))
                .unwrap();
        }
        // Every artifact is inside the exact new fixture directory above, never an existing folder.
        let actual = std::fs::canonicalize(&base).unwrap();
        assert!(actual.starts_with(std::fs::canonicalize(std::env::temp_dir()).unwrap()));
        assert_eq!(actual.file_name(), base.file_name());
        std::fs::remove_dir_all(actual).unwrap();
    }
}

pub(super) fn bounded_connection(memory: &Path, limit: Duration) -> Result<Conn, String> {
    let text = std::fs::read_to_string(memory.join(".env"))
        .map_err(|_| "The memory database configuration is unavailable.")?;
    let a = crate::memdb::access_from_env(&text)
        .ok_or("The memory database configuration is incomplete.")?;
    let opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(crate::memdb::HOST))
        .tcp_port(a.port)
        .db_name(Some(a.name))
        .user(Some(a.user))
        .pass(Some(a.password))
        .prefer_socket(false)
        .tcp_connect_timeout(Some(limit.min(Duration::from_secs(5))))
        .read_timeout(Some(limit))
        .write_timeout(Some(limit));
    Conn::new(opts).map_err(|_| "The database could not be reached within the time limit.".into())
}
