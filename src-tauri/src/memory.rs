//! Reading the member's MemoryCore folder: which features are installed. MemoryCore features
//! create their own folders or skills when installed, so the files themselves are the answer
//! (the repo's Feature/ folders say nothing - installs often delete them).

use serde::Serialize;
use std::path::Path;

/// The MemoryCore features Castle knows how to show, as found in one folder.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    /// daily-diary/ (MemoryCore's diary; always there in a fresh MemoryCore)
    pub diary: bool,
    /// projects/ (LRU Projects)
    pub projects: bool,
    /// library/ (Library)
    pub library: bool,
    /// a save-memory skill from the Skill Plugin System
    pub save_memory: bool,
    /// main/reminders.md (Reminders)
    pub reminders: bool,
    /// adventures/ (Interactive Story; Castle's STORY room creates it when missing)
    pub adventures: bool,
    /// music/ (Song Creation)
    pub music: bool,
    /// media-generation/ (Image / Video Generation)
    pub media: bool,
    /// appearance/ (Castle's APPEARANCE room)
    pub appearance: bool,
}

/// Whether any plugin in `plugins/` carries a skill folder named `skill` with a SKILL.md.
fn has_skill(folder: &Path, skill: &str) -> bool {
    let Ok(plugins) = std::fs::read_dir(folder.join("plugins")) else {
        return false;
    };
    plugins.flatten().any(|plugin| plugin.path().join("skills").join(skill).join("SKILL.md").is_file())
}

pub fn features(folder: &Path) -> Features {
    let dir = |name: &str| folder.join(name).is_dir();
    Features {
        diary: dir("daily-diary"),
        projects: dir("projects"),
        library: dir("library"),
        save_memory: has_skill(folder, "save-memory"),
        reminders: folder.join("main").join("reminders.md").is_file(),
        adventures: dir("adventures"),
        music: dir("music"),
        media: dir("media-generation"),
        appearance: dir("appearance"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_are_found_by_what_they_create() {
        let root = std::env::temp_dir().join(format!("castle-features-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("daily-diary")).unwrap();
        assert_eq!(features(&root), Features { diary: true, ..Features::default() });

        std::fs::create_dir_all(root.join("projects").join("active")).unwrap();
        std::fs::create_dir_all(root.join("main")).unwrap();
        std::fs::write(root.join("main").join("reminders.md"), "## Open\n").unwrap();
        let skill = root.join("plugins").join("rin-skills").join("skills").join("save-memory");
        std::fs::create_dir_all(&skill).unwrap();
        // a skill folder without its SKILL.md is not installed
        assert!(!features(&root).save_memory);
        std::fs::write(skill.join("SKILL.md"), "---\nname: save-memory\n---\n").unwrap();
        let found = features(&root);
        assert!(found.projects && found.reminders && found.save_memory);
        assert!(!found.library && !found.music);
        let _ = std::fs::remove_dir_all(root);
    }
}
