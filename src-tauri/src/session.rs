//! Castle's two sessions: the companion's main one in the MemoryCore folder, and the STORY
//! room's in <memory>\adventures. Each is a PtyHost; the page talks to them by room name.

use crate::env;
use crate::key;
use crate::pty::{self, PtyHost};
use crate::settings::{Settings, SettingsStore};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

pub const TERMINAL_OUTPUT: &str = "terminal-output";
pub const TERMINAL_EXIT: &str = "terminal-exit";
pub const STORY_OUTPUT: &str = "story-output";
pub const STORY_EXIT: &str = "story-exit";
/// The STORY room's folder inside the memory folder (MemoryCore's Interactive Story uses it).
pub const STORY_FOLDER: &str = "adventures";

#[derive(Default)]
pub struct MainHost(pub PtyHost);
#[derive(Default)]
pub struct StoryHost(pub PtyHost);

/// Which session a command is for: the main one (the default) or the STORY room's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Room {
    Main,
    Story,
}

impl Room {
    pub fn named(room: Option<&str>) -> Room {
        if room == Some("story") {
            Room::Story
        } else {
            Room::Main
        }
    }

    fn events(self) -> (&'static str, &'static str) {
        match self {
            Room::Main => (TERMINAL_OUTPUT, TERMINAL_EXIT),
            Room::Story => (STORY_OUTPUT, STORY_EXIT),
        }
    }

    /// The folder the room's session starts in.
    pub fn folder(self, settings: &Settings) -> Option<PathBuf> {
        let memory = PathBuf::from(settings.memory_folder.as_ref()?);
        Some(match self {
            Room::Main => memory,
            Room::Story => memory.join(STORY_FOLDER),
        })
    }
}

/// Whether a room's session can start, and if not, why - in words for the screen.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub ready: bool,
    pub message: String,
    /// What the screen can offer to fix it: "setup", "settings" or "install".
    pub fix: Option<&'static str>,
}

pub fn status(settings: &Settings, room: Room, program: Option<&std::path::Path>) -> SessionStatus {
    let not_ready = |message: String, fix: &'static str| SessionStatus { ready: false, message, fix: Some(fix) };
    let Some(folder) = room.folder(settings) else {
        return not_ready("No MemoryCore folder is chosen yet.".into(), "setup");
    };
    if room == Room::Main && !folder.is_dir() {
        return not_ready("Your MemoryCore folder was not found. Choose it again in Settings.".into(), "settings");
    }
    if room == Room::Story && !folder.is_dir() {
        return not_ready("The story room is not ready yet. Open STORY once to prepare it.".into(), "setup");
    }
    if program.is_none() {
        let cli = settings.cli.label();
        return not_ready(format!("{cli} was not found on this computer. Install it, then try again."), "install");
    }
    let message = match room {
        Room::Main => format!("Ready to wake {}.", settings.companion_label()),
        Room::Story => "Ready to open the story.".to_string(),
    };
    SessionStatus { ready: true, message, fix: None }
}

fn host<'a>(room: Room, main: &'a MainHost, story: &'a StoryHost) -> &'a PtyHost {
    match room {
        Room::Main => &main.0,
        Room::Story => &story.0,
    }
}

