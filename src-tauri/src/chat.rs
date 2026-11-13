//! Chat: each session's own record, read only. The main session and the STORY room each have
//! their own reader, so the two Chats never mix.

use crate::files;
use crate::session::{MainHost, Room, StoryHost, STORY_FOLDER};
use crate::settings::{Cli, SettingsStore};
use crate::transcript::{self, claude, ChatEntry, Follower};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::State;

/// The record a Chat follows, and the session it belongs to.
#[derive(Default)]
pub struct ChatState(Mutex<Option<(u64, Follower)>>);
#[derive(Default)]
pub struct MainChat(pub ChatState);
#[derive(Default)]
pub struct StoryChat(pub ChatState);

/// Pictures the member dropped or pasted into Chat this run: the screen may show these as
/// thumbnails before they are sent (once sent, the record reader allows them as the member's).
#[derive(Default)]
pub struct MemberPictures(pub Mutex<HashSet<String>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPoll {
    entries: Vec<ChatEntry>,
    /// True when Chat started reading a new record: the screen clears and shows these.
    reset: bool,
    /// "asleep", "waiting" (no record yet), "ok" or "unreadable"
    status: &'static str,
}

fn pick<'a>(room: Room, main: &'a MainChat, story: &'a StoryChat) -> &'a ChatState {
    match room {
        Room::Main => &main.0,
        Room::Story => &story.0,
    }
}

/// The newest record of `folder`'s session written since `started`, and how to follow it.
pub(crate) fn newest_record(cli: Cli, folder: &Path, started: SystemTime) -> Option<(PathBuf, fn(PathBuf, SystemTime) -> Follower)> {
    match cli {
        Cli::Claude => {
            let records = claude::projects_root()?.join(claude::project_folder_name(&folder.display().to_string()));
            claude::newest_record(&records, started).map(|path| (path, claude::follower as fn(PathBuf, SystemTime) -> Follower))
        }
        Cli::Codex => transcript::codex::newest_record(folder, started).map(|path| (path, transcript::codex::follower as fn(PathBuf, SystemTime) -> Follower)),
    }
}

/// New chat entries since the last poll. Follows the newest record of this session's folder,
/// so a fresh record (after /clear or /new) is picked up.
#[tauri::command]
pub fn chat_poll(
    room: Option<String>,
    store: State<SettingsStore>,
    main: State<MainHost>,
    story: State<StoryHost>,
    main_chat: State<MainChat>,
    story_chat: State<StoryChat>,
) -> ChatPoll {
    let room = Room::named(room.as_deref());
    let host = match room {
        Room::Main => &main.0,
        Room::Story => &story.0,
    };
    let chat = pick(room, &main_chat, &story_chat);
    let empty = |status| ChatPoll { entries: Vec::new(), reset: false, status };
    let Some((session, started)) = host.started() else {
        return empty("asleep");
    };
    let mut settings = store.get();
    settings.cli = host.provider().unwrap_or(settings.cli);
    let Some(folder) = room.folder(&settings) else {
        return empty("waiting");
    };
    let Some((newest, follow)) = newest_record(settings.cli, &folder, started) else {
        return empty("waiting");
    };

    let mut slot = chat.0.lock().unwrap();
    let mut reset = false;
    let current = slot.as_ref().map(|(id, f)| (*id, f.path.clone()));
    if current != Some((session, newest.clone())) {
        let follower = follow(newest, started);
        // The main Chat leaves the STORY room's pictures and voices to the STORY room.
        let follower = match room {
            Room::Main => follower.skipping(&folder.join(STORY_FOLDER)),
            Room::Story => follower,
        };
        *slot = Some((session, follower));
        reset = true;
    }
    let (_, follower) = slot.as_mut().expect("set above");
    match follower.read_new() {
        Ok(mut entries) => {
            entries.retain(|e| !crate::providers::castle_wrote(e));
            if let Err(error) = crate::providers::remember(&store, room, settings.cli, session, &mut entries) {
                eprintln!("MemoryCore Castle: {error}");
            }
            if reset && entries.len() > transcript::FIRST_READ_LIMIT {
                entries = entries.split_off(entries.len() - transcript::FIRST_READ_LIMIT);
            }
            if reset { entries = crate::providers::history(&store, room); }
            ChatPoll { entries, reset, status: "ok" }
        }
        Err(_) => ChatPoll { entries: Vec::new(), reset, status: "unreadable" },
    }
}

#[tauri::command]
pub fn chat_history(room: Option<String>, store: State<SettingsStore>) -> Vec<ChatEntry> {
    crate::providers::history(&store, Room::named(room.as_deref()))
}

/// A picture or voice file Chat already showed (the companion's this session, or one the member
/// shared), as a data URL - never any other path.
#[tauri::command]
pub fn chat_media(
    path: String,
    room: Option<String>,
    main_chat: State<MainChat>,
    story_chat: State<StoryChat>,
    member: State<MemberPictures>,
    store: State<SettingsStore>,
) -> Result<String, String> {
    let chat = pick(Room::named(room.as_deref()), &main_chat, &story_chat);
    let allowed = chat.0.lock().unwrap().as_ref().map(|(_, f)| f.was_shown(&path)).unwrap_or(false)
        || member.0.lock().unwrap().contains(&path)
        || crate::providers::history(&store, Room::named(room.as_deref())).iter().any(|e| e.path.as_ref() == Some(&path) || e.pictures.contains(&path));
    if !allowed {
        return Err("That file is not one Chat has shown this session.".into());
    }
    files::data_url(Path::new(&path))
}

/// Takes the paths the member dropped on Chat; returns the pictures among them (existing png /
/// jpg / webp files up to 25 MB), which the screen may then show.
#[tauri::command]
pub fn chat_attach(paths: Vec<String>, member: State<MemberPictures>) -> Vec<String> {
    let pictures: Vec<String> = paths
        .into_iter()
        .filter(|p| transcript::media_kind(p) == Some("picture"))
        .filter(|p| std::fs::metadata(p).map(|m| m.is_file() && m.len() <= 25 * 1024 * 1024).unwrap_or(false))
        .collect();
    member.0.lock().unwrap().extend(pictures.iter().cloned());
    pictures
}
