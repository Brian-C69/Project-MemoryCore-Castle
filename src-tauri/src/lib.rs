//! MemoryCore Castle: a full-screen desktop that hosts the member's own AI CLI
//! (Claude Code or Codex) inside their MemoryCore folder.

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("MemoryCore Castle could not start");
}
