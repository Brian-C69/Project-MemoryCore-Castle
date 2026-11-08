//! Codex's record: date-sharded "rollout" files in %CODEX_HOME%\sessions\YYYY\MM\DD\
//! (default %USERPROFILE%\.codex). The first line, `session_meta`, names the working folder;
//! `event_msg` lines carry the member's and the companion's words; `response_item` lines carry
//! the tools it ran.

use super::{media_paths, short, strip_insight, ChatEntry, Follower, WORK_DETAIL_LIMIT};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Where Codex keeps its sessions for this computer's user.
pub fn sessions_root() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CODEX_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(home).join("sessions"));
    }
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex").join("sessions"))
}

/// A folder path compared the way Windows does: case-insensitive, either slash, no trailing slash.
fn same_folder(a: &str, b: &str) -> bool {
    let norm = |p: &str| p.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    norm(a) == norm(b)
}

/// The working folder a rollout file belongs to, from its first `session_meta` line.
pub fn rollout_cwd(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(5).map_while(Result::ok) {
        let Ok(record) = serde_json::from_str::<Value>(&line) else { continue };
        if record.get("type").and_then(Value::as_str) == Some("session_meta") {
            return record.get("payload").and_then(|p| p.get("cwd")).and_then(Value::as_str).map(str::to_string);
        }
    }
    None
}

fn modified(path: &Path) -> Option<SystemTime> {
    path.metadata().and_then(|m| m.modified()).ok()
}

/// Rollout files under `root` written since `floor`. A day folder that has not changed since
/// `floor` cannot hold a new file, so older days are never opened.
fn recent_rollouts(root: &Path, floor: SystemTime) -> Vec<(SystemTime, PathBuf)> {
    let mut found = Vec::new();
    let dirs = |p: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(p).map(|e| e.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default()
    };
    for year in dirs(root) {
        for month in dirs(&year) {
            for day in dirs(&month) {
                if modified(&day).is_some_and(|t| t < floor) {
                    continue;
                }
                let Ok(entries) = std::fs::read_dir(&day) else { continue };
                for path in entries.flatten().map(|e| e.path()) {
                    let is_rollout = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"));
                    if let (true, Some(time)) = (is_rollout, modified(&path)) {
                        if time >= floor {
                            found.push((time, path));
                        }
                    }
                }
            }
        }
    }
    found
}

/// The newest rollout for `folder` written since `since` (a minute's grace for clocks).
pub fn newest_record(folder: &Path, since: SystemTime) -> Option<PathBuf> {
    newest_record_in(&sessions_root()?, folder, since)
}

pub fn newest_record_in(root: &Path, folder: &Path, since: SystemTime) -> Option<PathBuf> {
    let floor = since.checked_sub(Duration::from_secs(60)).unwrap_or(since);
    let wanted = folder.display().to_string();
    recent_rollouts(root, floor)
        .into_iter()
        .filter(|(_, path)| rollout_cwd(path).is_some_and(|cwd| same_folder(&cwd, &wanted)))
        .max_by_key(|(time, _)| *time)
        .map(|(_, path)| path)
}

/// A stable id for a line that carries none: its time plus a short hash of the line.
fn line_id(at: &str, line: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line.hash(&mut hasher);
    format!("{at}-{:08x}", hasher.finish() as u32)
}

