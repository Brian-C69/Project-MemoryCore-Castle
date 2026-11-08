//! The small commands the page uses for the desktop itself: gauges, apps, clipboard, themes.

use crate::settings::SettingsStore;
use crate::{apps, clipboard, files, system, themes};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

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
pub async fn clipboard_paste(member: State<'_, crate::chat::MemberPictures>) -> Result<clipboard::Pasted, String> {
    let pasted = tauri::async_runtime::spawn_blocking(clipboard::read)
        .await
        .map_err(|_| "The clipboard could not be read.".to_string())??;
    // A pasted picture, or copied picture files: Chat may show them before they are sent.
    if pasted.kind == "picture" || pasted.kind == "files" {
        let mut allowed = member.0.lock().unwrap();
        for path in crate::transcript::media_paths(&pasted.text) {
            if crate::transcript::media_kind(&path) == Some("picture") {
                allowed.insert(path);
            }
        }
    }
    Ok(pasted)
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

/// Hides Castle to the tray (the top bar's minus button).
#[tauri::command]
pub fn hide_castle(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Opens a file from the memory folder in its own app, or its folder with the file selected.
/// Only files inside the memory folder may be opened.
#[tauri::command]
pub fn media_open(path: String, folder: bool, store: State<SettingsStore>) -> Result<(), String> {
    let memory = store.get().memory_folder.map(PathBuf::from).ok_or("No MemoryCore folder is chosen yet.")?;
    let file = files::fenced(&memory, &path)?;
    let mut command = std::process::Command::new("explorer.exe");
    if folder {
        command.arg(format!("/select,{}", file.display()));
    } else {
        command.arg(&file);
    }
    command.spawn().map(|_| ()).map_err(|_| "Windows could not open it.".to_string())
}

/// Which MemoryCore features the member's folder holds (rooms and quick actions follow it).
#[tauri::command]
pub fn memory_features(store: State<SettingsStore>) -> crate::memory::Features {
    match store.get().memory_folder {
        Some(folder) => crate::memory::features(std::path::Path::new(&folder)),
        None => crate::memory::Features::default(),
    }
}
