//! Fixed member-authored follow-ups; no output or payload chooses another job.
use super::*;
pub fn validate_policy(j: &Value) -> Result<(), String> {
    for key in ["on_success_run", "on_failure_run"] {
        let v = &j["policy"][key];
        if !v.is_null()
            && v.as_str()
                .is_none_or(|s| s.len() > 320 || s.chars().count() > 80 || s.contains('\0'))
        {
            return Err("Choose a follow-up job name of at most 80 characters.".into());
        }
        if v.as_str()
            .is_some_and(|s| !s.is_empty() && s == j["name"].as_str().unwrap_or(""))
        {
            return Err("A job cannot follow itself.".into());
        }
    }
    Ok(())
}
pub fn payload(mut p: Value, id: u64, cause: &str, job: &Value) -> Value {
    if cause != "chain" {
        p["_chain"] = json!([id]);
        p["_untrusted"] = json!(
            matches!(cause, "message" | "file" | "webhook")
                || matches!(
                    job["triggerKind"].as_str(),
                    Some("message" | "file" | "webhook")
                )
        );
    }
    p
}
pub fn guard(p: &Value, target: u64, job: &Value) -> Result<(), String> {
    let path = p["_chain"].as_array().ok_or("Missing chain origin.")?;
    if path.len() >= 4 || path.contains(&json!(target)) {
        return Err("Follow-up stopped: chain depth or repeated job limit.".into());
    }
    if p["_untrusted"] == true
        && job["action"]["kind"] == "castle"
        && matches!(
            job["action"]["action"].as_str(),
            Some("wake_companion" | "save_diary_prompt")
        )
    {
        return Err(
            "Untrusted triggers cannot reach companion session actions through a chain.".into(),
        );
    }
    Ok(())
}
pub fn next(
    c: &mut Conn,
    j: &Value,
    status: &str,
    p: &Value,
) -> Result<Option<(Value, Value)>, String> {
    let key = if status == "ok" {
        "on_success_run"
    } else if matches!(status, "failed" | "timed_out") {
        "on_failure_run"
    } else {
        return Ok(None);
    };
    let Some(name) = j["policy"][key].as_str().filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let target = store::list(c)?
        .into_iter()
        .find(|j| j["name"] == name && j["state"] == "active" && j["enabled"] == 1)
        .ok_or("Follow-up job is missing, disabled or awaiting approval.")?;
    guard(
        p,
        target["id"].as_u64().ok_or("Missing follow-up id.")?,
        &target,
    )?;
    let mut p = p.clone();
    p["cause"] = json!("chain");
    p["_chain"]
        .as_array_mut()
        .unwrap()
        .push(target["id"].clone());
    Ok(Some((target, p)))
}
fn record(c: &mut Conn, run: u64, entry: Value) -> Result<(), String> {
    let text: Option<String> = c
        .exec_first("SELECT delivered FROM automation_runs WHERE id=?", (run,))
        .map_err(|_| "Follow-up audit unavailable.")?;
    let mut entries: Vec<Value> = serde_json::from_str(&text.ok_or("Missing parent run.")?)
        .map_err(|_| "Follow-up audit unreadable.")?;
    entries.retain(|e| e["kind"] != "chain");
    entries.push(entry);
    c.exec_drop(
        "UPDATE automation_runs SET delivered=? WHERE id=?",
        (json!(entries).to_string(), run),
    )
    .map_err(|_| "Follow-up audit could not be stored; no new follow-up started.".into())
}
pub fn follow(app: &AppHandle, j: &Value, run: u64, status: &str, p: &Value) {
    let result = (|| {
        let mut c = connection(&app.state::<SettingsStore>())?;
        let Some((target, p)) = next(&mut c, j, status, p)? else {
            return Ok(None);
        };
        let target_id = target["id"].as_u64().ok_or("Missing follow-up id.")?;
        record(
            &mut c,
            run,
            json!({"kind":"chain","status":"sending","jobId":target_id}),
        )?;
        let child = runner::launch_payload(app, target_id, "chain", p, Some(&target))?;
        Ok::<_, String>(Some(
            json!({"kind":"chain","status":"ok","runId":child,"jobId":target_id}),
        ))
    })();
    let entry = match result {
        Ok(None) => return,
        Ok(Some(e)) => e,
        Err(error) => json!({"kind":"chain","status":"failed","error":error}),
    };
    if let Ok(mut c) = connection(&app.state::<SettingsStore>()) {
        let _ = record(&mut c, run, entry.clone());
    }
    if entry["status"] == "failed" {
        crate::alerts::fire(
            app,
            "automation",
            "Automation follow-up stopped",
            entry["error"].as_str().unwrap_or("Follow-up failed."),
        );
    }
    let _ = app.emit("castle://automations", json!({"chain":true}));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn depth_cycles_and_untrusted_origin() {
        let j = json!({"triggerKind":"message"});
        let p = payload(json!({"text":";calc"}), 1, "message", &j);
        let target = json!({"action":{"kind":"castle","action":"save_diary_prompt"}});
        assert!(guard(&p, 2, &target).is_err());
        assert!(guard(&p, 1, &json!({})).is_err());
        assert!(guard(&json!({"_chain":[1,2,3,4]}), 5, &json!({})).is_err());
        assert!(guard(&json!({"_chain":[1,2,3],"_untrusted":false}), 4, &target).is_ok());
        assert!(validate_policy(&json!({"name":"A","policy":{"on_success_run":"A"}})).is_err());
    }
}
