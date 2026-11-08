//! Claude Code's record: one jsonl per session in %USERPROFILE%\.claude\projects\<folder name>.

use super::{media_kind, media_paths, relative_media_paths, strip_insight, work_detail, ChatEntry, Follower};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The paths in a record line that may name a file the companion made: its tool results' text, its
/// commands' arguments (a Write's file path, a shell command), the files Claude Code lists as
/// changed by a shell command, and drive-less paths read against the record's own folder.
pub fn media_candidates(line: &str) -> Vec<String> {
    let Ok(record) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let mut found: Vec<String> = record
        .get("toolUseResult")
        .and_then(|r| r.get("bashEditDiff"))
        .and_then(|d| d.get("changedFiles"))
        .and_then(Value::as_array)
        .map(|files| {
            files
                .iter()
                .filter_map(Value::as_str)
                .filter(|path| media_kind(path).is_some())
                .map(|path| path.replace('/', "\\"))
                .collect()
        })
        .unwrap_or_default();
    let cwd = record.get("cwd").and_then(Value::as_str).map(|c| c.trim_end_matches(['\\', '/']).replace('/', "\\"));
    let Some(Value::Array(blocks)) = record.get("message").and_then(|m| m.get("content")) else {
        return found;
    };
    let mut texts = Vec::new();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_result") => match block.get("content") {
                Some(Value::String(text)) => texts.push(text.clone()),
                Some(Value::Array(parts)) => texts.extend(
                    parts.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).map(str::to_string),
                ),
                _ => {}
            },
            Some("tool_use") => {
                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                // Reading a picture is looking, not making.
                if name == "Read" {
                    continue;
                }
                if let Some(input) = block.get("input") {
                    for key in ["file_path", "command"] {
                        if let Some(text) = input.get(key).and_then(Value::as_str) {
                            texts.push(text.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for text in &texts {
        found.extend(media_paths(text));
        if let Some(cwd) = &cwd {
            found.extend(relative_media_paths(text).into_iter().map(|p| format!("{cwd}\\{p}")));
        }
    }
    let mut seen = std::collections::HashSet::new();
    found.retain(|p| seen.insert(p.to_lowercase()));
    found
}

/// Claude Code's folder name for a working folder: every character that is not a letter or
/// digit becomes `-`.
pub fn project_folder_name(folder: &str) -> String {
    folder.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

/// Where Claude Code keeps the records for this computer's user.
pub fn projects_root() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".claude").join("projects"))
}

/// The newest record in `folder` written since `since` (a minute's grace for clocks).
pub fn newest_record(folder: &Path, since: SystemTime) -> Option<PathBuf> {
    let floor = since.checked_sub(Duration::from_secs(60)).unwrap_or(since);
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jsonl"))
        .filter_map(|p| p.metadata().and_then(|m| m.modified()).ok().map(|t| (t, p)))
        .filter(|(t, _)| *t >= floor)
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Turns one record line into the chat entries it holds (often none).
pub fn parse_line(line: &str) -> Vec<ChatEntry> {
    let Ok(record) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let kind = record.get("type").and_then(Value::as_str).unwrap_or("");
    if record.get("isMeta").and_then(Value::as_bool) == Some(true)
        || record.get("isSidechain").and_then(Value::as_bool) == Some(true)
    {
        return Vec::new();
    }
    let id = record.get("uuid").and_then(Value::as_str).unwrap_or("").to_string();
    let at = record.get("timestamp").and_then(Value::as_str).map(str::to_string);
    let content = record.get("message").and_then(|m| m.get("content"));
    let entry = |n: usize, role: &str, text: String| ChatEntry {
        id: format!("{id}-{n}"),
        role: role.to_string(),
        text,
        at: at.clone(),
        path: None,
        pictures: Vec::new(),
        state: None,
    };

    match kind {
        // A message sent while the companion was busy: it waits in the queue, then leaves it (taken in
        // mid-turn, or as the next turn's words).
        "queue-operation" => {
            let text = record.get("content").and_then(Value::as_str).unwrap_or("").trim().to_string();
            if text.is_empty() || text.starts_with('<') {
                return Vec::new();
            }
            let stamp = at.clone().unwrap_or_default();
            match record.get("operation").and_then(Value::as_str) {
                Some("enqueue") => vec![ChatEntry {
                    id: format!("queued-{stamp}"),
                    state: Some("queued".into()),
                    ..entry(0, "member", text)
                }],
                Some("remove") => vec![ChatEntry {
                    id: format!("receipt-{stamp}"),
                    state: Some(record.get("reason").and_then(Value::as_str).unwrap_or("removed").to_string()),
                    ..entry(0, "receipt", text)
                }],
                _ => Vec::new(),
            }
        }
        "user" => {
            if record.get("toolUseResult").is_some() {
                return Vec::new();
            }
            let text = match content {
                Some(Value::String(text)) => text.clone(),
                Some(Value::Array(blocks)) => blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            let text = text.trim();
            // Slash commands, hook and system wrappers arrive as tags; they are not the member's words.
            if text.is_empty() || text.starts_with('<') {
                Vec::new()
            } else {
                vec![entry(0, "member", text.to_string())]
            }
        }
        "assistant" => {
            let Some(Value::Array(blocks)) = content else {
                return Vec::new();
            };
            blocks
                .iter()
                .enumerate()
                .filter_map(|(n, block)| match block.get("type").and_then(Value::as_str) {
                    Some("text") => block
                        .get("text")
                        .and_then(Value::as_str)
                        .map(strip_insight)
                        .filter(|t| !t.is_empty())
                        .map(|t| entry(n, "companion", t)),
                    // A plan the companion presents becomes a plan card holding the whole plan.
                    Some("tool_use")
                        if block.get("name").and_then(Value::as_str) == Some("ExitPlanMode")
                            && block.get("input").and_then(|i| i.get("plan")).and_then(Value::as_str).is_some() =>
                    {
                        let plan = block["input"]["plan"].as_str().unwrap_or("").trim().to_string();
                        (!plan.is_empty()).then(|| entry(n, "plan", plan))
                    }
                    Some("tool_use") => {
                        let name = block.get("name").and_then(Value::as_str).unwrap_or("a tool");
                        let detail = block.get("input").and_then(work_detail);
                        let text = match detail {
                            Some(detail) => format!("{name}: {detail}"),
                            None => name.to_string(),
                        };
                        Some(entry(n, "work", text))
                    }
                    _ => None,
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// A follower for one Claude Code record.
pub fn follower(path: PathBuf, since: SystemTime) -> Follower {
    Follower::new(path, since, parse_line, media_candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::*;
    use std::io::Write;

    #[test]
    fn a_memory_folder_maps_to_claude_codes_folder_name() {
        assert_eq!(
            project_folder_name("D:\\Users\\Rin\\My MemoryCore"),
            "D--Users-Rin-My-MemoryCore"
        );
    }

    #[test]
    fn his_words_and_hers_become_entries_and_the_rest_stays_out() {
        let member_line = r#"{"type":"user","uuid":"u1","timestamp":"t","message":{"role":"user","content":"hi Lyra"}}"#;
        assert_eq!(parse_line(member_line)[0].role, "member");
        assert_eq!(parse_line(member_line)[0].text, "hi Lyra");

        let companion_line = r#"{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[
            {"type":"thinking","thinking":"private"},
            {"type":"text","text":"Good evening, Rin."},
            {"type":"tool_use","name":"Read","input":{"file_path":"D:\\Castle\\src\\lib.rs"}}]}}"#
            .replace('\n', "");
        let entries = parse_line(&companion_line);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].role, "companion");
        assert_eq!(entries[1].role, "work");
        assert_eq!(entries[1].text, "Read: D:\\Castle\\src\\lib.rs");

        let tool_result = r#"{"type":"user","uuid":"u2","toolUseResult":{},"message":{"role":"user","content":[{"type":"tool_result","content":"x"}]}}"#;
        assert!(parse_line(tool_result).is_empty());
        let meta = r#"{"type":"user","uuid":"u3","isMeta":true,"message":{"role":"user","content":"caveat"}}"#;
        assert!(parse_line(meta).is_empty());
        let command = r#"{"type":"user","uuid":"u4","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#;
        assert!(parse_line(command).is_empty());
        assert!(parse_line(r#"{"type":"attachment","uuid":"x"}"#).is_empty());
        assert!(parse_line("{ broken").is_empty());
    }

    #[test]
    fn the_follower_holds_back_a_half_written_line() {
        let folder = std::env::temp_dir().join(format!("castle-transcript-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("session.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, r#"{{"type":"user","uuid":"u1","message":{{"content":"first"}}}}"#).unwrap();
        write!(file, r#"{{"type":"user","uuid":"u2","mess"#).unwrap();
        file.flush().unwrap();

        let mut follower = super::follower(path.clone(), SystemTime::now());
        let first = follower.read_new().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].text, "first");

        writeln!(file, r#"age":{{"content":"second"}}}}"#).unwrap();
        file.flush().unwrap();
        let second = follower.read_new().unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].text, "second");
        assert!(follower.read_new().unwrap().is_empty());

        assert_eq!(newest_record(&folder, SystemTime::now()), Some(path));
        assert_eq!(newest_record(&folder, SystemTime::now() + Duration::from_secs(3600)), None);
    }

    #[test]
    fn a_presented_plan_becomes_one_plan_entry_with_the_whole_plan() {
        let line = r##"{"type":"assistant","uuid":"p1","message":{"content":[
            {"type":"text","text":"Here is the plan."},
            {"type":"tool_use","name":"ExitPlanMode","input":{"plan":"# Link Lv.20 - The Update Door\n\n### Phase 1\n- [ ] one","planFilePath":"x.md"}}]}}"##
            .replace('\n', "");
        let entries = parse_line(&line);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].role, "plan");
        assert!(entries[1].text.starts_with("# Link Lv.20 - The Update Door"));
        assert!(entries[1].text.contains("### Phase 1"));

        let empty = r#"{"type":"assistant","uuid":"p2","message":{"content":[{"type":"tool_use","name":"ExitPlanMode","input":{}}]}}"#;
        assert_eq!(parse_line(empty)[0].role, "work");
    }

    #[test]
    fn media_paths_are_found_in_wrapped_lines_commands_and_bash_paths() {
        let wrapped = "saved_path        : D:\\Users\\Rin\\Castle Work\\Project \n                    Resources\\sight\\castle-design-1.png\ncost_estimate_usd : 0.01";
        assert_eq!(
            media_paths(wrapped),
            ["D:\\Users\\Rin\\Castle Work\\Project Resources\\sight\\castle-design-1.png"]
        );
        let speak = "powershell -File castle-voice.ps1 -TextFile \"C:\\t\\line.txt\" -Lang ja -OutFile \"D:\\Memory\\voice\\day 203.mp3\"";
        assert_eq!(media_paths(speak), ["D:\\Memory\\voice\\day 203.mp3"]);
        assert_eq!(media_paths("saved to /d/My Files/art/x.webp done"), ["D:\\My Files\\art\\x.webp"]);
        assert!(media_paths("edited D:\\Castle\\src\\lib.rs and C:\\notes.txt").is_empty());
        assert_eq!(media_paths("a C:\\a.png then D:\\b.mp3"), ["C:\\a.png", "D:\\b.mp3"]);
        assert_eq!(media_kind("x.JPEG"), Some("picture"));
        assert_eq!(media_kind("x.m4a"), Some("voice"));
    }

    /// Reads a real record and lists the pictures and voices it would show (names only).
    /// Run by hand: KEEP_RECORD_FOLDER=<working folder> cargo test real_record -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_record_media() {
        let folder = std::env::var("KEEP_RECORD_FOLDER").expect("KEEP_RECORD_FOLDER");
        let records = projects_root().unwrap().join(project_folder_name(&folder));
        let since = SystemTime::now() - Duration::from_secs(24 * 3600);
        let record = newest_record(&records, since).expect("a record from the last day");
        let mut follower = super::follower(record, since);
        let media: Vec<_> = follower.read_new().unwrap().into_iter().filter(|e| e.path.is_some()).collect();
        println!("media shown: {}", media.len());
        for entry in &media {
            println!("  {} {}", entry.role, entry.text);
        }
    }

    #[test]
    fn a_made_file_is_shown_once_and_a_read_or_old_one_never() {
        let folder = std::env::temp_dir().join(format!("castle-media-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let picture = folder.join("art.png");
        let later = folder.join("voice.mp3");
        std::fs::write(&picture, b"png").unwrap();
        let record = folder.join("session.jsonl");
        let result = |path: &Path| {
            format!(
                r#"{{"type":"user","uuid":"r","toolUseResult":{{}},"message":{{"content":[{{"type":"tool_result","content":"saved_path : {}"}}]}}}}"#,
                path.display().to_string().replace('\\', "\\\\")
            )
        };
        let read = format!(
            r#"{{"type":"assistant","uuid":"a","message":{{"content":[{{"type":"tool_use","name":"Read","input":{{"file_path":"{}"}}}}]}}}}"#,
            folder.join("other.png").display().to_string().replace('\\', "\\\\")
        );
        std::fs::write(folder.join("other.png"), b"png").unwrap();
        std::fs::write(&record, format!("{}\n{}\n{}\n", result(&picture), read, result(&later))).unwrap();

        let mut follower = super::follower(record.clone(), SystemTime::now());
        let first = follower.read_new().unwrap();
        let media: Vec<_> = first.iter().filter(|e| e.path.is_some()).collect();
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].role, "picture");
        assert!(follower.was_shown(&picture.display().to_string()));
        assert!(!follower.was_shown(&folder.join("other.png").display().to_string()));

        // The voice file is finished after its line was read: it appears on a later read.
        std::fs::write(&later, b"mp3").unwrap();
        let second = follower.read_new().unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].role, "voice");

        // A file written before the session woke is only mentioned, never shown.
        let mut fresh_session = super::follower(record, SystemTime::now() + Duration::from_secs(3600));
        assert!(fresh_session.read_new().unwrap().iter().all(|e| e.path.is_none()));
    }

    #[test]
    fn the_insight_block_leaves_chat_and_her_words_stay() {
        let rule = "`─────────────────────────────────────────────────`";
        let open = "`★ Insight ─────────────────────────────────────`";
        let start = format!("{open}\n- one\n- two\n{rule}\n\nCastle is ready~");
        assert_eq!(strip_insight(&start), "Castle is ready~");
        let middle = format!("Done.\n\n{open}\n- why\n{rule}\n\nSealed at abc 💜");
        assert_eq!(strip_insight(&middle), "Done.\n\nSealed at abc 💜");
        let two = format!("{open}\na\n{rule}\nhi\n{open}\nb\n{rule}\nbye");
        assert_eq!(strip_insight(&two), "hi\nbye");
        assert_eq!(strip_insight(&format!("{open}\n- only\n{rule}")), "");
        let unclosed = format!("{open}\n- no end\nstill the companion's");
        assert_eq!(strip_insight(&unclosed), unclosed);
        assert_eq!(strip_insight("  plain words  "), "plain words");
        // A reply that is only an Insight block makes no bubble.
        let line = format!(
            r#"{{"type":"assistant","uuid":"i","message":{{"content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(&format!("{open}\n- x\n{rule}")).unwrap()
        );
        assert!(parse_line(&line).is_empty());
    }

    #[test]
    fn changed_files_and_relative_paths_name_what_she_made() {
        let changed = r#"{"type":"user","uuid":"c","cwd":"D:\\Memory","toolUseResult":{"bashEditDiff":{"changedFiles":["D:\\Memory\\daily-diary\\x-art.png","D:\\Memory\\notes.md"]}},"message":{"content":[{"type":"tool_result","content":"saved_path        : D:\\Memory\\daily-diary\\x-comm\ncost_estimate_usd : 0.01"}]}}"#;
        assert_eq!(media_candidates(changed), ["D:\\Memory\\daily-diary\\x-art.png"]);

        let spoke = r#"{"type":"user","uuid":"s","cwd":"D:\\Memory","message":{"content":[{"type":"tool_result","content":"SPOKE elevenlabs ja -> daily-diary\\current\\2026-11-07\\voice\\2047-how-i-feel-tonight.mp3 (306407 bytes)"}]}}"#;
        assert_eq!(media_candidates(spoke), ["D:\\Memory\\daily-diary\\current\\2026-11-07\\voice\\2047-how-i-feel-tonight.mp3"]);

        let joined = r#"{"type":"assistant","uuid":"j","cwd":"D:\\Memory","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"$out = Join-Path $root \"daily-diary\\current\\a.png\"; & castle-image.ps1 -OutFile $out"}}]}}"#;
        assert_eq!(media_candidates(joined), ["D:\\Memory\\daily-diary\\current\\a.png"]);

        // Without the record's folder a relative path cannot be placed; flags and full paths are not relative.
        let no_cwd = r#"{"type":"user","uuid":"n","message":{"content":[{"type":"tool_result","content":"wrote voice/x.mp3"}]}}"#;
        assert!(media_candidates(no_cwd).is_empty());
        assert!(relative_media_paths("-OutFile x.png C:\\a\\b.png /d/a/b.png").is_empty());
        assert_eq!(relative_media_paths("see ./art/x.webp."), ["art\\x.webp"]);
    }

    #[test]
    fn main_chat_leaves_the_story_rooms_pictures_to_the_story_room() {
        let memory = std::env::temp_dir().join(format!("castle-skip-{}", std::process::id()));
        let story_scene = memory.join("adventures").join("w").join("scenes");
        std::fs::create_dir_all(&story_scene).unwrap();
        let painting = story_scene.join("04-witch.png");
        let diary_art = memory.join("art.png");
        std::fs::write(&painting, b"png").unwrap();
        std::fs::write(&diary_art, b"png").unwrap();
        let record = memory.join("r.jsonl");
        let line = |p: &Path| {
            format!(
                r#"{{"type":"user","uuid":"u","toolUseResult":{{}},"message":{{"content":[{{"type":"tool_result","content":{}}}]}}}}"#,
                serde_json::to_string(&format!("saved_path : {}", p.display())).unwrap()
            )
        };
        std::fs::write(&record, format!("{}\n{}\n", line(&painting), line(&diary_art))).unwrap();

        let mut main_chat = super::follower(record.clone(), SystemTime::now() - Duration::from_secs(5)).skipping(&memory.join("adventures"));
        let shown: Vec<_> = main_chat.read_new().unwrap().into_iter().filter_map(|e| e.path).collect();
        assert_eq!(shown, [diary_art.display().to_string()]);

        let mut story_chat = super::follower(record, SystemTime::now() - Duration::from_secs(5));
        assert_eq!(story_chat.read_new().unwrap().iter().filter(|e| e.path.is_some()).count(), 2);
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn his_shared_pictures_ride_on_his_message_and_may_be_shown() {
        let folder = std::env::temp_dir().join(format!("castle-member-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let shot = folder.join("Screenshot 1.png");
        std::fs::write(&shot, b"png").unwrap();
        let text = format!("look & '{}' and this C:\\nowhere\\gone.png", shot.display());
        assert_eq!(member_pictures(&text), [shot.display().to_string()]);
        let record = folder.join("r.jsonl");
        let line = format!(
            r#"{{"type":"user","uuid":"k","message":{{"content":{}}}}}"#,
            serde_json::to_string(&text).unwrap()
        );
        std::fs::write(&record, format!("{line}\n")).unwrap();
        let mut follower = super::follower(record, SystemTime::now() + Duration::from_secs(3600));
        let entries = follower.read_new().unwrap();
        assert_eq!(entries[0].role, "member");
        assert_eq!(entries[0].pictures, [shot.display().to_string()]);
        // The member's picture is theirs even when older than the session.
        assert!(follower.was_shown(&shot.display().to_string()));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_relative_path_that_does_not_exist_is_never_shown() {
        let folder = std::env::temp_dir().join(format!("castle-relative-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("voice")).unwrap();
        std::fs::write(folder.join("voice").join("here.mp3"), b"mp3").unwrap();
        let record = folder.join("r.jsonl");
        let line = format!(
            r#"{{"type":"user","uuid":"v","cwd":{},"message":{{"content":[{{"type":"tool_result","content":"SPOKE -> voice\\here.mp3 and voice\\missing.mp3"}}]}}}}"#,
            serde_json::to_string(&folder.display().to_string()).unwrap()
        );
        std::fs::write(&record, format!("{line}\n")).unwrap();
        let mut follower = super::follower(record, SystemTime::now());
        let media: Vec<_> = follower.read_new().unwrap().into_iter().filter(|e| e.path.is_some()).collect();
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].role, "voice");
        assert!(media[0].path.as_deref().unwrap().ends_with("voice\\here.mp3"));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_message_sent_while_she_works_is_queued_then_leaves_the_queue() {
        let queued = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-11-08T13:07:40.802Z","content":"also add upload and download below ping"}"#;
        let taken = r#"{"type":"queue-operation","operation":"remove","timestamp":"2026-11-08T13:07:43.194Z","content":"also add upload and download below ping","reason":"absorbed_mid_turn"}"#;
        let folded = r#"{"type":"attachment","uuid":"a","attachment":{"type":"queued_command","prompt":"also add upload and download below ping"}}"#;
        let q = parse_line(queued);
        assert_eq!((q[0].role.as_str(), q[0].state.as_deref(), q[0].text.as_str()), ("member", Some("queued"), "also add upload and download below ping"));
        let r = parse_line(taken);
        assert_eq!((r[0].role.as_str(), r[0].state.as_deref()), ("receipt", Some("absorbed_mid_turn")));
        assert_ne!(q[0].id, r[0].id);
        assert!(parse_line(folded).is_empty(), "the folded copy is not shown twice");
        let hook = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"t","content":"<task-notification>x</task-notification>"}"#;
        assert!(parse_line(hook).is_empty());
    }
}
