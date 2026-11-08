//! MemoryCore Castle: a full-screen desktop that hosts the member's own AI CLI
//! (Claude Code or Codex) inside their MemoryCore folder.

mod screen;
mod startup;
mod tray;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Whether any companion session is running (asked before quitting). Sessions arrive in Phase 2.
pub fn sessions_running(_app: &AppHandle) -> bool {
    false
}

/// Whether this launch came from the Windows sign-in entry.
pub struct Launch {
    pub at_sign_in: bool,
}

pub fn run() {
    tauri::Builder::default()
        // Must come first: a second launch only brings the running Castle forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![startup::SIGN_IN_ARG]),
        ))
        .setup(|app| {
            app.manage(Launch { at_sign_in: startup::is_sign_in_launch(std::env::args()) });

            tray::install(app.handle())?;

            let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyM);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        if shortcut == &toggle && event.state() == ShortcutState::Pressed {
                            tray::toggle_main(app);
                        }
                    })
                    .build(),
            )?;
            // Another app may already hold the keys; the tray still brings Castle back.
            if let Err(error) = app.global_shortcut().register(toggle) {
                eprintln!("MemoryCore Castle: the hotkey could not be registered: {error}");
            }

            if let Some(window) = app.get_webview_window("main") {
                screen::fill_desktop(&window)?;
                window.show()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides Castle to the tray; Quit in the tray ends it.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("MemoryCore Castle could not start")
        .run(|_app, event| {
            if let RunEvent::Exit = event {
                // Phase 2: the companion's sessions end with Castle.
            }
        });
}
