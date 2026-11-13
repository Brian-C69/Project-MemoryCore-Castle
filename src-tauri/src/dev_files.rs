//! The DEV tab's FILES: the open project's tree, a file for the reader, and Explorer on a file.
//! Every command finds the project from the tab's slot in the settings, so the screen only ever
//! names a path inside it; `tree` holds the fences.

use crate::settings::{Settings, SettingsStore};
use crate::tree;
use std::path::PathBuf;
use tauri::State;

/// The project folder open in a DEV tab, or why there is none.
pub fn project_of(settings: &Settings, slot: usize) -> Result<PathBuf, String> {
    let folder = settings
        .dev_tabs
        .get(slot)
        .cloned()
        .flatten()
        .map(PathBuf::from)
        .ok_or_else(|| "That DEV tab has no project open.".to_string())?;
    if folder.is_dir() {
        Ok(folder)
    } else {
        Err("This project folder was not found.".into())
    }
}

/// Runs `work` on the tab's project away from the screen's thread.
async fn on_project<T: Send + 'static>(
    store: &SettingsStore,
    slot: usize,
    work: impl FnOnce(PathBuf) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let folder = project_of(&store.get(), slot)?;
    tauri::async_runtime::spawn_blocking(move || work(folder))
        .await
        .map_err(|_| "Castle could not read the project.".to_string())?
}

/// One level of the tab's project tree ("" = the top).
#[tauri::command]
pub async fn dev_tree(slot: usize, rel: String, store: State<'_, SettingsStore>) -> Result<tree::Level, String> {
    on_project(&store, slot, move |folder| tree::list(&folder, &rel)).await
}

/// A file of the tab's project for the reader: its text, its picture, or one line saying why not.
#[tauri::command]
pub async fn dev_file(slot: usize, rel: String, store: State<'_, SettingsStore>) -> Result<tree::Shown, String> {
    on_project(&store, slot, move |folder| tree::read(&folder, &rel)).await
}

/// Windows Explorer opened with the file selected (inside the project only).
#[tauri::command]
pub async fn dev_reveal(slot: usize, rel: String, store: State<'_, SettingsStore>) -> Result<(), String> {
    on_project(&store, slot, move |folder| tree::reveal(&folder, &rel)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_without_a_project_is_refused() {
        let here = std::env::temp_dir().display().to_string();
        let settings = Settings { dev_tabs: vec![Some(here.clone()), None, Some("Z:/no/such/castle/folder".into())], ..Settings::default() };
        assert_eq!(project_of(&settings, 0).unwrap(), PathBuf::from(here));
        assert!(project_of(&settings, 1).is_err());
        assert!(project_of(&settings, 2).is_err());
        assert!(project_of(&settings, 7).is_err());
    }
}