/// Where Castle's helper scripts live (bundled resources).
pub fn tools_folder(app: &AppHandle) -> PathBuf {
    app.path().resource_dir().map(|dir| dir.join("tools")).unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalView {
    running: bool,
    status: SessionStatus,
    snapshot: String,
}

fn view(store: &SettingsStore, host: &PtyHost, room: Room) -> TerminalView {
    let settings = store.get();
    let program = env::find_on_path(settings.cli.command());
    TerminalView {
        running: host.is_running(),
        status: status(&settings, room, program.as_deref()),
        snapshot: host.snapshot(),
    }
}

/// Starts the room's session unless it is already running. The screen hears it through that
/// room's output and exit events.
fn wake(app: &AppHandle, store: &SettingsStore, host: &PtyHost, room: Room, cols: u16, rows: u16) -> Result<(), String> {
    let settings = store.get();
    let program = env::find_on_path(settings.cli.command());
    let ready = status(&settings, room, program.as_deref());
    let (Some(folder), Some(program), true) = (room.folder(&settings), program, ready.ready) else {
        return Err(ready.message);
    };
    let mut vars = env::session_env(settings.cli, &|name| key::load(name), &tools_folder(app));
    // the drawing styles (standard appearance, turnarounds, try-on) ship beside the tools
    if let Ok(dir) = app.path().resource_dir() {
        vars.push(("CASTLE_STYLES".into(), dir.join("styles").display().to_string()));
    }
    let (output_event, exit_event) = room.events();
    let output = app.clone();
    let ending = app.clone();
    host.start(
        pty::launch_command(&program, &folder, &vars),
        cols.max(20),
        rows.max(5),
        move |text| {
            let _ = output.emit(output_event, text);
        },
        move |id, code| {
            match room {
                Room::Main => ending.state::<MainHost>().0.finished(id),
                Room::Story => ending.state::<StoryHost>().0.finished(id),
            }
            let _ = ending.emit(exit_event, code);
        },
    )?;
    Ok(())
}

pub fn any_running(app: &AppHandle) -> bool {
    app.state::<MainHost>().0.is_running() || app.state::<StoryHost>().0.is_running()
}

pub fn end_all(app: &AppHandle) {
    app.state::<MainHost>().0.kill();
    app.state::<StoryHost>().0.kill();
}

#[tauri::command]
pub fn terminal_state(room: Option<String>, store: State<SettingsStore>, main: State<MainHost>, story: State<StoryHost>) -> TerminalView {
    let room = Room::named(room.as_deref());
    view(&store, host(room, &main, &story), room)
}

#[tauri::command]
pub fn terminal_start(
    app: AppHandle,
    cols: u16,
    rows: u16,
    room: Option<String>,
    store: State<SettingsStore>,
    main: State<MainHost>,
    story: State<StoryHost>,
) -> Result<TerminalView, String> {
    let room = Room::named(room.as_deref());
    let host = host(room, &main, &story);
    wake(&app, &store, host, room, cols, rows)?;
    Ok(view(&store, host, room))
}

#[tauri::command]
pub fn terminal_write(data: String, room: Option<String>, main: State<MainHost>, story: State<StoryHost>) -> Result<(), String> {
    host(Room::named(room.as_deref()), &main, &story).write(data.as_bytes())
}

#[tauri::command]
pub fn terminal_resize(cols: u16, rows: u16, room: Option<String>, main: State<MainHost>, story: State<StoryHost>) -> Result<(), String> {
    host(Room::named(room.as_deref()), &main, &story).resize(cols.max(20), rows.max(5))
}

/// Ends the STORY room's session ("Leave the story"); the main one is never ended this way.
#[tauri::command]
pub fn story_close(story: State<StoryHost>) {
    story.0.kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("castle-session-{name}-{}", std::process::id()));
        // an old run with the same process id may have left it behind
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn status_names_what_is_missing_and_how_to_fix_it() {
        let program = Path::new("C:\\bin\\claude.exe");
        let unset = Settings::default();
        assert_eq!(status(&unset, Room::Main, Some(program)).fix, Some("setup"));

        let gone = Settings { memory_folder: Some(folder("x").join("nope").display().to_string()), ..Settings::default() };
        assert_eq!(status(&gone, Room::Main, Some(program)).fix, Some("settings"));

        let memory = folder("ok");
        let set = Settings { memory_folder: Some(memory.display().to_string()), companion_name: Some("Lyra".into()), ..Settings::default() };
        let missing_cli = status(&set, Room::Main, None);
        assert_eq!((missing_cli.ready, missing_cli.fix), (false, Some("install")));
        assert!(missing_cli.message.contains("Claude Code"));

        let ready = status(&set, Room::Main, Some(program));
        assert!(ready.ready);
        assert_eq!(ready.message, "Ready to wake Lyra.");

        // the story room needs its own folder
        assert!(!status(&set, Room::Story, Some(program)).ready);
        std::fs::create_dir_all(memory.join(STORY_FOLDER)).unwrap();
        assert!(status(&set, Room::Story, Some(program)).ready);
    }

    #[test]
    fn rooms_are_named_by_the_page() {
        assert_eq!(Room::named(Some("story")), Room::Story);
        assert_eq!(Room::named(None), Room::Main);
        assert_eq!(Room::named(Some("anything")), Room::Main);
    }
}
