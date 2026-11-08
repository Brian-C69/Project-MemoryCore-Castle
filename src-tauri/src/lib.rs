//! MemoryCore Castle: a full-screen desktop that hosts the member's own AI CLI
//! (Claude Code or Codex) inside their MemoryCore folder.

mod key;
mod screen;
mod settings;
mod startup;
mod tray;

use serde::Serialize;
use settings::{Settings, SettingsStore};
use tauri::{AppHandle, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Whether any companion session is running (asked before quitting). Sessions arrive in Task 2.2.
pub fn sessions_running(_app: &AppHandle) -> bool {
    false
}

/// Whether this launch came from the Windows sign-in entry.
pub struct Launch {
    pub at_sign_in: bool,
}

/// What the page reads about the settings.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    settings: Settings,
    companion_label: String,
    cli_label: &'static str,
    warning: Option<String>,
    startup_note: Option<String>,
}

fn view(store: &SettingsStore) -> SettingsView {
    let settings = store.get();
    SettingsView {
        companion_label: settings.companion_label(),
        cli_label: settings.cli.label(),
        warning: store.warning.lock().unwrap().clone(),
        startup_note: store.startup_note.lock().unwrap().clone(),
        settings,
    }
}

#[tauri::command]
fn get_settings(store: State<SettingsStore>) -> SettingsView {
    view(&store)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings, store: State<SettingsStore>) -> Result<SettingsView, String> {
    let settings = settings.normalized();
    if let Some(problem) = settings.problem() {
        return Err(problem);
    }
    settings::save(&store.path, &settings).map_err(|_| "The settings could not be saved.".to_string())?;
    *store.startup_note.lock().unwrap() = startup::sync(&app, settings.start_with_windows);
    *store.current.lock().unwrap() = settings;
    *store.warning.lock().unwrap() = None;
    Ok(view(&store))
}

/// Opens the folder picker; None when the member closes it without choosing.
#[tauri::command]
async fn choose_folder(app: AppHandle, title: Option<String>) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title.unwrap_or_else(|| "Choose your MemoryCore folder".into()))
            .blocking_pick_folder()
            .and_then(|folder| folder.into_path().ok())
            .map(|path| path.display().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// Every key Castle can keep, masked.
#[tauri::command]
fn key_status() -> Vec<key::KeyStatus> {
    key::statuses()
}

#[tauri::command]
fn key_save(name: String, value: String) -> Result<Vec<key::KeyStatus>, String> {
    key::save(&name, &value)?;
    Ok(key::statuses())
}

#[tauri::command]
fn key_forget(name: String) -> Result<Vec<key::KeyStatus>, String> {
    key::forget(&name)?;
    Ok(key::statuses())
}

/// The whole key, only when the member presses Show in Settings.
#[tauri::command]
fn key_reveal(name: String) -> Option<String> {
    key::load(&name)
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
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            choose_folder,
            key_status,
            key_save,
            key_forget,
            key_reveal
        ])
        .setup(|app| {
            let config = match settings::test_override(settings::TEST_CONFIG_VAR) {
                Some(folder) => folder,
                None => app.path().app_config_dir()?,
            };
            let store = SettingsStore::open(&config);
            let wanted = store.get().start_with_windows;
            *store.startup_note.lock().unwrap() = startup::sync(app.handle(), wanted);
            app.manage(store);
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
                // Task 2.2: the companion's sessions end with Castle.
            }
        });
}
