//! MemoryCore Castle: a full-screen desktop that hosts the member's own AI CLI
//! (Claude Code or Codex) inside their MemoryCore folder.

mod appearance;
mod apps;
mod backup;
mod chat;
mod chatroom;
mod clipboard;
mod commands;
mod env;
mod files;
mod key;
mod memory;
mod pty;
mod quota;
mod roomread;
mod rooms;
mod roomturn;
mod signin;
mod screen;
mod session;
mod settings;
mod setup;
mod startup;
mod story;
mod studio;
mod system;
mod theme;
mod themes;
mod transcript;
mod tray;
mod update;
mod world;
mod worlds;

use serde::Serialize;
use settings::{Settings, SettingsStore};
use tauri::{AppHandle, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Whether any companion session is running (asked before quitting).
pub fn sessions_running(app: &AppHandle) -> bool {
    session::any_running(app)
}

/// Whether this launch came from the Windows sign-in entry.
pub struct Launch {
    pub at_sign_in: bool,
}

/// The one automatic start of this run; taken once by the screen.
pub struct AutoWake(pub std::sync::Mutex<bool>);

/// True once, when this run should start the companion's session by itself; false every time after.
#[tauri::command]
fn take_auto_wake(auto_wake: State<AutoWake>) -> bool {
    std::mem::replace(&mut *auto_wake.0.lock().unwrap(), false)
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
    settings::save(&store.path, &settings).map_err(|_| "The settings could not be saved.".to_string())?;
    *store.startup_note.lock().unwrap() = startup::sync(&app, settings.start_with_windows);
    *store.current.lock().unwrap() = settings;
    *store.warning.lock().unwrap() = None;
    studio::allow_memory_folder(&app);
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

/// Tests a kept key against the service's free account check.
#[tauri::command]
async fn key_test(name: String) -> Result<key::KeyTest, String> {
    tauri::async_runtime::spawn_blocking(move || key::test(&name)).await.map_err(|_| "The check stopped.".to_string())
}

/// The whole key, only when the member presses Show in Settings.
#[tauri::command]
fn key_reveal(name: String) -> Option<String> {
    key::load(&name)
}

pub fn run() {
    let builder = tauri::Builder::default();
    // the signed self-updater lives only in official builds (scripts/release.ps1)
    #[cfg(feature = "official")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        // Must come first: a second launch only brings the running Castle forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![startup::SIGN_IN_ARG]),
        ))
        .manage(session::MainHost::default())
        .manage(session::StoryHost::default())
        .manage(system::SystemMeters::default())
        .manage(chat::MainChat::default())
        .manage(chat::StoryChat::default())
        .manage(chat::MemberPictures::default())
        .manage(roomread::RoomReader::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            choose_folder,
            key_status,
            key_save,
            key_forget,
            key_reveal,
            key_test,
            session::terminal_state,
            session::terminal_start,
            session::terminal_write,
            session::terminal_resize,
            session::story_close,
            commands::system_meters,
            commands::apps_list,
            commands::apps_draw_icons,
            commands::app_open,
            commands::clipboard_paste,
            commands::clipboard_copy,
            commands::user_themes,
            commands::hide_castle,
            commands::media_open,
            commands::memory_features,
            memory::memory_home,
            memory::memory_shelves,
            memory::memory_read,
            theme::theme_create,
            theme::theme_delete,
            theme::theme_folder,
            studio::studio_library,
            studio::studio_powers,
            appearance::appearance_look,
            appearance::appearance_new,
            appearance::appearance_wear,
            story::story_prepare,
            story::story_worlds,
            world::world_screens,
            world::world_apply,
            world::world_identify,
            quota::cli_quota,
            rooms::rooms_apply,
            signin::member_sign_in,
            signin::member_status,
            signin::member_sign_out,
            chatroom::room_companion_in,
            chatroom::room_mine,
            chatroom::room_seen,
            chatroom::room_avatar,
            chatroom::room_face_set,
            roomread::room_visible,
            roomread::room_backlog,
            roomread::room_reset,
            worlds::worlds_mine,
            worlds::world_new,
            worlds::world_delete,
            worlds::world_prepare,
            chatroom::room_me,
            chatroom::room_lines,
            chatroom::room_presence,
            chatroom::room_post,
            chatroom::room_file,
            chatroom::room_rules,
            update::update_check,
            update::update_install,
            update::diary_stamp,
            commands::choose_picture,
            commands::avatar_set,
            commands::avatar_get,
            commands::avatar_clear,
            setup::setup_inspect,
            setup::setup_create,
            setup::setup_check_cli,
            setup::setup_apply,
            setup::setup_found_keys,
            setup::setup_import_keys,
            backup::backup_status,
            backup::backup_create,
            backup::backup_now,
            backup::backup_sign_in,
            commands::open_url,
            take_auto_wake,
            chat::chat_poll,
            chat::chat_media,
            chat::chat_attach
        ])
        .setup(|app| {
            let config = match settings::test_override(settings::TEST_CONFIG_VAR) {
                Some(folder) => folder,
                None => app.path().app_config_dir()?,
            };
            let store = SettingsStore::open(&config);
            let current = store.get();
            *store.startup_note.lock().unwrap() = startup::sync(app.handle(), current.start_with_windows);
            app.manage(store);
            let at_sign_in = startup::is_sign_in_launch(std::env::args());
            app.manage(Launch { at_sign_in });
            app.manage(AutoWake(std::sync::Mutex::new(startup::should_auto_wake(
                at_sign_in,
                current.start_session_on_launch,
            ))));

            // Pasted pictures are only needed for a while; yesterday's go.
            clipboard::clear_old(&clipboard::paste_folder(), clipboard::PASTES_KEPT_FOR);

            tray::install(app.handle())?;
            backup::start_diary_watcher(app.handle());
            studio::start_notice_watcher(app.handle());
            studio::allow_memory_folder(app.handle());

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
            world::restore(app.handle());
            world::watch_covered(app.handle());
            roomread::start(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides Castle to the tray; Quit in the tray ends it.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    world::set_visible(window.app_handle(), false);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("MemoryCore Castle could not start")
        .run(|app, event| {
            // The companion's sessions belong to Castle: when Castle ends, so do they.
            if let RunEvent::Exit = event {
                session::end_all(app);
            }
        });
}
