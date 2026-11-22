use super::schedule;
use mysql::{params, prelude::Queryable, Conn};
use serde_json::{json, Value};
pub fn list(c: &mut Conn) -> Result<Vec<Value>, String> {
    let rows:Vec<String>=c.query("SELECT JSON_OBJECT('id',j.id,'name',name,'note',note,'enabled',enabled,'triggerKind',trigger_kind,'trigger',JSON_EXTRACT(trigger_json,'$'),'action',JSON_EXTRACT(action_json,'$'),'delivery',JSON_EXTRACT(delivery_json,'$'),'policy',JSON_EXTRACT(policy_json,'$'),'state',state,'nextRun',DATE_FORMAT(next_run_at,'%Y-%m-%d %H:%i:%s'),'lastStatus',(SELECT CASE WHEN JSON_SEARCH(delivered,'one','failed',NULL,'$[*].status') IS NOT NULL THEN 'delivery_failed' WHEN JSON_SEARCH(delivered,'one','unknown',NULL,'$[*].status') IS NOT NULL THEN 'delivery_unknown' WHEN JSON_SEARCH(delivered,'one','sending',NULL,'$[*].status') IS NOT NULL THEN 'delivery_sending' ELSE status END FROM automation_runs r WHERE r.job_id=j.id ORDER BY r.id DESC LIMIT 1)) FROM automation_jobs j ORDER BY name").map_err(|_|"Jobs could not be read. Start the memory database in Settings > Database.".to_string())?;
    rows.into_iter()
        .map(|s| serde_json::from_str(&s).map_err(|_| "A job could not be read.".into()))
        .collect()
}
pub fn get(c: &mut Conn, id: u64) -> Result<Value, String> {
    list(c)?
        .into_iter()
        .find(|j| j["id"].as_u64() == Some(id))
        .ok_or("That job no longer exists.".into())
}
pub fn save(c: &mut Conn, j: &Value) -> Result<u64, String> {
    let next = if j["triggerKind"].as_str().unwrap_or("schedule") == "schedule" && j["enabled"].as_bool() == Some(true) {
        schedule::next(&j["trigger"], chrono::Local::now()).map(schedule::stamp)
    } else {
        None
    };
    let args = params! {"trigger_kind"=>j["triggerKind"].as_str().unwrap_or("schedule"),"name"=>j["name"].as_str().unwrap_or(""),"note"=>j["note"].as_str().unwrap_or(""),"enabled"=>j["enabled"].as_bool().unwrap_or(false),"trigger"=>j["trigger"].to_string(),"action"=>j["action"].to_string(),"delivery"=>j["delivery"].to_string(),"policy"=>j["policy"].to_string(),"next"=>next,"id"=>j["id"].as_u64().unwrap_or(0),"kind"=>if j["action"]["kind"]=="command"{"command"}else if j["action"]["kind"]=="castle"{"castle"}else if j["action"]["kind"]=="http"{"http"}else{"companion"}};
    if j["id"].as_u64().unwrap_or(0)>0 {c.exec_drop("UPDATE automation_jobs SET name=:name,note=:note,enabled=:enabled,trigger_kind=:trigger_kind,action_kind=:kind,trigger_json=:trigger,action_json=:action,delivery_json=:delivery,policy_json=:policy,next_run_at=:next WHERE id=:id AND state='active'",args)}
 else {c.exec_drop("INSERT INTO automation_jobs (name,note,enabled,trigger_kind,action_kind,trigger_json,action_json,delivery_json,policy_json,next_run_at) VALUES (:name,:note,0,:trigger_kind,:kind,:trigger,:action,:delivery,:policy,NULL)",args)}.map_err(|_|"Job could not be saved. Use a unique name.".to_string())?;
    if j["id"].as_u64().unwrap_or(0) > 0 && c.affected_rows() == 0 {
        let exists: Option<u64> = c
            .exec_first(
                "SELECT id FROM automation_jobs WHERE id=? AND state='active'",
                (j["id"].as_u64(),),
            )
            .map_err(|_| "Job could not be checked.".to_string())?;
        if exists.is_none() {
            return Err("Only existing approved jobs can be edited.".into());
        }
    }
    Ok(j["id"]
        .as_u64()
        .filter(|n| *n > 0)
        .unwrap_or(c.last_insert_id()))
}
pub fn runs(c: &mut Conn, id: u64) -> Result<Vec<Value>, String> {
    let rows:Vec<String>=c.exec("SELECT JSON_OBJECT('id',id,'jobId',job_id,'startedAt',CAST(started_at AS CHAR),'finishedAt',CAST(finished_at AS CHAR),'cause',cause,'status',status,'summary',summary,'output',output,'error',error,'delivered',JSON_EXTRACT(delivered,'$')) FROM automation_runs WHERE job_id=? ORDER BY id DESC LIMIT 200",(id,)).map_err(|_|"History could not be read.".to_string())?;
    rows.into_iter()
        .map(|s| serde_json::from_str(&s).map_err(|_| "History is unreadable.".into()))
        .collect()
}
pub fn begin(c: &mut Conn, id: u64, cause: &str) -> Result<u64, String> {
    c.exec_drop(
        "INSERT INTO automation_runs (job_id,cause,output,delivered) VALUES (?,?,'','[]')",
        (id, cause),
    )
    .map_err(|_| "Run could not be recorded.".to_string())?;
    Ok(c.last_insert_id())
}
/// Claim the due occurrence and advance its schedule in the same transaction.
/// A failed/uncertain commit never starts an action; the occurrence cannot be replayed.
pub fn begin_scheduled(c:&mut Conn,j:&Value)->Result<u64,String>{
 c.query_drop("START TRANSACTION").map_err(|_|"The schedule claim could not begin.")?;
 let result=(||{let current:Option<String>=c.exec_first("SELECT DATE_FORMAT(next_run_at,'%Y-%m-%d %H:%i:%s') FROM automation_jobs WHERE id=? AND state='active' AND enabled=1 AND trigger_kind='schedule' AND next_run_at IS NOT NULL FOR UPDATE",(j["id"].as_u64(),)).map_err(|_|"The schedule claim could not be checked.")?;
 if current.as_deref()!=j["nextRun"].as_str()||current.is_none(){return Err("That scheduled occurrence was already claimed or changed.".into());}
 let run=begin(c,j["id"].as_u64().ok_or("Missing job id.")?,"schedule")?;advance(c,j)?;Ok(run)})();
 if result.is_ok(){if c.query_drop("COMMIT").is_err(){let _=c.query_drop("ROLLBACK");return Err("Schedule claim confirmation unknown; no action was started.".into());}}else{let _=c.query_drop("ROLLBACK");}result
}
pub fn finish(
    c: &mut Conn,
    id: u64,
    status: &str,
    output: &str,
    error: Option<&str>,
    delivered: Value,
) -> Result<(), String> {
    let summary: String = error.unwrap_or(output).chars().take(400).collect();
    c.exec_drop("UPDATE automation_runs SET status=?,finished_at=NOW(),summary=?,output=?,error=?,delivered=? WHERE id=? AND status='running'",(status,summary,output,error,delivered.to_string(),id)).map_err(|_|"Run result could not be saved.".into())
}
pub fn recover(c: &mut Conn) -> Result<(), String> {
    c.query_drop("UPDATE automation_runs SET status='failed',finished_at=NOW(),error='Castle closed during the run',summary='Castle closed during the run' WHERE status='running'").map_err(|_|"Interrupted runs could not be recovered.".to_string())?;
    let rows:Vec<(u64,String)>=c.query("SELECT id,delivered FROM automation_runs WHERE JSON_SEARCH(delivered,'one','sending',NULL,'$[*].status') IS NOT NULL").map_err(|_|"Delivery recovery failed.".to_string())?;
    for (id, text) in rows {
        let mut delivered: Vec<Value> = serde_json::from_str(&text)
            .map_err(|_| "Delivery history is unreadable.".to_string())?;
        for entry in &mut delivered {
            if entry["status"] == "sending" {
                entry["status"] = json!("unknown");
                entry["error"] =
                    json!("Castle closed before delivery confirmation. It was not resent.");
            }
        }
        c.exec_drop(
            "UPDATE automation_runs SET delivered=? WHERE id=?",
            (json!(delivered).to_string(), id),
        )
        .map_err(|_| "Delivery recovery failed.".to_string())?;
    }
    Ok(())
}

