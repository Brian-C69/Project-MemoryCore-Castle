//! Approved bridge messages are data, never commands or real-session input.
use super::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
static PROBLEM: Mutex<Option<String>> = Mutex::new(None);
pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}
static INTAKE: Mutex<()> = Mutex::new(());
static MATCHING: AtomicBool = AtomicBool::new(false);
const MAX_TEXT: usize = 16_384;
const MATCH_LIMIT: Duration = Duration::from_millis(50);
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub platform: String,
    #[serde(rename = "match")]
    pub mode: String,
    pub text: String,
    #[serde(default)]
    pub case: bool,
    #[serde(default)]
    pub also_chat: bool,
}
fn compile(s: &Message) -> Result<regex::Regex, String> {
    regex::RegexBuilder::new(&s.text)
        .case_insensitive(!s.case)
        .size_limit(128 * 1024)
        .dfa_size_limit(128 * 1024)
        .nest_limit(16)
        .build()
        .map_err(|_| {
            "Use a valid, small regular expression without lookaround or backreferences.".into()
        })
}
pub fn spec(v: &Value) -> Result<Message, String> {
    let s: Message =
        serde_json::from_value(v.clone()).map_err(|_| "Check the message trigger fields.")?;
    if !["telegram", "discord", "whatsapp", "any"].contains(&s.platform.as_str())
        || !["contains", "starts_with", "regex"].contains(&s.mode.as_str())
        || s.text.trim().is_empty()
        || s.text.len() > 256
        || s.text.contains('\0')
    {
        return Err("Choose a platform, match mode and pattern of 1–256 bytes.".into());
    }
    if s.mode == "regex" {
        compile(&s)?;
    }
    Ok(s)
}
fn matches(s: &Message, text: &str) -> Result<bool, String> {
    if text.len() > MAX_TEXT {
        return Err("Message is over the 16 KB matching limit.".into());
    }
    if s.mode == "regex" {
        return compile(s).map(|r| r.is_match(text));
    }
    let (text, pattern) = if s.case {
        (text.to_string(), s.text.clone())
    } else {
        (text.to_lowercase(), s.text.to_lowercase())
    };
    Ok(if s.mode == "contains" {
        text.contains(&pattern)
    } else {
        text.starts_with(&pattern)
    })
}
/// One bounded worker globally. A timed-out match cannot accumulate abandoned threads.
fn matching(jobs: Vec<Value>, platform: String, text: String) -> Result<Vec<Value>, String> {
    matching_work(move || {
        let mut selected = Vec::new();
        for j in jobs {
            let s = spec(&j["trigger"])?;
            if (s.platform == "any" || s.platform == platform) && matches(&s, &text)? {
                selected.push(j);
            }
        }
        Ok(selected)
    })
}
fn matching_work(
    work: impl FnOnce() -> Result<Vec<Value>, String> + Send + 'static,
) -> Result<Vec<Value>, String> {
    if MATCHING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("Message matching is busy.".into());
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                MATCHING.store(false, Ordering::Release);
            }
        }
        let _clear = Clear;
        let result = work();
        let _ = tx.send(result);
    });
    rx.recv_timeout(MATCH_LIMIT)
        .map_err(|_| "Message matching exceeded its time limit.".to_string())?
}
#[tauri::command]
pub async fn automation_message_match(trigger: Value, text: String) -> Result<bool, String> {
    spec(&trigger)?;
    if text.len() > MAX_TEXT {
        return Err("Use sample text of at most 16 KB.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let platform = trigger["platform"].as_str().unwrap_or("any").to_string();
        matching(vec![json!({"trigger":trigger})], platform, text).map(|jobs| !jobs.is_empty())
    })
    .await
    .map_err(|_| "The match preview could not finish.".to_string())?
}
pub fn approved(s: &crate::settings::Settings, platform: &str, chat: &str) -> bool {
    crate::bridges::notification_target(s, platform).is_ok_and(|id| id == chat)
}
pub(super) fn event_key(platform: &str, chat: &str, id: &str) -> String {
    let mut hash = Sha256::new();
    for part in [platform, chat, id] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
/// Return whether normal companion chat may continue. Errors fail closed while automations
/// are on: never route a potentially matching untrusted message into the real session.
pub fn incoming(app: &AppHandle, platform: &str, chat: &str, event: &str, text: &str) -> bool {
    let store = app.state::<SettingsStore>();
    if !approved(&store.get(), platform, chat) {
        return false;
    }
    if !allowed(&store.get()) {
        return true;
    }
    match process(app, platform, chat, event, text) {
        Ok(chat) => {
            *PROBLEM.lock().unwrap() = None;
            chat
        }
        Err(error) => {
            *PROBLEM.lock().unwrap() = Some(format!("Message intake stopped: {error} Normal chat was suppressed; this event was not retried."));
            let _ = app.emit("castle://automations", json!({"messageProblem":true}));
            false
        }
    }
}
fn process(
    app: &AppHandle,
    platform: &str,
    chat: &str,
    event: &str,
    text: &str,
) -> Result<bool, String> {
    let _guard = INTAKE.lock().map_err(|_| "Message intake unavailable.")?;
    let store = app.state::<SettingsStore>();
    if !approved(&store.get(), platform, chat) {
        return Ok(false);
    }
    if !allowed(&store.get()) {
        return Ok(true);
    }
    let mut c = connection(&store)?;
    let (also_chat, selected) = route(&mut c, platform, chat, event, text)?;
    let payload = json!({"cause":"message","platform":platform,"text":text,"time":schedule::stamp(chrono::Local::now())});
    let mut failed = false;
    for j in selected {
        if !approved(&store.get(), platform, chat) || !allowed(&store.get()) {
            return Ok(false);
        }
        if runner::launch_payload(
            app,
            j["id"].as_u64().ok_or("Missing job id.")?,
            "message",
            payload.clone(),
            Some(&j),
        )
        .is_err()
        {
            failed = true;
        }
    }
    if failed {
        return Err("A message job could not start; the event was not replayed.".into());
    }
    Ok(also_chat)
}
/// Reserve one incoming event before any action starts. No raw message is persisted here.
fn route(
    c: &mut Conn,
    platform: &str,
    chat: &str,
    event: &str,
    text: &str,
) -> Result<(bool, Vec<Value>), String> {
    let jobs: Vec<_> = store::list(c)?
        .into_iter()
        .filter(|j| j["state"] == "active" && j["enabled"] == 1 && j["triggerKind"] == "message")
        .collect();
    if jobs.is_empty() {
        return Ok((true, vec![]));
    }
    if jobs.len() > 64 || event.is_empty() || event.len() > 256 || text.len() > MAX_TEXT {
        return Err("Message trigger limits exceeded.".into());
    }
    let key = event_key(platform, chat, event);
    let seen: Option<u8> = c
        .exec_first(
            "SELECT also_chat FROM automation_messages WHERE event_key=?",
            (&key,),
        )
        .map_err(|_| "Message receipts unavailable.")?;
    // A duplicate must never repeat a job OR a real-session chat reply.
    if seen.is_some() {
        return Ok((false, vec![]));
    }
    let selected = matching(jobs, platform.into(), text.into())?;
    if selected.is_empty() {
        return Ok((true, vec![]));
    }
    let also_chat = selected
        .iter()
        .all(|j| spec(&j["trigger"]).is_ok_and(|s| s.also_chat));
    c.exec_drop(
        "INSERT INTO automation_messages(event_key,also_chat) VALUES (?,?)",
        (&key, also_chat),
    )
    .map_err(|_| "Message could not be reserved.")?;
    Ok((also_chat, selected))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(mode: &str, case: bool) -> Value {
        json!({"platform":"any","match":mode,"text":"invoice","case":case})
    }
    #[test]
    fn modes_case_unicode_and_default_no_chat() {
        let mut s = spec(&value("contains", false)).unwrap();
        assert!(!s.also_chat);
        assert!(matches(&s, "An INVOICE ; calc").unwrap());
        s.case = true;
        assert!(!matches(&s, "INVOICE").unwrap());
        s.mode = "starts_with".into();
        assert!(!matches(&s, "An invoice").unwrap());
        assert!(matches(&s, "invoice 3").unwrap());
        s.text = "CAFÉ".into();
        s.case = false;
        assert!(matches(&s, "café").unwrap());
        s.mode = "regex".into();
        s.text = "^invoice [0-9]+$".into();
        assert!(matches(&s, "INVOICE 23").unwrap());
        assert!(!matches(&s, "invoice xx").unwrap());
    }
    #[test]
    fn pattern_input_and_complexity_limits() {
        for v in [
            json!({"platform":"else","match":"contains","text":"x"}),
            json!({"platform":"any","match":"regex","text":"("}),
            json!({"platform":"any","match":"regex","text":"(a)\\1"}),
            json!({"platform":"any","match":"contains","text":""}),
            json!({"platform":"any","match":"contains","text":"x","chat_id":"other"}),
            json!({"platform":"any","match":"contains","text":"x".repeat(257)}),
        ] {
            assert!(spec(&v).is_err());
        }
        assert!(matches(
            &spec(&value("contains", false)).unwrap(),
            &"x".repeat(MAX_TEXT + 1)
        )
        .is_err());
        let mut v = value("regex", false);
        v["text"] = json!("(a+)+$");
        let s = spec(&v).unwrap();
        assert!(!matches(&s, &format!("{}!", "a".repeat(MAX_TEXT - 1))).unwrap());
    }
    #[test]
    fn all_platforms_require_current_room_bridge_and_exact_chat() {
        let mut s = crate::settings::Settings::default();
        for platform in ["telegram", "discord", "whatsapp"] {
            assert!(!approved(&s, platform, "123"));
        }
        s.rooms.insert("messaging".into(), true);
        s.telegram_bridge_on = true;
        s.discord_bridge_on = true;
        s.whatsapp_bridge_on = true;
        s.telegram_bridge_chat_id = Some("123".into());
        s.discord_bridge_channel_id = Some("123".into());
        s.whatsapp_bridge_chat_id = Some("123".into());
        for platform in ["telegram", "discord", "whatsapp"] {
            assert!(approved(&s, platform, "123"));
            assert!(!approved(&s, platform, "124"));
        }
        s.telegram_bridge_on = false;
        assert!(!approved(&s, "telegram", "123"));
    }
    #[test]
    fn matching_deadline_is_enforced_without_worker_accumulation() {
        let (release, blocked) = std::sync::mpsc::channel();
        let started = std::time::Instant::now();
        assert!(matching_work(move || {
            let _ = blocked.recv();
            Ok(vec![])
        })
        .is_err());
        assert!(started.elapsed() >= MATCH_LIMIT);
        assert!(MATCHING.load(Ordering::Acquire));
        assert!(matching(vec![], "telegram".into(), "invoice".into()).is_err());
        release.send(()).unwrap();
        for _ in 0..100 {
            if !MATCHING.load(Ordering::Acquire) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(!MATCHING.load(Ordering::Acquire));
    }
    #[test]
    fn event_keys_do_not_mix_chats_or_platforms() {
        assert_ne!(
            event_key("telegram", "12", "3"),
            event_key("telegram", "1", "23")
        );
        assert_ne!(
            event_key("telegram", "12", "3"),
            event_key("discord", "12", "3")
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn message_jobs_refuse_tool_session_and_unknown_triggers() {
        let mut j = json!({"name":"Inbox", "triggerKind":"message", "trigger":{"platform":"whatsapp","match":"contains","text":"invoice"},"action":{"kind":"castle","action":"add_task","project":"Admin","title":"{{payload.text}}"},"delivery":[],"policy":{}});
        assert!(validate(&j).is_ok());
        for action in ["save_diary_prompt", "wake_companion"] {
            j["action"] = json!({"kind":"castle","action":action,"allow_tools":true});
            assert!(validate(&j).is_err());
        }
        j["action"] = json!({"mode":"session","prompt":"{{payload.text}}"});
        assert!(validate(&j).is_err());
        j["action"] = json!({"mode":"think","prompt":"{{payload.text}}"});
        assert!(validate(&j).is_ok());
        // a message trigger's fields are not a web request trigger's
        j["triggerKind"] = json!("webhook");
        assert!(validate(&j).is_err());
    }
    #[test]
    fn file_and_web_request_jobs_may_not_open_tool_sessions_or_change_who_runs() {
        for (kind, trigger) in [("file", json!({"folder":"C:/drop","glob":"*.pdf","event":"created","settle_s":5})), ("webhook", json!({}))] {
            let mut j = json!({"name":"Intake","triggerKind":kind,"trigger":trigger,"action":{"kind":"castle","action":"add_task","project":"Admin","title":"{{payload.text}}"},"delivery":[],"policy":{}});
            #[cfg(windows)]
            assert!(validate(&j).is_ok(), "{kind}");
            for action in ["save_diary_prompt", "wake_companion"] {
                j["action"] = json!({"kind":"castle","action":action,"allow_tools":true});
                assert!(validate(&j).is_err(), "{kind} {action}");
            }
            j["action"] = json!({"mode":"session","prompt":"{{payload.text}}"});
            assert!(validate(&j).is_err(), "{kind}");
            j["action"] = json!({"mode":"think","prompt":"Summarise {{payload.text}}"});
            #[cfg(windows)]
            assert!(validate(&j).is_ok(), "{kind}");
        }
        let bad = json!({"name":"x","triggerKind":"webhook","trigger":{"port":1},"action":{"mode":"think","prompt":"x"},"delivery":[],"policy":{}});
        assert!(validate(&bad).is_err());
        let unknown = json!({"name":"x","triggerKind":"carrier-pigeon","trigger":{},"action":{"mode":"think","prompt":"x"},"delivery":[],"policy":{}});
        assert!(validate(&unknown).is_err());
    }
    #[test]
    fn file_and_web_request_jobs_are_never_due_as_schedules() {
        for kind in ["file", "webhook"] {
            assert!(!runner::due(&json!({"enabled":1,"state":"active","triggerKind":kind,"nextRun":"2026-01-01 00:00:00"}), "2026-12-01 00:00:00"));
        }
    }
    #[test]
    fn payload_expands_only_literal_fields_once_and_bounds_title() {
        let s=castle_actions::spec(&json!({"kind":"castle","action":"add_task","title":"{{payload.text}}","text":"Details: {{payload.text}}","project":"{{payload.text}}","due":"","folder":"{{payload.text}}"})).unwrap();
        let payload = json!({"text":"invoice ; calc & {{projects}} <script>"});
        let result = castle_actions::with_payload(s.clone(), &payload).unwrap();
        assert_eq!(result.title, payload["text"].as_str().unwrap());
        assert_eq!(result.project, "{{payload.text}}");
        assert_eq!(result.folder, "{{payload.text}}");
        assert!(result.text.contains("{{projects}}"));
        assert_eq!(
            castle_actions::with_payload(s, &json!({"text":"\u{732b}".repeat(300)}))
                .unwrap()
                .title
                .chars()
                .count(),
            255
        );
    }
    #[test]
    fn message_jobs_are_never_due_as_schedules() {
        assert!(!runner::due(
            &json!({"enabled":1,"state":"active","triggerKind":"message","nextRun":"2026-01-01 00:00:00"}),
            "2026-12-01 00:00:00"
        ));
    }
    /// Opt-in isolated local DB. No live bridge, chat session or outbound network request.
    #[test]
    #[ignore]
    fn live_invoice_routing_task_dedup_and_persistence() {
        let config = std::env::var("CASTLE_TEST_DB").expect("CASTLE_TEST_DB=port:admin:password");
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let user = parts.next().unwrap();
        let pass = parts.next().unwrap_or("");
        let access = crate::memdb::provision(
            port,
            user,
            pass,
            &format!("Message Test {}", std::process::id()),
        )
        .unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        c.query_drop("INSERT INTO projects(name,shelf) VALUES ('Admin','active')")
            .unwrap();
        let mut j = json!({"name":"Invoice","enabled":true,"triggerKind":"message","trigger":{"platform":"whatsapp","match":"contains","text":"invoice","also_chat":false},"action":{"kind":"castle","action":"add_task","project":"Admin","title":"{{payload.text}}"},"delivery":[],"policy":{}});
        let id = store::save(&mut c, &j).unwrap();
        assert_eq!(store::get(&mut c, id).unwrap()["enabled"], 0);
        let state = Running::default();
        let interrupted = store::begin(&mut c, id, "manual").unwrap();
        runner::recover_once(&mut c, &state).unwrap();
        assert_eq!(store::runs(&mut c, id).unwrap()[0]["status"], "failed");
        let current = store::begin(&mut c, id, "message").unwrap();
        runner::recover_once(&mut c, &state).unwrap();
        assert_eq!(store::runs(&mut c, id).unwrap()[0]["status"], "running");
        c.exec_drop(
            "DELETE FROM automation_runs WHERE id IN (?,?)",
            (interrupted, current),
        )
        .unwrap();
        j["id"] = json!(id);
        store::save(&mut c, &j).unwrap();
        assert!(store::get(&mut c, id).unwrap()["nextRun"].is_null());
        assert!(route(&mut c, "telegram", "123", "a", "invoice")
            .unwrap()
            .1
            .is_empty());
        assert!(route(&mut c, "whatsapp", "123", "b", "hello").unwrap().0);
        let text = "invoice ; calc & {{work_log}} <script>";
        let (chat, jobs) = route(&mut c, "whatsapp", "123", "c", text).unwrap();
        assert!(!chat);
        assert_eq!(jobs.len(), 1);
        let action = castle_actions::with_payload(
            castle_actions::spec(&jobs[0]["action"]).unwrap(),
            &json!({"text":text}),
        )
        .unwrap();
        castle_actions::database_action(&action, &mut c, std::path::Path::new("."), 1).unwrap();
        let title: Option<String> = c.query_first("SELECT title FROM project_tasks").unwrap();
        assert_eq!(title.as_deref(), Some(text));
        drop(c);
        let mut c = crate::memdb::open(&access).unwrap();
        let duplicate = route(&mut c, "whatsapp", "123", "c", text).unwrap();
        assert!(!duplicate.0);
        assert!(duplicate.1.is_empty());
        let count: Option<u64> = c.query_first("SELECT COUNT(*) FROM project_tasks").unwrap();
        assert_eq!(count, Some(1));
        j["trigger"]["also_chat"] = json!(true);
        store::save(&mut c, &j).unwrap();
        assert!(route(&mut c, "whatsapp", "123", "d", "invoice").unwrap().0);
        let mut second = j.clone();
        second.as_object_mut().unwrap().remove("id");
        second["name"] = json!("Invoice no chat");
        second["trigger"]["also_chat"] = json!(false);
        let id2 = store::save(&mut c, &second).unwrap();
        second["id"] = json!(id2);
        store::save(&mut c, &second).unwrap();
        assert!(!route(&mut c, "whatsapp", "123", "e", "invoice").unwrap().0);
        second["enabled"] = json!(false);
        store::save(&mut c, &second).unwrap();
        assert!(route(&mut c, "whatsapp", "123", "f", "invoice").unwrap().0);
        c.exec_drop(
            "UPDATE automation_jobs SET state='proposed',enabled=1 WHERE id=?",
            (id,),
        )
        .unwrap();
        assert!(route(&mut c, "whatsapp", "123", "g", "invoice")
            .unwrap()
            .1
            .is_empty());
        c.query_drop("DROP TABLE automation_messages").unwrap();
        c.query_drop("DELETE FROM schema_migrations WHERE name='019_automation_messages'")
            .unwrap();
        assert_eq!(
            crate::memdb::migrate(&mut c).unwrap(),
            vec!["019_automation_messages"]
        );
        c.query_drop(format!("DROP DATABASE `{}`", access.name))
            .unwrap();
        let mut root = Conn::new(
            mysql::OptsBuilder::new()
                .ip_or_hostname(Some("127.0.0.1"))
                .tcp_port(port)
                .user(Some(user))
                .pass(Some(pass)),
        )
        .unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user))
                .unwrap();
        }
    }
}
