use super::*;
use mysql::prelude::Queryable;
use std::time::{Duration, Instant};
pub fn room_capacity(ids: &BTreeSet<u64>, id: u64) -> bool {
    ids.len() < 2 && !ids.contains(&id)
}
/// Caller holds Running.0, shared by startup recovery and every launch.
pub(super) fn recover_once(c: &mut Conn, state: &Running) -> Result<(), String> {
    let mut recovered = state.1.lock().map_err(|_| "Recovery state unavailable.")?;
    if !*recovered {
        store::recover(c)?;
        *recovered = true;
    }
    Ok(())
}
fn begin_run(c: &mut Conn, j: &Value, cause: &str) -> Result<u64, String> {
    if cause == "schedule" {
        store::begin_scheduled(c, j)
    } else {
        store::begin(c, j["id"].as_u64().ok_or("Missing job id.")?, cause)
    }
}
struct ActiveRun(AppHandle, u64);
impl Drop for ActiveRun {
    fn drop(&mut self) {
        self.0
            .state::<Running>()
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&self.1);
    }
}
pub fn launch(app: &AppHandle, id: u64, cause: &str) -> Result<u64, String> {
    let payload = json!({"cause":cause,"time":schedule::stamp(chrono::Local::now()),"text":if cause == "test" {"Sample invoice message"} else {""}});
    launch_payload(app, id, cause, payload, None)
}
pub fn launch_payload(
    app: &AppHandle,
    id: u64,
    cause: &str,
    payload: Value,
    expected: Option<&Value>,
) -> Result<u64, String> {
    let state = app.state::<Running>();
    let mut active = state.0.lock().unwrap();
    if !allowed(&app.state::<SettingsStore>().get()) {
        return Err("Automations are off or paused.".into());
    }
    let mut c = connection(&app.state::<SettingsStore>())?;
    recover_once(&mut c, &state)?;
    let j = store::get(&mut c, id)?;
    if j["state"] != "active" {
        return Err("Only approved active jobs may run.".into());
    }
    validate(&j)?;
    if matches!(cause, "message" | "file" | "webhook" | "event")
        && (j["enabled"] != 1 || j["triggerKind"] != cause || expected != Some(&j))
    {
        return Err(format!(
            "The {cause} job changed or was switched off before it started."
        ));
    }
    let payload = chains::payload(payload, id, cause, &j);
    if cause == "chain" {
        if j["enabled"] != 1 || expected != Some(&j) {
            return Err("Follow-up changed or was disabled.".into());
        }
        let mut origin = payload.clone();
        origin["_chain"]
            .as_array_mut()
            .ok_or("Missing chain path.")?
            .pop();
        chains::guard(&origin, id, &j)?;
    }
    if !room_capacity(&active, id) {
        let run = begin_run(&mut c, &j, cause)?;
        store::finish(
            &mut c,
            run,
            "skipped",
            "",
            Some("A job is already running or both run slots are occupied."),
            json!([]),
        )?;
        return Ok(run);
    }
    let recent:Option<u64>=c.exec_first("SELECT COUNT(*) FROM automation_runs WHERE job_id=? AND started_at>DATE_SUB(NOW(),INTERVAL 1 HOUR) AND status NOT IN ('skipped','cancelled')",(id,)).map_err(|_|"Run limit could not be checked.")?;
    if recent.unwrap_or(0) >= j["policy"]["rate_per_hour"].as_u64().unwrap_or(6) {
        let run = begin_run(&mut c, &j, cause)?;
        store::finish(
            &mut c,
            run,
            "skipped",
            "",
            Some("Hourly run limit reached."),
            json!([]),
        )?;
        return Ok(run);
    }
    let run = begin_run(&mut c, &j, cause)?;
    active.insert(id);
    drop(active);
    let app = app.clone();
    let test = cause == "test";
    std::thread::spawn(move || {
        let active_guard = ActiveRun(app.clone(), id);
        let mut attempt = 0;
        let retries = j["policy"]["retries"].as_u64().unwrap_or(0);
        let result = loop {
            if !allowed(&app.state::<SettingsStore>().get()) {
                break actions::Report {
                    status: "cancelled",
                    output: String::new(),
                    error: Some("Cancelled: automations paused or switched off.".into()),
                };
            }
            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                actions::perform(&app, &j, run, &payload)
            })) {
                Ok(result) => result,
                Err(_) => {
                    break actions::Report {
                        status: "failed",
                        output: String::new(),
                        error: Some("The action stopped unexpectedly. It was not repeated.".into()),
                    }
                }
            };
            if result.status == "ok" || attempt >= retries {
                break result;
            }
            let until = Instant::now() + Duration::from_secs([60, 300, 900][attempt as usize]);
            attempt += 1;
            while Instant::now() < until && allowed(&app.state::<SettingsStore>().get()) {
                std::thread::sleep(Duration::from_secs(1));
            }
        };
        let actions::Report {
            status,
            output,
            error,
        } = result;
        let mut pending = Some(health::Pending::new(&app));
        // Persist before delivery. Failed persistence must never look like a completed job.
        loop {
            if let Ok(mut c) = connection(&app.state::<SettingsStore>()) {
                if store::finish(&mut c, run, status, &output, error.as_deref(), json!([])).is_ok()
                {
                    drop(pending.take());
                    let delivered = deliver::all(
                        &app,
                        &mut c,
                        &j,
                        run,
                        status,
                        error.as_deref().unwrap_or(&output),
                        test,
                    );
                    let _ = c.exec_drop(
                        "UPDATE automation_runs SET delivered=? WHERE id=?",
                        (delivered.to_string(), run),
                    );
                    break;
                }
            }
            std::thread::sleep(Duration::from_secs(15));
        }
        drop(active_guard);
        if !test && allowed(&app.state::<SettingsStore>().get()) {
            chains::follow(&app, &j, run, status, &payload);
        }
    });
    Ok(run)
}
pub fn due(j: &Value, now: &str) -> bool {
    j["triggerKind"].as_str().unwrap_or("schedule") == "schedule"
        && j["enabled"].as_i64() == Some(1)
        && j["state"] == "active"
        && j["nextRun"].as_str().is_some_and(|t| t <= now)
}
pub fn missed_skip(interrupted: bool, missed: &str) -> bool {
    interrupted && missed != "run_once"
}
/// Skip at most one missed occurrence. A resumed run-once job uses the same atomic claim.
pub(super) fn skip_missed(c: &mut Conn, j: &Value, reason: Option<&str>) -> Result<bool, String> {
    if missed_skip(
        reason.is_some(),
        j["policy"]["missed"].as_str().unwrap_or("skip"),
    ) {
        let run = store::begin_scheduled(c, j)?;
        store::finish(
            c,
            run,
            "skipped",
            "",
            Some(&format!(
                "Missed while the runner was unavailable: {}.",
                reason.unwrap()
            )),
            json!([]),
        )?;
        return Ok(true);
    }
    Ok(false)
}
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut recovered = false;
        let mut continuity = health::Continuity::default();
        let mut retention = Instant::now() - Duration::from_secs(86400);
        loop {
            let h = app.state::<health::Health>();
            if let Some(reason) = h.take_interruption() {
                continuity.interrupted(reason);
            }
            let interruption = continuity.tick(chrono::Utc::now().timestamp_millis());
            h.beat(chrono::Utc::now().timestamp_millis());
            if !allowed(&app.state::<SettingsStore>().get()) {
                continuity.interrupted("Automations were off or paused");
            } else {
                let scan = (|| -> Result<(), String> {
                    let mut c = connection(&app.state::<SettingsStore>())?;
                    h.database(true, None);
                    if !recovered {
                        let state = app.state::<Running>();
                        let _active = state.0.lock().unwrap();
                        recover_once(&mut c, &state)?;
                        recovered = true;
                    }
                    let jobs = store::list(&mut c)?;
                    companion::requests(&app, &mut c);
                    let now = schedule::stamp(chrono::Local::now());
                    let mut problems = Vec::new();
                    let mut paused = false;
                    for j in jobs {
                        if !allowed(&app.state::<SettingsStore>().get()) {
                            continuity.interrupted("Automations were off or paused");
                            paused = true;
                            break;
                        }
                        if !due(&j, &now) {
                            continue;
                        }
                        let result = skip_missed(&mut c, &j, interruption).and_then(|skip| {
                            if skip {
                                Ok(())
                            } else {
                                launch(&app, j["id"].as_u64().ok_or("Missing job id.")?, "schedule")
                                    .map(|_| ())
                            }
                        });
                        if let Err(e) = result {
                            problems.push(e);
                        }
                    }
                    if !paused {
                        if problems.is_empty() {
                            if let Some(reason) = interruption {
                                h.interruption(reason);
                            }
                            continuity.scanned();
                        } else {
                            continuity.interrupted("A schedule check failed");
                        }
                    }
                    h.scan((!problems.is_empty()).then(|| problems.join(" ")));
                    if retention.elapsed() >= Duration::from_secs(86400) {
                        store::retain(&mut c)?;
                        c.query_drop("DELETE FROM automation_messages WHERE created_at<DATE_SUB(NOW(),INTERVAL 30 DAY)").map_err(|_|"Message receipt retention failed.")?;
                        retention = Instant::now();
                    }
                    Ok(())
                })();
                if let Err(problem) = scan {
                    h.database(false, Some(problem));
                    continuity.interrupted("Database was unavailable");
                }
            }
            let _ = app.emit("castle://automations", json!({"health":true}));
            std::thread::sleep(Duration::from_secs(15));
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrency_and_same_job() {
        let mut ids = BTreeSet::new();
        assert!(room_capacity(&ids, 1));
        ids.insert(1);
        assert!(!room_capacity(&ids, 1));
        assert!(room_capacity(&ids, 2));
        ids.insert(2);
        assert!(!room_capacity(&ids, 3));
    }
    #[test]
    fn missed_runs_never_replay_every_occurrence() {
        assert!(missed_skip(true, "skip"));
        assert!(!missed_skip(true, "run_once"));
        assert!(!missed_skip(false, "skip"));
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    #[test]
    fn only_enabled_approved_due_schedules_run() {
        let mut j = json!({"enabled":1,"state":"active","nextRun":"2026-11-21 08:00:00"});
        assert!(due(&j, "2026-11-21 09:00:00"));
        assert!(!due(&j, "2026-11-20 09:00:00"));
        j["state"] = json!("proposed");
        assert!(!due(&j, "2026-11-21 09:00:00"));
        j["state"] = json!("active");
        j["enabled"] = json!(0);
        assert!(!due(&j, "2026-11-21 09:00:00"));
    }
}
#[cfg(test)]
mod reliability_tests {
    use super::*;
    #[test]
    #[ignore]
    fn live_interruption_policy_and_atomic_schedule_claims() {
        let cfg = std::env::var("CASTLE_TEST_DB").expect("CASTLE_TEST_DB=port:admin:password");
        let mut b = cfg.splitn(3, ':');
        let port = b.next().unwrap().parse().unwrap();
        let user = b.next().unwrap();
        let pw = b.next().unwrap_or("");
        let access = crate::memdb::provision(
            port,
            user,
            pw,
            &format!("Phase8 Test {}", std::process::id()),
        )
        .unwrap();
        eprintln!(
            "Owned fixture: database={}, user={}",
            access.name, access.user
        );
        let mut c = crate::memdb::open(&access).unwrap();
        let mut skip = json!({"name":"Skip after interruption","enabled":true,"trigger":{"every_minutes":1},"action":{"kind":"castle","action":"check_database"},"delivery":[],"policy":{"missed":"skip"}});
        let id = store::save(&mut c, &skip).unwrap();
        skip["id"] = json!(id);
        store::save(&mut c, &skip).unwrap();
        c.exec_drop(
            "UPDATE automation_jobs SET next_run_at=DATE_SUB(NOW(),INTERVAL 10 MINUTE) WHERE id=?",
            (id,),
        )
        .unwrap();
        let due = store::get(&mut c, id).unwrap();
        assert!(skip_missed(&mut c, &due, Some("Database was unavailable")).unwrap());
        assert_eq!(store::runs(&mut c, id).unwrap()[0]["status"], "skipped");
        assert!(store::begin_scheduled(&mut c, &due).is_err());
        let mut once = skip.clone();
        once.as_object_mut().unwrap().remove("id");
        once["name"] = json!("Run once after interruption");
        once["policy"]["missed"] = json!("run_once");
        let id = store::save(&mut c, &once).unwrap();
        once["id"] = json!(id);
        store::save(&mut c, &once).unwrap();
        c.exec_drop(
            "UPDATE automation_jobs SET next_run_at=DATE_SUB(NOW(),INTERVAL 10 MINUTE) WHERE id=?",
            (id,),
        )
        .unwrap();
        let due = store::get(&mut c, id).unwrap();
        assert!(!skip_missed(&mut c, &due, Some("Execution or clock gap")).unwrap());
        c.query_drop("CREATE TRIGGER phase8_refuse_advance BEFORE UPDATE ON automation_jobs FOR EACH ROW SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='owned rollback fixture'").unwrap();
        assert!(store::begin_scheduled(&mut c, &due).is_err());
        assert!(store::runs(&mut c, id).unwrap().is_empty());
        assert_eq!(store::get(&mut c, id).unwrap()["nextRun"], due["nextRun"]);
        c.query_drop("DROP TRIGGER phase8_refuse_advance").unwrap();
        let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let mut conn = crate::memdb::open(&access).unwrap();
            let gate = gate.clone();
            let due = due.clone();
            threads.push(std::thread::spawn(move || {
                gate.wait();
                store::begin_scheduled(&mut conn, &due)
            }));
        }
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(store::runs(&mut c, id).unwrap().len(), 1);
        let run = results.into_iter().find_map(Result::ok).unwrap();
        store::finish(&mut c, run, "ok", "one completed result", None, json!([])).unwrap();
        store::finish(&mut c, run, "ok", "must not overwrite", None, json!([])).unwrap();
        assert_eq!(
            store::runs(&mut c, id).unwrap()[0]["output"],
            "one completed result"
        );
        c.query_drop(format!("DROP DATABASE `{}`", access.name))
            .unwrap();
        let mut admin = Conn::new(
            mysql::OptsBuilder::new()
                .ip_or_hostname(Some("127.0.0.1"))
                .tcp_port(port)
                .user(Some(user))
                .pass(Some(pw)),
        )
        .unwrap();
        for host in ["localhost", "127.0.0.1"] {
            admin
                .query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user))
                .unwrap();
        }
    }
}