pub fn advance(c: &mut Conn, j: &Value) -> Result<(), String> {
    let next = schedule::next(&j["trigger"], chrono::Local::now()).map(schedule::stamp);
    c.exec_drop("UPDATE automation_jobs SET next_run_at=?,enabled=IF(? IS NULL,0,enabled),state=IF(? IS NULL AND JSON_EXTRACT(trigger_json,'$.at') IS NOT NULL,'archived',state) WHERE id=?",(&next,&next,&next,j["id"].as_u64())).map_err(|_|"Next run could not be saved.".into())
}
pub fn retain(c: &mut Conn) -> Result<(), String> {
    let ids: Vec<u64> = c
        .query("SELECT id FROM automation_jobs")
        .map_err(|_| "Retention failed.".to_string())?;
    for id in ids {
        c.exec_drop("DELETE FROM automation_runs WHERE job_id=? AND status!='running' AND id < COALESCE((SELECT cutoff FROM (SELECT id AS cutoff FROM automation_runs WHERE job_id=? ORDER BY id DESC LIMIT 1 OFFSET 199) k),0)",(id,id)).map_err(|_|"Retention failed.".to_string())?;
    }
    Ok(())
}
pub fn home(c: &mut Conn) -> Result<Value, String> {
    let rows:Vec<String>=c.query("SELECT JSON_OBJECT('name',j.name,'status',CASE WHEN JSON_SEARCH(r.delivered,'one','failed',NULL,'$[*].status') IS NOT NULL THEN 'delivery_failed' WHEN JSON_SEARCH(r.delivered,'one','unknown',NULL,'$[*].status') IS NOT NULL THEN 'delivery_unknown' ELSE r.status END,'summary',CASE WHEN JSON_SEARCH(r.delivered,'one','failed',NULL,'$[*].status') IS NOT NULL THEN 'A delivery or follow-up failed; open Automations history for details.' WHEN JSON_SEARCH(r.delivered,'one','unknown',NULL,'$[*].status') IS NOT NULL THEN 'Delivery or follow-up confirmation unknown; it was not replayed.' ELSE r.summary END,'at',CAST(r.started_at AS CHAR)) FROM automation_runs r JOIN automation_jobs j ON j.id=r.job_id WHERE r.cause!='test' AND (r.status IN ('failed','timed_out') OR JSON_SEARCH(r.delivered,'one','failed',NULL,'$[*].status') IS NOT NULL OR JSON_SEARCH(r.delivered,'one','unknown',NULL,'$[*].status') IS NOT NULL OR JSON_CONTAINS(j.delivery_json,JSON_OBJECT('kind','home'))) ORDER BY r.id DESC LIMIT 5").map_err(|_|"Automations notices unavailable.".to_string())?;
    Ok(json!(rows
        .into_iter()
        .filter_map(|s| serde_json::from_str::<Value>(&s).ok())
        .collect::<Vec<_>>()))
}

