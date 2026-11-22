use super::*;
use mysql::prelude::Queryable;
use tauri::Emitter;
pub fn destinations(j: &Value, status: &str, test: bool) -> Vec<String> {
    if test {
        return Vec::new();
    }
    let list = if status == "ok" {
        &j["delivery"]
    } else {
        &j["policy"]["on_failure_delivery"]
    };
    let mut out: Vec<String> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["kind"].as_str().map(str::to_string))
        .collect();
    if status != "ok" || j["action"]["kind"] == "castle" && j["action"]["action"] == "notify" {
        if !out.iter().any(|d| d == "home") {
            out.push("home".into());
        }
    }
    out
}
fn persist(c: &mut Conn, run: u64, rows: &[Value]) -> Result<(), String> {
    c.exec_drop(
        "UPDATE automation_runs SET delivered=? WHERE id=?",
        (json!(rows).to_string(), run),
    )
    .map_err(|_| "Delivery could not be recorded; no further message was sent.".into())
}
pub fn all(
    app: &AppHandle,
    c: &mut Conn,
    j: &Value,
    run: u64,
    status: &str,
    text: &str,
    test: bool,
) -> Value {
    let store = app.state::<SettingsStore>();
    let selected = destinations(j, status, test);
    if selected.is_empty() {
        return json!([]);
    }
    let mut records = Vec::new();
    let mut home = false;
    let mut phone_failed = false;
    let phone_text = actions::scrub(
        format!(
            "{}: {}\n{}",
            j["name"].as_str().unwrap_or("Automation"),
            if status == "ok" { "OK" } else { "Failed" },
            text
        ),
        &actions::known_secrets(&store),
    );
    for destination in selected {
        if !allowed(&store.get()) {
            records.push(json!({"kind":destination,"status":"skipped","error":"Automations switched off or paused."}));
            continue;
        }
        if destination == "home" {
            home = true;
            records.push(json!({"kind":"home","status":"ok"}));
            continue;
        }
        records.push(json!({"kind":destination,"status":"sending"}));
        if let Err(e) = persist(c, run, &records) {
            let last = records.last_mut().unwrap();
            last["status"] = json!("failed");
            last["error"] = json!(e);
            phone_failed = true;
            break;
        }
        let result = crate::bridges::notify(app, &destination, &phone_text);
        let last = records.last_mut().unwrap();
        match result {
            Ok(parts) => {
                last["status"] = json!("ok");
                last["parts"] = json!(parts);
            }
            Err(error) => {
                last["status"] = json!("failed");
                last["error"] = json!(actions::scrub(error, &actions::known_secrets(&store)));
                phone_failed = true;
            }
        }
        if persist(c, run, &records).is_err() {
            phone_failed = true;
            break;
        }
    }
    if (home || phone_failed) && allowed(&store.get()) {
        let body = if phone_failed {
            "Phone delivery failed. Open Automations history for details."
        } else {
            text
        };
        crate::alerts::fire(
            app,
            "automation",
            j["name"].as_str().unwrap_or("Automation"),
            &actions::capped(body, 400),
        );
        if !home {
            records.push(json!({"kind":"home","status":"ok","reason":"delivery_failure"}));
        }
    }
    let _ = app.emit(
        "castle://automations",
        json!({"status":if phone_failed{"delivery_failed"}else{status}}),
    );
    json!(records)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_uses_its_own_destinations_and_home_is_always_included() {
        let j = json!({"delivery":[{"kind":"telegram"}],"policy":{"on_failure_delivery":[{"kind":"discord"}]}});
        assert_eq!(destinations(&j, "ok", false), vec!["telegram"]);
        assert_eq!(destinations(&j, "failed", false), vec!["discord", "home"]);
        assert!(destinations(&j, "failed", true).is_empty());
    }
    #[test]
    fn home_notification_action_always_has_home() {
        let j = json!({"action":{"kind":"castle","action":"notify"},"delivery":[]});
        assert_eq!(destinations(&j, "ok", false), vec!["home"]);
    }
}