/// A few words on a tool call, for the "working:" line: the shell command, or the tool's name.
fn work_text(payload: &Value) -> Option<String> {
    let kind = payload.get("type").and_then(Value::as_str)?;
    let name = payload.get("name").and_then(Value::as_str);
    let detail = match kind {
        "function_call" => {
            let args: Value = payload.get("arguments").and_then(Value::as_str).and_then(|a| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
            match args.get("command") {
                Some(Value::Array(parts)) => Some(parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")),
                Some(Value::String(text)) => Some(text.clone()),
                _ => ["path", "file_path", "query", "prompt"].iter().find_map(|k| args.get(*k).and_then(Value::as_str).map(str::to_string)),
            }
        }
        "local_shell_call" => payload
            .get("action")
            .and_then(|a| a.get("command"))
            .and_then(Value::as_array)
            .map(|parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")),
        "custom_tool_call" => payload.get("input").and_then(Value::as_str).and_then(|input| {
            // apply_patch: name the first file it touches
            input.lines().find_map(|l| ["*** Add File: ", "*** Update File: ", "*** Delete File: "].iter().find_map(|p| l.strip_prefix(p)).map(str::to_string))
        }),
        "web_search_call" => payload.get("action").and_then(|a| a.get("query")).and_then(Value::as_str).map(str::to_string),
        _ => return None,
    };
    let label = name.unwrap_or(match kind {
        "local_shell_call" => "shell",
        "web_search_call" => "web search",
        _ => "a tool",
    });
    Some(match detail.map(|d| short(&d, WORK_DETAIL_LIMIT)).filter(|d| !d.is_empty()) {
        Some(detail) => format!("{label}: {detail}"),
        None => label.to_string(),
    })
}

/// Turns one rollout line into the chat entries it holds (often none). The member's and the
/// companion's words come from `event_msg` lines (the same words also appear as `response_item`
/// messages, which are skipped so nothing shows twice); tool calls come from `response_item`.
pub fn parse_line(line: &str) -> Vec<ChatEntry> {
    let Ok(record) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let at = record.get("timestamp").and_then(Value::as_str).map(str::to_string);
    let Some(payload) = record.get("payload") else {
        return Vec::new();
    };
    let sub = payload.get("type").and_then(Value::as_str).unwrap_or("");
    let entry = |role: &str, text: String| ChatEntry {
        id: line_id(at.as_deref().unwrap_or(""), line),
        role: role.to_string(),
        text,
        at: at.clone(),
        path: None,
        pictures: Vec::new(),
        state: None,
    };
    match (record.get("type").and_then(Value::as_str).unwrap_or(""), sub) {
        ("event_msg", "user_message") => {
            let text = payload.get("message").and_then(Value::as_str).unwrap_or("").trim();
            // Instructions and environment wrappers arrive as tags; they are not the member's words.
            if text.is_empty() || text.starts_with('<') {
                Vec::new()
            } else {
                vec![entry("member", text.to_string())]
            }
        }
        ("event_msg", "agent_message") => {
            let text = strip_insight(payload.get("message").and_then(Value::as_str).unwrap_or(""));
            if text.is_empty() {
                Vec::new()
            } else {
                vec![entry("companion", text)]
            }
        }
        ("response_item", _) => work_text(payload).map(|text| vec![entry("work", text)]).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The paths in a rollout line that may name a file the companion made: tool arguments, tool
/// output, patch inputs and its own words. Reading a file is not making one, so a plain
/// `cat` / `type` / `Get-Content` of a picture is left out by the media check that follows
/// (only files written since the session started are ever shown).
pub fn media_candidates(line: &str) -> Vec<String> {
    let Ok(record) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let Some(payload) = record.get("payload") else {
        return Vec::new();
    };
    let mut texts: Vec<String> = Vec::new();
    for key in ["arguments", "output", "input", "message"] {
        match payload.get(key) {
            Some(Value::String(text)) => texts.push(text.clone()),
            Some(other @ Value::Object(_)) => texts.push(other.to_string()),
            _ => {}
        }
    }
    let mut found: Vec<String> = texts.iter().flat_map(|t| media_paths(&t.replace("\\\\", "\\"))).collect();
    let mut seen = std::collections::HashSet::new();
    found.retain(|p| seen.insert(p.to_lowercase()));
    found
}

/// A follower for one Codex rollout.
pub fn follower(path: PathBuf, since: SystemTime) -> Follower {
    Follower::new(path, since, parse_line, media_candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    const META: &str = r#"{"timestamp":"2026-11-09T04:00:00.000Z","type":"session_meta","payload":{"id":"abc","timestamp":"2026-11-09T04:00:00.000Z","cwd":"D:\\Users\\Rin\\My MemoryCore","originator":"codex_cli_rs","cli_version":"0.50.0"}}"#;

    #[test]
    fn words_come_from_event_messages_and_wrappers_are_skipped() {
        let member = r#"{"timestamp":"2026-11-09T04:00:01.000Z","type":"event_msg","payload":{"type":"user_message","message":"hi Lyra","kind":"plain"}}"#;
        let wrapped = r#"{"timestamp":"t","type":"event_msg","payload":{"type":"user_message","message":"<environment_context>cwd</environment_context>"}}"#;
        let companion = r#"{"timestamp":"2026-11-09T04:00:02.000Z","type":"event_msg","payload":{"type":"agent_message","message":"Lyra is here."}}"#;
        let duplicate = r#"{"timestamp":"t","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Lyra is here."}]}}"#;
        assert_eq!(parse_line(member)[0].role, "member");
        assert_eq!(parse_line(member)[0].text, "hi Lyra");
        assert!(parse_line(wrapped).is_empty());
        assert_eq!(parse_line(companion)[0].role, "companion");
        assert!(parse_line(duplicate).is_empty(), "the response_item copy of a message is skipped");
        assert!(parse_line(META).is_empty());
        assert!(parse_line("not json").is_empty());
        // ids are stable and differ between lines
        assert_eq!(parse_line(member)[0].id, parse_line(member)[0].id);
        assert_ne!(parse_line(member)[0].id, parse_line(companion)[0].id);
    }

    #[test]
    fn tool_calls_become_work_lines() {
        let shell = r#"{"timestamp":"t","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"powershell\",\"-Command\",\"Get-ChildItem daily-diary\"]}","call_id":"c1"}}"#;
        let patch = r#"{"timestamp":"t","type":"response_item","payload":{"type":"custom_tool_call","name":"apply_patch","input":"*** Begin Patch\n*** Add File: daily-diary/current/2026-11-09.md\n+hello\n*** End Patch"}}"#;
        let output = r#"{"timestamp":"t","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"done"}}"#;
        let reasoning = r#"{"timestamp":"t","type":"response_item","payload":{"type":"reasoning","summary":[]}}"#;
        assert_eq!(parse_line(shell)[0].text, "shell: powershell -Command Get-ChildItem daily-diary");
        assert_eq!(parse_line(shell)[0].role, "work");
        assert_eq!(parse_line(patch)[0].text, "apply_patch: daily-diary/current/2026-11-09.md");
        assert!(parse_line(output).is_empty());
        assert!(parse_line(reasoning).is_empty());
    }

    #[test]
    fn media_paths_are_found_in_tool_arguments_and_output() {
        let call = r#"{"timestamp":"t","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"powershell\",\"-File\",\"castle-image.ps1\",\"-OutFile\",\"D:\\\\Memory\\\\media-generation\\\\image-generation\\\\sky.png\"]}"}}"#;
        let out = r#"{"timestamp":"t","type":"response_item","payload":{"type":"function_call_output","output":"saved D:\\Memory\\appearance\\portrait.png"}}"#;
        assert_eq!(media_candidates(call), ["D:\\Memory\\media-generation\\image-generation\\sky.png"]);
        assert_eq!(media_candidates(out), ["D:\\Memory\\appearance\\portrait.png"]);
    }

    #[test]
    fn the_newest_rollout_for_the_folder_is_chosen() {
        let root = std::env::temp_dir().join(format!("castle-codex-{}", std::process::id()));
        let day = root.join("2026").join("11").join("09");
        std::fs::create_dir_all(&day).unwrap();
        let mine = day.join("rollout-2026-11-09T04-00-00-abc.jsonl");
        let other = day.join("rollout-2026-11-09T04-01-00-def.jsonl");
        std::fs::write(&mine, format!("{META}\n")).unwrap();
        std::fs::write(&other, META.replace("My MemoryCore", "Elsewhere") + "\n").unwrap();
        std::fs::write(day.join("notes.txt"), "x").unwrap();
        let since = SystemTime::now() - Duration::from_secs(5);
        assert_eq!(newest_record_in(&root, Path::new("d:/users/rin/my memorycore/"), since), Some(mine.clone()));
        assert_eq!(rollout_cwd(&mine).as_deref(), Some("D:\\Users\\Rin\\My MemoryCore"));
        // a session that started later than every file finds nothing
        assert_eq!(newest_record_in(&root, Path::new("D:\\Users\\Rin\\My MemoryCore"), SystemTime::now() + Duration::from_secs(3600)), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_follower_reads_a_growing_rollout() {
        let root = std::env::temp_dir().join(format!("castle-codex-follow-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("rollout-x.jsonl");
        let member = r#"{"timestamp":"a","type":"event_msg","payload":{"type":"user_message","message":"save diary"}}"#;
        std::fs::write(&path, format!("{META}\n{member}\n")).unwrap();
        let mut follower = follower(path.clone(), SystemTime::now() - Duration::from_secs(5));
        let first = follower.read_new().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].text, "save diary");
        let reply = r#"{"timestamp":"b","type":"event_msg","payload":{"type":"agent_message","message":"Diary saved."}}"#;
        std::fs::write(&path, format!("{META}\n{member}\n{reply}\n")).unwrap();
        let second = follower.read_new().unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].role, "companion");
        let _ = std::fs::remove_dir_all(root);
    }
}
