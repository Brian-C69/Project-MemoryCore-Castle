//! STORY: the member's adventures, MemoryCore's Interactive Story in a room of its own. The STORY
//! session runs in <memory>/adventures with Castle's rulebook written into CLAUDE.md and AGENTS.md
//! there; worlds already in adventures/ are listed and read, whatever made them.

use crate::session::STORY_FOLDER;
use crate::settings::SettingsStore;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

const RULES: &str = include_str!("../resources/story-rules.md");
const BEGIN: &str = "<!-- castle:story:begin";
const END: &str = "<!-- castle:story:end -->";

/// `text` with the story rules put in place between their markers; everything else kept.
pub fn with_rules(text: &str, rules: &str) -> String {
    let rules = rules.trim_end();
    if let (Some(start), Some(end)) = (text.find(BEGIN), text.find(END)) {
        if end > start {
            return format!("{}{}{}", &text[..start], rules, &text[end + END.len()..]);
        }
    }
    if text.trim().is_empty() {
        format!("{rules}\n")
    } else {
        format!("{}\n\n{rules}\n", text.trim_end())
    }
}

/// Makes adventures/ (when missing) and writes the rulebook with the names into its CLAUDE.md and
/// AGENTS.md, so the STORY session plays by MemoryCore's Interactive Story rules on either CLI.
pub fn prepare(memory: &Path, companion: &str, member: &str) -> Result<PathBuf, String> {
    let folder = memory.join(STORY_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|_| "The adventures folder could not be created.".to_string())?;
    let rules = RULES.replace("{{AI_NAME}}", companion.trim()).replace("{{USER_NAME}}", member.trim());
    for name in ["CLAUDE.md", "AGENTS.md"] {
        let path = folder.join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let next = with_rules(&text, &rules);
        // only a real change is written, so opening STORY never touches a file for nothing
        if next != text {
            std::fs::write(&path, next).map_err(|_| format!("{name} could not be written in adventures."))?;
        }
    }
    Ok(folder)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub id: String,
    pub name: String,
    pub status: String,
    pub chapter: Option<String>,
    /// the story files, oldest first (relative to the memory folder)
    pub parts: Vec<String>,
    pub summary: Option<String>,
    /// scene pictures (absolute paths), newest last
    pub scenes: Vec<String>,
}

