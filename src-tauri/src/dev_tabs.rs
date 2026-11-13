//! The DEV tab's bookkeeping, kept pure so it can be tested: which tab a project opens in, the
//! recent list, and which folders are never a project (the companion's own memory).

use std::path::{Path, PathBuf};

/// At most this many projects are open at once, each in its own tab with its own session.
pub const SLOTS: usize = 8;
/// How many recent projects the list keeps.
pub const RECENT: usize = 20;

/// A folder written the way it is compared: no trailing slash, forward slashes, lower case on
/// Windows (where `D:\Shop` and `d:/shop/` are the same folder).
pub fn same_folder_key(folder: &str) -> String {
    let trimmed = folder.trim().trim_end_matches(['\\', '/']).replace('\\', "/");
    if cfg!(windows) {
        trimmed.to_lowercase()
    } else {
        trimmed
    }
}

/// The name a tab shows: the folder's last part.
pub fn name_of(folder: &str) -> String {
    Path::new(folder.trim_end_matches(['\\', '/']))
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| folder.to_string())
}

/// Why a folder cannot be a DEV project, in words for the screen; None when it can.
pub fn refusal(folder: &Path, memory: Option<&Path>) -> Option<String> {
    if !folder.is_dir() {
        return Some("That folder was not found.".into());
    }
    if let Some(memory) = memory {
        let inside = |a: &Path, b: &Path| {
            let (a, b) = (same_folder_key(&a.display().to_string()), same_folder_key(&b.display().to_string()));
            a == b || a.starts_with(&format!("{b}/"))
        };
        let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        if inside(&real(folder), &real(memory)) {
            return Some("That is your companion's memory folder - the main room works there. Choose a project folder instead.".into());
        }
    }
    None
}

/// The tab a project opens in: the tab it is already open in, else the first free one.
/// Err when all tabs are in use.
pub fn slot_for(tabs: &[Option<String>], folder: &str) -> Result<usize, String> {
    let key = same_folder_key(folder);
    if let Some(open) = tabs.iter().position(|t| t.as_deref().map(same_folder_key).as_deref() == Some(key.as_str())) {
        return Ok(open);
    }
    (0..SLOTS)
        .find(|&slot| tabs.get(slot).is_none_or(|t| t.is_none()))
        .ok_or_else(|| format!("All {SLOTS} DEV tabs are in use. Close one first."))
}

/// The tabs with `folder` placed in `slot` (the list grows to fit, never past `SLOTS`).
pub fn with_tab(tabs: &[Option<String>], slot: usize, folder: Option<String>) -> Vec<Option<String>> {
    let mut next: Vec<Option<String>> = tabs.iter().take(SLOTS).cloned().collect();
    if slot < SLOTS {
        if next.len() <= slot {
            next.resize(slot + 1, None);
        }
        next[slot] = folder;
    }
    while next.last().is_some_and(|t| t.is_none()) {
        next.pop();
    }
    next
}

/// The recent list with `folder` at the top (once), at most `RECENT` long.
pub fn remembered(recent: &[String], folder: &str) -> Vec<String> {
    let key = same_folder_key(folder);
    let mut next = vec![folder.to_string()];
    next.extend(recent.iter().filter(|f| same_folder_key(f) != key).cloned());
    next.truncate(RECENT);
    next
}

/// The recent list without `folder`.
pub fn forgotten(recent: &[String], folder: &str) -> Vec<String> {
    let key = same_folder_key(folder);
    recent.iter().filter(|f| same_folder_key(f) != key).cloned().collect()
}

/// Where a DEV tab's boot file lives: Castle's own settings folder, never the project.
pub fn boot_file(settings_folder: &Path, slot: usize) -> PathBuf {
    settings_folder.join("boot").join(format!("dev-{slot}.md"))
}

