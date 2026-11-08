//! The small commands the page uses for the desktop itself: gauges, apps, clipboard, themes.

use crate::settings::SettingsStore;
use crate::{apps, clipboard, system, themes};
use tauri::State;

/// The two gauges: processor and memory in use on this computer, in percent.
#[tauri::command]
pub fn system_meters(meters: State<system::SystemMeters>) -> system::Meters {
    meters.read()
}

#[tauri::command]
pub fn apps_list(store: State<SettingsStore>) -> Vec<apps::App> {
    apps::list(&store.folder(), &apps::start_menu_folders())
}

/// Draws the icons not cached yet; returns how many were drawn.
#[tauri::command]
pub async fn apps_draw_icons(store: State<'_, SettingsStore>) -> Result<usize, String> {
    let folder = store.folder();
    tauri::async_runtime::spawn_blocking(move || apps::draw_missing_icons(&folder, &apps::start_menu_folders()))
        .await
        .map_err(|_| "The icons could not be drawn.".to_string())
}

/// Opens a Start-menu app (only one that is really in the Start menu) and counts the use.
#[tauri::command]
pub fn app_open(path: String, store: State<SettingsStore>) -> Result<(), String> {
    let known = apps::scan(&apps::start_menu_folders())
        .into_iter()
        .find(|(_, shortcut)| shortcut.to_string_lossy() == path);
    let Some((name, shortcut)) = known else {
        return Err("That app is not in the Start menu.".into());
    };
    apps::open(&shortcut)?;
    let _ = apps::count_use(&store.folder(), &name);
    Ok(())
}

/// What a paste puts into the terminal: copied files' paths, text, or a saved picture's path.
#[tauri::command]
pub async fn clipboard_paste() -> Result<clipboard::Pasted, String> {
    tauri::async_runtime::spawn_blocking(clipboard::read)
        .await
        .map_err(|_| "The clipboard could not be read.".to_string())?
}

#[tauri::command]
pub fn clipboard_copy(text: String) -> Result<(), String> {
    clipboard::write_text(&text)
}

/// The themes the member dropped into Castle's settings folder, each with its picture or its reason.
#[tauri::command]
pub fn user_themes(store: State<SettingsStore>) -> Vec<themes::UserTheme> {
    themes::list(&store.folder())
}