fn field(text: &str, label: &str) -> Option<String> {
    // "- **Status**: In Progress", "**Status**: Completed" or "Status: Completed"
    text.lines().find_map(|l| {
        let plain = l.replace('*', "");
        let plain = plain.trim().trim_start_matches(['-', ' ']);
        let value = plain.strip_prefix(&format!("{label}:"))?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Every world in adventures/: its name (setting.md's first heading), status and chapter.
pub fn worlds(memory: &Path) -> Vec<World> {
    let root = memory.join(STORY_FOLDER);
    let rel = |p: &Path| p.strip_prefix(memory).map(|r| r.display().to_string().replace('\\', "/")).unwrap_or_default();
    let mut found: Vec<World> = std::fs::read_dir(&root)
        .map(|e| e.flatten().map(|e| e.path()).filter(|p| p.join("setting.md").is_file()).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|dir| {
            let setting = std::fs::read_to_string(dir.join("setting.md")).unwrap_or_default();
            let id = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            let name = setting.lines().find_map(|l| l.trim().strip_prefix("# ").map(|h| h.trim().to_string())).unwrap_or_else(|| id.replace('-', " "));
            let mut parts: Vec<PathBuf> = std::fs::read_dir(dir.join("story"))
                .map(|e| e.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md")).collect())
                .unwrap_or_default();
            // story-2 before story-10
            parts.sort_by_key(|p| {
                let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                stem.rsplit('-').next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(0)
            });
            let mut scenes: Vec<PathBuf> = std::fs::read_dir(dir.join("scenes"))
                .map(|e| {
                    e.flatten()
                        .map(|e| e.path())
                        .filter(|p| matches!(p.extension().and_then(|x| x.to_str()).map(|x| x.to_lowercase()).as_deref(), Some("png" | "jpg" | "jpeg" | "webp")))
                        .collect()
                })
                .unwrap_or_default();
            scenes.sort();
            World {
                status: field(&setting, "Status").unwrap_or_else(|| "In Progress".into()),
                chapter: field(&setting, "Current Chapter"),
                summary: dir.join("summary.md").is_file().then(|| rel(&dir.join("summary.md"))),
                parts: parts.iter().map(|p| rel(p)).collect(),
                scenes: scenes.iter().map(|p| p.display().to_string()).collect(),
                id,
                name,
            }
        })
        .collect();
    // worlds in progress first, then by name
    found.sort_by(|a, b| (a.status != "In Progress").cmp(&(b.status != "In Progress")).then(a.name.cmp(&b.name)));
    found
}

#[tauri::command]
pub fn story_prepare(store: State<SettingsStore>) -> Result<String, String> {
    let s = store.get();
    let memory = s.memory_folder.clone().map(PathBuf::from).ok_or("No MemoryCore folder is chosen yet.")?;
    let member = s.member_name.clone().unwrap_or_else(|| "the member".into());
    prepare(&memory, &s.companion_label(), &member).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn story_worlds(store: State<SettingsStore>) -> Vec<World> {
    store.get().memory_folder.map(|m| worlds(Path::new(&m))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_go_in_with_the_names_and_the_members_words_stay() {
        let memory = std::env::temp_dir().join(format!("castle-story-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&memory);
        std::fs::create_dir_all(memory.join("adventures")).unwrap();
        std::fs::write(memory.join("adventures").join("CLAUDE.md"), "# my story notes\n").unwrap();
        prepare(&memory, "Lyra", "Rin").unwrap();
        let written = std::fs::metadata(memory.join("adventures").join("CLAUDE.md")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        prepare(&memory, "Lyra", "Rin").unwrap();
        // opening STORY again changes nothing, so the file is not touched
        assert_eq!(std::fs::metadata(memory.join("adventures").join("CLAUDE.md")).unwrap().modified().unwrap(), written);
        let claude = std::fs::read_to_string(memory.join("adventures").join("CLAUDE.md")).unwrap();
        assert!(claude.starts_with("# my story notes"));
        assert_eq!(claude.matches(BEGIN).count(), 1);
        assert!(claude.contains("Lyra") && !claude.contains("{{AI_NAME}}") && !claude.contains("{{USER_NAME}}"));
        assert!(memory.join("adventures").join("AGENTS.md").is_file());
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn worlds_are_listed_with_status_chapter_and_parts_in_order() {
        let memory = std::env::temp_dir().join(format!("castle-worlds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&memory);
        let world = memory.join("adventures").join("sky-kingdom");
        std::fs::create_dir_all(world.join("story")).unwrap();
        std::fs::create_dir_all(world.join("scenes")).unwrap();
        std::fs::write(world.join("setting.md"), "# The Sky Kingdom\n- **Status**: In Progress\n- **Current Chapter**: 3\n").unwrap();
        for n in [10, 2, 1] {
            std::fs::write(world.join("story").join(format!("story-{n}.md")), "x").unwrap();
        }
        std::fs::write(world.join("scenes").join("01-gate.png"), b"png").unwrap();
        let done = memory.join("adventures").join("old-tale");
        std::fs::create_dir_all(&done).unwrap();
        std::fs::write(done.join("setting.md"), "# An Old Tale\n**Status**: Completed\n").unwrap();
        let list = worlds(&memory);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "The Sky Kingdom");
        assert_eq!(list[0].chapter.as_deref(), Some("3"));
        assert_eq!(list[0].parts, ["adventures/sky-kingdom/story/story-1.md", "adventures/sky-kingdom/story/story-2.md", "adventures/sky-kingdom/story/story-10.md"]);
        assert_eq!(list[0].scenes.len(), 1);
        assert_eq!(list[1].status, "Completed");
        let _ = std::fs::remove_dir_all(memory);
    }
}