/// What the companion is told when she wakes in a project: who she is there, and how plans work.
pub fn dev_block(companion: &str, member: &str, project: &str, memory_loaded: bool) -> String {
    let memory = if memory_loaded {
        "Your memory is below - Castle read it from your database when it woke you."
    } else {
        "Your memory is not loaded in this session: run `castle-memory boot` first and follow what it prints."
    };
    format!(
        "# You are {companion}, at work\n\n\
         You are {companion}, working with {member} in the project **{project}** (this folder). {memory}\n\n\
         This is a work session: be {companion}, and keep to the project. The project's own CLAUDE.md, if it \
         has one, is below.\n\n\
         ## Plans\n\
         When {member} approves a plan, run `castle-memory plan copy` so it is kept in your memory, then walk it one \
         item at a time: `castle-memory plan next` -> do that item -> `castle-memory plan mark --next`. \
         `castle-memory plan status` shows where you are; {member} sees the same plan in Castle's Plan panel.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_reopens_in_its_own_tab_else_the_first_free_one() {
        let tabs = vec![Some("D:\\Shop".to_string()), None, Some("D:\\Blog".to_string())];
        assert_eq!(slot_for(&tabs, "D:\\Blog"), Ok(2));
        assert_eq!(slot_for(&tabs, "D:\\New"), Ok(1));
        if cfg!(windows) {
            assert_eq!(slot_for(&tabs, "d:/shop/"), Ok(0), "the same folder written another way");
        }
        let full: Vec<Option<String>> = (0..SLOTS).map(|i| Some(format!("D:\\p{i}"))).collect();
        assert!(slot_for(&full, "D:\\extra").unwrap_err().contains("in use"));
    }

    #[test]
    fn tabs_grow_and_shrink_without_holes_at_the_end() {
        let tabs = with_tab(&[], 2, Some("D:\\a".into()));
        assert_eq!(tabs, vec![None, None, Some("D:\\a".to_string())]);
        assert!(with_tab(&tabs, 2, None).is_empty());
        assert_eq!(with_tab(&[], SLOTS + 3, Some("x".into())), Vec::<Option<String>>::new());
    }

    #[test]
    fn the_recent_list_keeps_each_project_once_newest_first() {
        let list = remembered(&["D:\\a".into(), "D:\\b".into()], "D:\\b");
        assert_eq!(list, vec!["D:\\b".to_string(), "D:\\a".to_string()]);
        let long: Vec<String> = (0..RECENT + 5).map(|i| format!("D:\\p{i}")).collect();
        assert_eq!(remembered(&long, "D:\\new").len(), RECENT);
        assert_eq!(forgotten(&list, "D:\\a"), vec!["D:\\b".to_string()]);
    }

    #[test]
    fn the_memory_folder_and_anything_inside_it_is_never_a_project() {
        let root = std::env::temp_dir().join(format!("castle-dev-tabs-{}", std::process::id()));
        let memory = root.join("memory");
        let inside = memory.join("adventures");
        let project = root.join("shop");
        for f in [&inside, &project] {
            std::fs::create_dir_all(f).unwrap();
        }
        assert!(refusal(&memory, Some(&memory)).unwrap().contains("memory folder"));
        assert!(refusal(&inside, Some(&memory)).is_some());
        assert_eq!(refusal(&project, Some(&memory)), None);
        assert!(refusal(&root.join("gone"), Some(&memory)).unwrap().contains("not found"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_work_block_names_the_companion_the_project_and_the_plan_verbs() {
        let block = dev_block("Lyra", "Rin", "shop", true);
        assert!(block.contains("You are Lyra") && block.contains("**shop**") && block.contains("castle-memory plan copy"));
        assert!(dev_block("Lyra", "Rin", "shop", false).contains("castle-memory boot"));
        assert_eq!(name_of("D:\\Projects\\shop\\"), "shop");
        assert!(boot_file(Path::new("C:/castle"), 3).ends_with("boot/dev-3.md"));
    }
}