#[cfg(test)]
mod live_tests {
    use super::*;
    /// Opt-in, isolated database. The test creates and removes only its exact generated DB/user.
    #[test]
    #[ignore]
    fn fresh_existing_history_recovery_and_retention() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("CASTLE_TEST_DB=port:admin:password");
        let mut bits = spec.splitn(3, ':');
        let port = bits.next().unwrap().parse().unwrap();
        let admin = bits.next().unwrap();
        let pw = bits.next().unwrap_or("");
        let access = crate::memdb::provision(
            port,
            admin,
            pw,
            &format!("Automation Test {}", std::process::id()),
        )
        .unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        assert!(crate::memdb::migrate(&mut c).unwrap().is_empty());
        let mut j = json!({"name":"Briefing test","note":"owned fixture","trigger":{"every_minutes":1},"action":{"mode":"think","prompt":"Hello"},"delivery":[{"kind":"home"}],"policy":{},"enabled":true});
        let id = save(&mut c, &j).unwrap();
        let saved = get(&mut c, id).unwrap();
        assert_eq!(saved["enabled"], 0);
        assert!(saved["nextRun"].is_null());
        j["id"] = json!(id);
        save(&mut c, &j).unwrap();
        assert_eq!(get(&mut c, id).unwrap()["enabled"], 1);
        assert!(get(&mut c, id).unwrap()["nextRun"].is_string());
        j["action"] = json!({"kind":"command","program":"fixture.exe","args":[],"cwd":"fixture","env":{},"stdin":"none"});
        save(&mut c, &j).unwrap();
        let kind: Option<String> = c
            .exec_first("SELECT action_kind FROM automation_jobs WHERE id=?", (id,))
            .unwrap();
        assert_eq!(kind.as_deref(), Some("command"));
        assert_eq!(get(&mut c, id).unwrap()["action"]["kind"], "command");
        let run = begin(&mut c, id, "manual").unwrap();
        recover(&mut c).unwrap();
        assert_eq!(runs(&mut c, id).unwrap()[0]["status"], "failed");
        finish(&mut c, run, "ok", "late output", None, json!([])).unwrap();
        assert_eq!(runs(&mut c, id).unwrap()[0]["status"], "failed");
        for _ in 0..205 {
            let run = begin(&mut c, id, "manual").unwrap();
            finish(
                &mut c,
                run,
                "ok",
                "Result <script> stays plain text",
                None,
                json!([{"kind":"home"}]),
            )
            .unwrap();
        }
        retain(&mut c).unwrap();
        let count: Option<u64> = c
            .exec_first("SELECT COUNT(*) FROM automation_runs WHERE job_id=?", (id,))
            .unwrap();
        assert_eq!(count, Some(200));
        assert_eq!(runs(&mut c, id).unwrap().len(), 200);
        c.exec_drop(
            "UPDATE automation_jobs SET state='proposed' WHERE id=?",
            (id,),
        )
        .unwrap();
        assert_eq!(get(&mut c, id).unwrap()["state"], "proposed");
        let sent = begin(&mut c, id, "manual").unwrap();
        finish(
            &mut c,
            sent,
            "ok",
            "fixture",
            None,
            json!([{"kind":"telegram","status":"sending"}]),
        )
        .unwrap();
        recover(&mut c).unwrap();
        assert_eq!(
            runs(&mut c, id).unwrap()[0]["delivered"][0]["status"],
            "unknown"
        );
        c.exec_drop("DELETE FROM automation_runs WHERE id=?", (sent,))
            .unwrap();
        // Reopen the actual server connection: saved jobs and history survive a process restart.
        drop(c);
        let mut c = crate::memdb::open(&access).unwrap();
        assert_eq!(runs(&mut c, id).unwrap().len(), 200);
        // Re-apply 018 to an existing database still at 017, then re-open it.
        c.query_drop("DROP TABLE automation_requests,automation_approvals").unwrap();
        c.query_drop("DELETE FROM schema_migrations WHERE name='020_automation_companion'").unwrap();
        c.query_drop("DROP TABLE automation_messages").unwrap();
        c.query_drop("DELETE FROM schema_migrations WHERE name='019_automation_messages'").unwrap();
        c.query_drop("DROP TABLE automation_runs").unwrap();
        c.query_drop("DROP TABLE automation_jobs").unwrap();
        c.query_drop("DELETE FROM schema_migrations WHERE name='018_automations'")
            .unwrap();
        assert_eq!(
            crate::memdb::migrate(&mut c).unwrap(),
            vec!["018_automations", "019_automation_messages", "020_automation_companion"]
        );
        c.query_drop(format!("DROP DATABASE `{}`", access.name))
            .unwrap();
        // Account cleanup uses the same root access explicitly supplied to this test.
        let opts = mysql::OptsBuilder::new()
            .ip_or_hostname(Some("127.0.0.1"))
            .tcp_port(port)
            .user(Some(admin))
            .pass(Some(pw));
        let mut root = Conn::new(opts).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user))
                .unwrap();
        }
    }
}
