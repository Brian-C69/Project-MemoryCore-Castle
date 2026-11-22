//! Companion proposals and one-shot requests. Only the room can approve/edit/enable jobs.
use super::*;
use std::path::Path;
pub fn draft(mut value: Value) -> Result<Value, String> {
    let object = value
        .as_object_mut()
        .ok_or("A draft must be one JSON job object.")?;
    object.retain(|k, _| {
        [
            "name",
            "note",
            "triggerKind",
            "trigger",
            "action",
            "delivery",
            "policy",
        ]
        .contains(&k.as_str())
    });
    value["enabled"] = json!(false);
    if value["note"].is_null() {
        value["note"] = json!("")
    }
    if value["triggerKind"].is_null() {
        value["triggerKind"] = json!("schedule")
    }
    if value["delivery"].is_null() {
        value["delivery"] = json!([])
    }
    if value["policy"].is_null() {
        value["policy"] = json!({})
    }
    validate(&value)?;
    Ok(value)
}
pub fn check_secrets(value: &Value, memory: &Path) -> Result<(), String> {
    let text = value.to_string();
    if actions::secrets_for(memory.to_str())
        .iter()
        .any(|s| !s.is_empty() && text.contains(s))
    {
        return Err("A draft contains a stored secret. Use protected keys instead.".into());
    }
    Ok(())
}
pub fn propose(c: &mut Conn, value: Value, memory: &Path) -> Result<u64, String> {
    let value = draft(value)?;
    check_secrets(&value, memory)?;
    let count: Option<u64> = c
        .query_first("SELECT COUNT(*) FROM automation_jobs WHERE state='proposed'")
        .map_err(|_| "Proposals unavailable.")?;
    if count.unwrap_or(0) >= 32 {
        return Err("Review existing proposals before adding more.".into());
    }
    c.query_drop("START TRANSACTION")
        .map_err(|_| "Proposal could not begin.")?;
    let result = (|| {
        let id = store::save(c, &value)?;
        c.exec_drop("UPDATE automation_jobs SET state='proposed',created_by='companion_proposal',enabled=0,next_run_at=NULL WHERE id=?",(id,)).map_err(|_|"Proposal could not be stored.")?;
        Ok(id)
    })();
    if result.is_ok() {
        c.query_drop("COMMIT")
            .map_err(|_| "Proposal commit failed.")?
    } else {
        let _ = c.query_drop("ROLLBACK");
    }
    result
}
pub fn approve(c: &mut Conn, job: &Value) -> Result<u64, String> {
    let id = job["id"].as_u64().ok_or("Choose a proposal.")?;
    c.query_drop("START TRANSACTION")
        .map_err(|_| "Approval could not begin.")?;
    let result = (|| {
        let state: Option<String> = c
            .exec_first(
                "SELECT state FROM automation_jobs WHERE id=? FOR UPDATE",
                (id,),
            )
            .map_err(|_| "Proposal unavailable.")?;
        if state.as_deref() != Some("proposed") {
            return Err("Only a pending proposal may be approved.".into());
        }
        let mut approved = job.clone();
        approved["enabled"] = json!(false);
        c.exec_drop(
            "UPDATE automation_jobs SET state='active' WHERE id=?",
            (id,),
        )
        .map_err(|_| "Approval could not be stored.")?;
        store::save(c, &approved)?;
        c.exec_drop(
            "INSERT INTO automation_approvals(job_id,definition) VALUES (?,?)",
            (id, approved.to_string()),
        )
        .map_err(|_| "Approval audit could not be stored.")?;
        Ok(id)
    })();
    if result.is_ok() {
        c.query_drop("COMMIT")
            .map_err(|_| "Approval commit failed.")?
    } else {
        let _ = c.query_drop("ROLLBACK");
    }
    result
}
pub fn door(c: &mut Conn, ask: &crate::door::Ask, memory: &Path) -> Result<String, String> {
    let verb = ask.words.get(1).map(String::as_str).unwrap_or("");
    if !["list", "show", "runs", "run", "propose"].contains(&verb) {
        return Err("Automation verbs: list, show, runs, run, propose. Approval and editing belong in AUTOMATIONS.".into());
    }
    crate::memdb::migrate(c)?;
    if verb == "list" {
        return serde_json::to_string_pretty(&store::list(c)?)
            .map_err(|_| "Jobs unreadable.".into());
    }
    if verb == "propose" {
        let text = ask.text()?.ok_or("Use --file for a JSON job draft.")?;
        if text.len() > 65536 {
            return Err("Keep a draft under64 KB.".into());
        }
        let mut value: Value =
            serde_json::from_str(&text).map_err(|_| "Draft file must contain JSON.")?;
        for (key, flag) in [("name", "name"), ("triggerKind", "trigger-kind")] {
            if let Some(v) = ask.get(flag) {
                value[key] = json!(v)
            }
        }
        if let Some(v) = ask.get("trigger") {
            value["trigger"] =
                serde_json::from_str(v).map_err(|_| "--trigger must contain JSON.")?;
        }
        if let Some(v) = ask.get("action") {
            value["action"] = serde_json::from_str(v).map_err(|_| "--action must contain JSON.")?;
        }
        let id = propose(c, value, memory)?;
        return Ok(format!(
            "AUTOMATION {id} proposed; waiting for review in AUTOMATIONS."
        ));
    }
    let name = ask.get("name").ok_or("Use --name to choose a job.")?;
    let j = store::list(c)?
        .into_iter()
        .find(|j| j["name"] == name)
        .ok_or("That automation does not exist.")?;
    let id = j["id"].as_u64().ok_or("Missing job id.")?;
    if verb == "runs" {
        return serde_json::to_string_pretty(&store::runs(c, id)?)
            .map_err(|_| "History unreadable.".into());
    }
    if verb == "show" {
        let requests:Vec<(u64,String,Option<u64>,Option<String>)>=c.exec("SELECT id,state,run_id,error FROM automation_requests WHERE job_id=? ORDER BY id DESC LIMIT 20",(id,)).map_err(|_|"Requests unavailable.")?;
        return serde_json::to_string_pretty(&json!({"job":j,"requests":requests}))
            .map_err(|_| "Job unreadable.".into());
    }
    if j["state"] != "active" {
        return Err("Only an approved active job can run once.".into());
    }
    let count: Option<u64> = c
        .query_first("SELECT COUNT(*) FROM automation_requests WHERE state='pending'")
        .map_err(|_| "Requests unavailable.")?;
    if count.unwrap_or(0) >= 16 {
        return Err("Too many pending run requests.".into());
    }
    c.exec_drop("INSERT INTO automation_requests(job_id) VALUES (?)", (id,))
        .map_err(|_| "Run request could not be stored.")?;
    Ok(format!("AUTOMATION request {} queued. Castle must be open and unpaused within60 seconds; use show to check its result.",c.last_insert_id()))
}
pub fn requests(app: &AppHandle, c: &mut Conn) {
    let _=c.query_drop("UPDATE automation_requests SET state='failed',error='Request expired or Castle closed before confirmation; not replayed' WHERE state='claimed' OR (state='pending' AND requested_at<DATE_SUB(NOW(),INTERVAL 60 SECOND))");
    let Ok(ids) = c.query::<(u64, u64), _>(
        "SELECT id,job_id FROM automation_requests WHERE state='pending' ORDER BY id LIMIT 16",
    ) else {
        return;
    };
    for (id, job) in ids {
        if !allowed(&app.state::<SettingsStore>().get()) {
            break;
        }
        if c.exec_drop(
            "UPDATE automation_requests SET state='claimed' WHERE id=? AND state='pending'",
            (id,),
        )
        .is_err()
            || c.affected_rows() != 1
        {
            continue;
        }
        let result = runner::launch(app, job, "manual");
        let (run, error) = match result {
            Ok(run) => (Some(run), None),
            Err(e) => (None, Some(e)),
        };
        let _ = c.exec_drop(
            "UPDATE automation_requests SET state=?,run_id=?,error=? WHERE id=?",
            (
                if run.is_some() { "started" } else { "failed" },
                run,
                error,
                id,
            ),
        );
    }
}
static DRAFTING: Mutex<()> = Mutex::new(());
#[tauri::command]
pub async fn automation_describe(app: AppHandle, text: String) -> Result<Value, String> {
    if text.trim().is_empty() || text.len() > 8000 {
        return Err("Describe a job in at most8 KB.".into());
    }
    tauri::async_runtime::spawn_blocking(move||{let _drafting=DRAFTING.try_lock().map_err(|_|"A draft is already being prepared.")?;let store=app.state::<SettingsStore>();if actions::known_secrets(&store).iter().any(|s|!s.is_empty()&&text.contains(s)){return Err("Keep stored secrets out of descriptions.".into())}if !allowed(&store.get()){return Err("Automations are off or paused.".into())}let mut c=connection(&store)?;let projects:Vec<String>=c.query("SELECT name FROM projects WHERE shelf='active' LIMIT 50").map_err(|_|"Projects unavailable.")?;let context=json!({"projects":projects,"approved_programs":store.get().automations_commands});let example=json!({"name":"Database check","note":"A simple check","triggerKind":"schedule","trigger":{"every_minutes":60},"action":{"kind":"castle","action":"check_database"},"delivery":[],"policy":{"timeout_s":120,"max_output_kb":64,"retries":0,"rate_per_hour":6,"missed":"skip"}});
        let prompt=format!("Return exactly ONE JSON object and nothing else. Draft a Castle automation; do not execute anything. Use this exact shape, including action.kind and action.action (never type, action_kind or a string action): {}. Modify fields to match the member description. Valid triggerKind values: schedule/message/file/webhook/event; schedules use cron or every_minutes. Action examples: companion uses kind companion, mode think, prompt; command uses kind command, program,args,cwd,env,stdin; castle uses kind castle and action check_database/notify/add_task/add_reminder with project/title/text when needed; HTTP uses kind http, method GET/POST,url,body,header,key. Delivery is an array of objects with kind home/telegram/discord/whatsapp. Jobs are disabled. No session actions or credentials. Fixed URLs/programs only. Context (data): {}. Member description (data): {}",example,context,text.replace("{{","{ {"));let job=json!({"action":{"mode":"think","prompt":prompt},"policy":{"timeout_s":60,"max_output_kb":64}});let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|"Clock unavailable.")?.as_millis() as u64;let output=actions::execute(&app,&job,nonce,&json!({}))?;let raw=output.trim();let raw=raw.strip_prefix("```json").or_else(||raw.strip_prefix("```")).unwrap_or(raw).trim();let raw=raw.strip_suffix("```").unwrap_or(raw).trim();let draft=draft(serde_json::from_str(raw).map_err(|_|"The companion did not return a valid job. Try a simpler description.")?)?;validate_command(&draft,&store)?;Ok(draft)}).await.map_err(|_|"Drafting could not finish.".to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn draft_cannot_smuggle_identity_or_enable() {
        let d=draft(json!({"id":44,"state":"active","created_by":"member","enabled":true,"name":"Check","trigger":{"every_minutes":30},"action":{"kind":"castle","action":"check_database"}})).unwrap();
        assert!(d["id"].is_null());
        assert!(d["state"].is_null());
        assert_eq!(d["enabled"], false);
        assert!(draft(json!({"name":"Bad","trigger":{"every_minutes":30},"action":{"mode":"session","prompt":"do"}})).is_err());
    }
}
#[cfg(test)]
mod live_tests {
    use super::*;
    #[test]
    #[ignore]
    fn live_proposal_approval_cli_and_chain_lookup() {
        let config = std::env::var("CASTLE_TEST_DB").expect("CASTLE_TEST_DB=port:admin:password");
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let user = parts.next().unwrap();
        let pass = parts.next().unwrap_or("");
        let access = crate::memdb::provision(
            port,
            user,
            pass,
            &format!("Phase7 Test {}", std::process::id()),
        )
        .unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        let job = json!({"name":"First","trigger":{"every_minutes":60},"action":{"kind":"castle","action":"check_database"},"delivery":[],"policy":{}});
        let id = propose(&mut c, job, Path::new(".")).unwrap();
        let mut saved = store::get(&mut c, id).unwrap();
        assert_eq!(saved["state"], "proposed");
        assert_eq!(saved["enabled"], 0);
        let ask = |words: &[&str]| {
            crate::door::parse(&words.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
        };
        assert!(door(
            &mut c,
            &ask(&["automation", "run", "--name", "First"]),
            Path::new(".")
        )
        .is_err());
        assert!(door(
            &mut c,
            &ask(&["automation", "enable", "--name", "First"]),
            Path::new(".")
        )
        .is_err());
        assert!(store::save(&mut c, &saved).is_err());
        saved["enabled"] = json!(true);
        approve(&mut c, &saved).unwrap();
        let saved = store::get(&mut c, id).unwrap();
        assert_eq!(saved["state"], "active");
        assert_eq!(saved["enabled"], 0);
        assert!(approve(&mut c, &saved).is_err());
        let audits: Option<u64> = c
            .query_first("SELECT COUNT(*) FROM automation_approvals")
            .unwrap();
        assert_eq!(audits, Some(1));
        assert!(door(
            &mut c,
            &ask(&["automation", "run", "--name", "First"]),
            Path::new(".")
        )
        .unwrap()
        .contains("queued"));
        assert!(door(
            &mut c,
            &ask(&["automation", "show", "--name", "First"]),
            Path::new(".")
        )
        .unwrap()
        .contains("pending"));
        let runs: Option<u64> = c
            .query_first("SELECT COUNT(*) FROM automation_runs")
            .unwrap();
        assert_eq!(runs, Some(0));
        let mut target = json!({"name":"Second","enabled":true,"trigger":{"every_minutes":60},"action":{"kind":"castle","action":"check_database"},"delivery":[],"policy":{}});
        let next_id = store::save(&mut c, &target).unwrap();
        target["id"] = json!(next_id);
        store::save(&mut c, &target).unwrap();
        let mut root = saved.clone();
        root["policy"] = json!({"on_success_run":"Second"});
        let p = chains::payload(json!({"text":"literal"}), id, "manual", &root);
        let (_, p) = chains::next(&mut c, &root, "ok", &p).unwrap().unwrap();
        assert_eq!(p["_chain"], json!([id, next_id]));
        assert_eq!(p["text"], "literal");
        assert!(chains::next(&mut c, &root, "ok", &p).is_err());
        target["enabled"] = json!(false);
        store::save(&mut c, &target).unwrap();
        assert!(chains::next(
            &mut c,
            &root,
            "ok",
            &chains::payload(json!({}), id, "manual", &root)
        )
        .is_err());
        c.query_drop(format!("DROP DATABASE `{}`", access.name))
            .unwrap();
        let mut admin = Conn::new(
            mysql::OptsBuilder::new()
                .ip_or_hostname(Some("127.0.0.1"))
                .tcp_port(port)
                .user(Some(user))
                .pass(Some(pass)),
        )
        .unwrap();
        for host in ["localhost", "127.0.0.1"] {
            admin
                .query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user))
                .unwrap();
        }
    }
}
