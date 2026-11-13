//! The DEV tab's Source panel, as commands: each takes the tab's slot, finds that tab's project
//! folder in the settings and runs one git step there (see `git.rs`). Git work runs off the
//! window's thread, so the window never waits on it.

use crate::git;
use crate::settings::{Settings, SettingsStore};
use std::path::{Path, PathBuf};
use tauri::State;

/// The project folder open in a DEV tab; refused when the tab is empty or not there.
fn project_folder(settings: &Settings, slot: usize) -> Result<PathBuf, String> {
    settings
        .dev_tabs
        .get(slot)
        .cloned()
        .flatten()
        .map(PathBuf::from)
        .ok_or_else(|| "That DEV tab has no project open.".to_string())
}

/// Runs one git step in a tab's project folder, off the window's thread.
async fn in_project<T: Send + 'static>(
    store: &SettingsStore,
    slot: usize,
    step: impl FnOnce(&Path) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let folder = project_folder(&store.get(), slot)?;
    tauri::async_runtime::spawn_blocking(move || step(&folder))
        .await
        .map_err(|_| "git could not run.".to_string())?
}

/// The branch, its upstream, how far ahead or behind, and the changed files.
#[tauri::command]
pub async fn dev_git_status(slot: usize, store: State<'_, SettingsStore>) -> Result<git::Status, String> {
    in_project(&store, slot, git::status).await
}

/// One file's diff: in the working tree, or inside one commit when `commit` is given.
#[tauri::command]
pub async fn dev_git_diff(slot: usize, path: String, commit: Option<String>, store: State<'_, SettingsStore>) -> Result<String, String> {
    in_project(&store, slot, move |folder| git::diff(folder, &path, commit.as_deref())).await
}

/// One page of the commit log (50 commits, newest first) after `skip`.
#[tauri::command]
pub async fn dev_git_log(slot: usize, skip: u32, store: State<'_, SettingsStore>) -> Result<Vec<git::Commit>, String> {
    in_project(&store, slot, move |folder| git::log(folder, skip)).await
}

/// Everything one commit's card shows.
#[tauri::command]
pub async fn dev_git_show(slot: usize, hash: String, store: State<'_, SettingsStore>) -> Result<git::CommitDetail, String> {
    in_project(&store, slot, move |folder| git::show(folder, &hash)).await
}

/// The member's commit: only the ticked files, with the message, in their own git name.
#[tauri::command]
pub async fn dev_git_commit(slot: usize, paths: Vec<String>, message: String, store: State<'_, SettingsStore>) -> Result<String, String> {
    in_project(&store, slot, move |folder| git::commit(folder, &paths, &message)).await
}

/// The member's push, after the screen's one question; never forced.
#[tauri::command]
pub async fn dev_git_push(slot: usize, store: State<'_, SettingsStore>) -> Result<String, String> {
    in_project(&store, slot, git::push).await
}

/// `https://github.com/<owner>/<repo>/commit/` when the project's origin is on GitHub, else None.
#[tauri::command]
pub async fn dev_git_remote(slot: usize, store: State<'_, SettingsStore>) -> Result<Option<String>, String> {
    in_project(&store, slot, |folder| Ok(git::remote_commit_base(folder))).await
}

/// Opens a GitHub page or a server on this PC in the member's browser; any other link is refused.
#[tauri::command]
pub fn dev_open_link(url: String) -> Result<(), String> {
    if !git::is_openable_link(&url) {
        return Err("Only a GitHub page or a server on this computer opens from here.".into());
    }
    // The same way open_url hands a link to the browser (explorer.exe drops links with "&").
    std::process::Command::new("rundll32.exe")
        .arg("url.dll,FileProtocolHandler")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|_| "The browser could not be opened.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_without_a_project_is_refused() {
        let settings = Settings { dev_tabs: vec![None, Some("D:\\Work\\Shop".into())], ..Settings::default() };
        assert_eq!(project_folder(&settings, 1), Ok(PathBuf::from("D:\\Work\\Shop")));
        assert!(project_folder(&settings, 0).is_err(), "an empty slot");
        assert!(project_folder(&settings, 7).is_err(), "a slot past the list");
    }

    #[test]
    fn a_strange_link_never_reaches_the_browser() {
        assert!(dev_open_link("file:///C:/Windows/System32/calc.exe".into()).is_err());
        assert!(dev_open_link("https://example.com/".into()).is_err());
    }
}
