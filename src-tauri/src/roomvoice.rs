//! The companion speaks a line aloud in the chat room, when the member allows it (Settings >
//! Chat: off by default, at most 5 a day, under the room's own cap) and an ElevenLabs key is
//! saved. The turn marks its say with <speak/>; the words are spoken with the member's key and
//! posted as one voice line (the words and the voice together) with the companion's room key.
//! Speaking runs on its own thread, one at a time; when a voice cannot be made or the room
//! refuses it, the words are posted as plain text, so the line is never lost.

use crate::chatroom::{check_voice, Room};
use crate::roomdraw::{left_today, Day};
use crate::settings::{Settings, SettingsStore};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// The longest say that is spoken; a longer one is posted as text.
pub const SPEAK_TEXT_MAX: usize = 300;
/// The voice STUDIO's voice helper uses when none is chosen (castle-voice.mjs).
pub const DEFAULT_VOICE: &str = "21m00Tcm4TlvDq8ikWAM";
const MODEL: &str = "eleven_multilingual_v2";
static SPEAKING: AtomicBool = AtomicBool::new(false);

fn has_eleven_key() -> bool {
    crate::key::load("elevenlabs").is_some()
}

/// The voice to speak in: the member's choice, or the default. Pure, tested.
pub fn voice_for(settings: &Settings) -> &str {
    settings.room_voice_id.as_deref().filter(|v| !v.is_empty()).unwrap_or(DEFAULT_VOICE)
}

/// The words a say is spoken with: trimmed, and only when short enough to speak. Pure, tested.
pub fn speakable(text: &str) -> Option<&str> {
    let text = text.trim();
    (!text.is_empty() && text.chars().count() <= SPEAK_TEXT_MAX).then_some(text)
}

fn today() -> String {
    crate::link::utc_label(crate::link::now_secs()).chars().take(10).collect()
}

fn day_file(folder: &Path) -> PathBuf {
    folder.join("voices-today.json")
}

fn read_day(folder: &Path) -> Day {
    std::fs::read_to_string(day_file(folder)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn count_one(folder: &Path) {
    let now = today();
    let mut day = read_day(folder);
    if day.date != now {
        day = Day { date: now, count: 0 };
    }
    day.count += 1;
    let _ = std::fs::write(day_file(folder), serde_json::to_string(&day).unwrap_or_default());
}

/// Voices still allowed by the member's own switch and limit, before the room is asked. Pure, tested.
pub fn member_left(on: bool, has_key: bool, speaking: bool, day: &Day, today: &str, limit: u32) -> u32 {
    if !on || !has_key || speaking {
        return 0;
    }
    left_today(day, today, limit)
}

/// How many voices the turn may offer now: 0 unless speaking is on, a key is saved, nothing is
/// being spoken, and both the member's limit and the room's cap leave one (a room that does not
/// answer leaves none).
pub fn voices_left(settings: &Settings, store: &SettingsStore, ai_room: &Room) -> u32 {
    let day = read_day(&crate::roomturn::room_folder(store));
    let mine = member_left(settings.room_voices_on, has_eleven_key(), SPEAKING.load(Ordering::Relaxed), &day, &today(), settings.room_voices_per_day);
    if mine == 0 {
        return 0;
    }
    let room = ai_room.voice_day().map(|(cap, used)| cap.saturating_sub(used)).unwrap_or(0);
    mine.min(room.min(u32::MAX as u64) as u32)
}

/// ElevenLabs text-to-speech with the member's key: an mp3, or ElevenLabs' own words (never the key).
fn eleven_speak(key: &str, voice: &str, text: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(60)).user_agent("MemoryCoreCastle").build().map_err(|_| "The voice could not start.".to_string())?;
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}");
    let answer = client
        .post(url)
        .header("xi-api-key", key)
        .header("Accept", "audio/mpeg")
        .json(&serde_json::json!({ "text": text, "model_id": MODEL }))
        .send()
        .map_err(|_| "ElevenLabs could not be reached.".to_string())?;
    let status = answer.status().as_u16();
    if !(200..300).contains(&status) {
        let body: Value = answer.json().unwrap_or(Value::Null);
        let said = body.pointer("/detail/message").or_else(|| body.get("detail")).and_then(Value::as_str).unwrap_or("the request was refused");
        return Err(format!("ElevenLabs said ({status}): {}", said.chars().take(200).collect::<String>()));
    }
    let bytes = answer.bytes().map_err(|_| "ElevenLabs' voice could not be read.".to_string())?.to_vec();
    check_voice(&bytes).map_err(|no| no.message)?;
    Ok(bytes)
}

