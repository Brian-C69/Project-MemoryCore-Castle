//! WORLD on the other screens: one frameless window per chosen monitor, filling it, showing the
//! 3D isles or the 2D slideshow. Windows are created only from async commands (a window built
//! from a sync command deadlocks on Windows), and they hide and show with Castle.

use crate::screen;
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const PREFIX: &str = "world-";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// the screen Castle itself is on
    pub is_main: bool,
    /// a WORLD window is open on it
    pub open: bool,
}

fn label(index: usize) -> String {
    format!("{PREFIX}{index}")
}

#[tauri::command]
pub fn world_screens(app: AppHandle) -> Vec<ScreenInfo> {
    let main_monitor = app.get_webview_window("main").and_then(|w| w.current_monitor().ok().flatten()).map(|m| m.position().clone());
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, m)| ScreenInfo {
            index,
            name: m.name().cloned().unwrap_or_else(|| format!("Screen {}", index + 1)),
            width: m.size().width,
            height: m.size().height,
            is_main: main_monitor.as_ref().is_some_and(|p| p == m.position()),
            open: app.get_webview_window(&label(index)).is_some(),
        })
        .collect()
}

/// Opens (or brings back) the WORLD on one screen. `mode` is "3d" or "2d".
#[tauri::command]
pub async fn world_open(app: AppHandle, index: usize, mode: String) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|_| "The screens could not be read.".to_string())?;
    let monitor = monitors.get(index).ok_or("That screen is not connected.")?.clone();
    let name = label(index);
    if let Some(existing) = app.get_webview_window(&name) {
        let _ = existing.eval(&format!("window.dispatchEvent(new CustomEvent('castle-world-mode', {{ detail: {:?} }}))", mode));
        let _ = existing.show();
        return Ok(());
    }
    let mode = if mode == "2d" { "2d" } else { "3d" };
    let window = WebviewWindowBuilder::new(&app, &name, WebviewUrl::App("index.html".into()))
        .title("MemoryCore Castle - World")
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .initialization_script(&format!("window.__CASTLE_VIEW__ = {{ view: 'world', mode: '{mode}', screen: {index} }};"))
        .build()
        .map_err(|_| "The world window could not be opened.".to_string())?;
    screen::fill_monitor(&window, &monitor).map_err(|_| "The world window could not be placed.".to_string())?;
    window.show().map_err(|_| "The world window could not be shown.".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn world_close(app: AppHandle, index: Option<usize>) {
    for (name, window) in app.webview_windows() {
        if name.starts_with(PREFIX) && index.is_none_or(|i| name == label(i)) {
            let _ = window.close();
        }
    }
}

/// The world windows hide and show with Castle (the tray, the hotkey, the hide button).
pub fn set_visible(app: &AppHandle, visible: bool) {
    for (name, window) in app.webview_windows() {
        if name.starts_with(PREFIX) {
            let _ = if visible { window.show() } else { window.hide() };
        }
    }
}