/// Speaks `words` and posts them as the companion's voice line, on its own thread; the words go
/// as text when the voice cannot be made or the room refuses it.
pub fn speak_and_post(app: &AppHandle, words: String) {
    let folder = crate::roomturn::room_folder(&app.state::<SettingsStore>());
    let app = app.clone();
    if SPEAKING.swap(true, Ordering::SeqCst) {
        // one voice at a time: this one is said as text instead
        std::thread::spawn(move || {
            let said = post_text(&folder, &words, "a voice is still being made");
            crate::roomturn::log(&folder, &said);
            let _ = app.emit("room-turn", ());
        });
        return;
    }
    std::thread::spawn(move || {
        let said = speak_now(&app, &folder, &words);
        crate::roomturn::log(&folder, &said);
        let _ = app.emit("room-turn", ());
        SPEAKING.store(false, Ordering::SeqCst);
    });
}

fn ai_room() -> Option<Room> {
    crate::key::load("room-companion").map(|key| Room::new(&crate::signin::portfolio(), &key))
}

fn post_text(_folder: &Path, words: &str, why: &str) -> String {
    let Some(room) = ai_room() else { return format!("could not speak ({why}), and the companion is no longer in the room") };
    match room.post(words) {
        Ok(line) => {
            if let Some(id) = line.get("id").and_then(Value::as_u64) {
                crate::roomturn::remember_own(id);
            }
            format!("could not speak ({why}) - said it as text: {words}")
        }
        Err(r) => format!("could not speak ({why}), and could not say it as text: {}", r.message),
    }
}

fn speak_now(app: &AppHandle, folder: &Path, words: &str) -> String {
    let settings = app.state::<SettingsStore>().get();
    let Some(key) = crate::key::load("elevenlabs") else { return post_text(folder, words, "the ElevenLabs key is gone") };
    let bytes = match eleven_speak(&key, voice_for(&settings), words) {
        Ok(b) => b,
        Err(why) => return post_text(folder, words, &why),
    };
    let Some(room) = ai_room() else { return "made a voice, but the companion is no longer in the room".into() };
    match room.post_voice(bytes, Some(words), None) {
        Ok(line) => {
            count_one(folder);
            if let Some(id) = line.get("id").and_then(Value::as_u64) {
                crate::roomturn::remember_own(id);
            }
            format!("spoke: {words}")
        }
        Err(refused) => post_text(folder, words, &format!("the room refused the voice: {}", refused.message)),
    }
}

/// What Settings > Chat shows about speaking.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicesStatus {
    has_key: bool,
    used_today: u32,
    limit: u32,
    /// the room's own cap and count for this companion; None while it is not in the room or the
    /// room does not answer
    room_cap: Option<u64>,
    room_today: Option<u64>,
}

#[tauri::command]
pub async fn room_voices_status(store: tauri::State<'_, SettingsStore>) -> Result<VoicesStatus, String> {
    let settings = store.get();
    let day = read_day(&crate::roomturn::room_folder(&store));
    let used_today = if day.date == today() { day.count } else { 0 };
    let room = tauri::async_runtime::spawn_blocking(|| ai_room().and_then(|room| room.voice_day())).await.unwrap_or(None);
    Ok(VoicesStatus { has_key: has_eleven_key(), used_today, limit: settings.room_voices_per_day, room_cap: room.map(|r| r.0), room_today: room.map(|r| r.1) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_member_allows_a_voice_only_when_on_with_a_key_and_nothing_speaking() {
        let day = Day { date: "2026-11-18".into(), count: 1 };
        assert_eq!(member_left(true, true, false, &day, "2026-11-18", 2), 1);
        assert_eq!(member_left(false, true, false, &day, "2026-11-18", 2), 0, "the switch off");
        assert_eq!(member_left(true, false, false, &day, "2026-11-18", 2), 0, "no ElevenLabs key");
        assert_eq!(member_left(true, true, true, &day, "2026-11-18", 2), 0, "one voice at a time");
        assert_eq!(member_left(true, true, false, &day, "2026-11-18", 0), 0, "0 a day = never");
        assert_eq!(member_left(true, true, false, &day, "2026-11-19", 2), 2, "a new day starts again");
    }

    #[test]
    fn only_a_short_say_is_spoken() {
        assert_eq!(speakable("  hello there  "), Some("hello there"));
        assert_eq!(speakable("   "), None);
        assert_eq!(speakable(&"a".repeat(SPEAK_TEXT_MAX)).map(str::len), Some(SPEAK_TEXT_MAX));
        assert_eq!(speakable(&"a".repeat(SPEAK_TEXT_MAX + 1)), None, "a long say goes as text");
    }

    #[test]
    fn the_voice_is_the_member_choice_or_the_default() {
        let mut settings = Settings::default();
        assert_eq!(voice_for(&settings), DEFAULT_VOICE);
        settings.room_voice_id = Some("pNInz6obpgDQGcFmaJgB".into());
        assert_eq!(voice_for(&settings), "pNInz6obpgDQGcFmaJgB");
    }
}
