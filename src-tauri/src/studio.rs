//! STUDIO: what the companion made - pictures, videos, songs, voice lines - and what was saved
//! instead when a key was missing. Plus the notices the helpers leave, turned into toasts.

use crate::key;
use crate::settings::{Cli, SettingsStore};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

pub const NOTICE_EVENT: &str = "castle-notice";
const LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Made {
    /// absolute path (the page shows it through the asset protocol, scoped to the memory folder)
    pub path: String,
    pub name: String,
    /// a group: an album for songs, a world for story scenes
    pub group: Option<String>,
    /// seconds since 1970
    pub at: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub images: Vec<Made>,
    pub videos: Vec<Made>,
    pub songs: Vec<Made>,
    pub voices: Vec<Made>,
    /// prompts, lyrics and lines saved instead of a render (no key)
    pub saved: Vec<Made>,
}

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            if depth < 4 {
                walk(&path, depth + 1, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn made(path: &Path, group: Option<String>) -> Made {
    let at = path.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    Made { path: path.display().to_string(), name: path.file_name().unwrap_or_default().to_string_lossy().to_string(), group, at }
}

/// Everything made in the memory folder: media-generation/ (pictures, videos, voice), music/
/// (albums and singles), adventures/*/scenes and music, appearance/.
pub fn library(folder: &Path) -> Library {
    let mut files = Vec::new();
    for dir in ["media-generation", "music", "adventures", "appearance"] {
        walk(&folder.join(dir), 0, &mut files);
    }
    let mut lib = Library::default();
    for path in files {
        let rel = path.strip_prefix(folder).map(|p| p.display().to_string().replace('\\', "/")).unwrap_or_default();
        let parts: Vec<&str> = rel.split('/').collect();
        let group = match parts.first() {
            Some(&"music") if parts.len() > 2 => Some(parts[1].to_string()),
            Some(&"adventures") if parts.len() > 2 => Some(parts[1].to_string()),
            _ => None,
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if name.ends_with(".prompt.md") || name.ends_with(".lyrics.md") || name.ends_with(".line.txt") {
            lib.saved.push(made(&path, group));
            continue;
        }
        match ext(&path).as_str() {
            "png" | "jpg" | "jpeg" | "webp" if !name.starts_with("cover") || parts.first() != Some(&"music") => lib.images.push(made(&path, group)),
            "mp4" | "webm" => lib.videos.push(made(&path, group)),
            "mp3" | "wav" | "m4a" | "ogg" => {
                if parts.first() == Some(&"music") || rel.contains("/music/") {
                    lib.songs.push(made(&path, group));
                } else {
                    lib.voices.push(made(&path, group));
                }
            }
            _ => {}
        }
    }
    for list in [&mut lib.images, &mut lib.videos, &mut lib.songs, &mut lib.voices, &mut lib.saved] {
        list.sort_by(|a, b| b.at.cmp(&a.at));
        list.truncate(LIMIT);
    }
    lib
}

/// Each STUDIO card's state: "ready", "builtin" (Codex draws by itself) or "prompt" (no key).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Powers {
    pub image: &'static str,
    pub video: &'static str,
    pub song: &'static str,
    pub voice: &'static str,
}

pub fn powers(cli: Cli, has: &dyn Fn(&str) -> bool) -> Powers {
    let state = |name: &str| if has(name) { "ready" } else { "prompt" };
    Powers {
        image: if cli == Cli::Codex { "builtin" } else { state("openai") },
        video: state("seedance"),
        song: state("suno"),
        voice: state("elevenlabs"),
    }
}

#[tauri::command]
pub fn studio_library(store: State<SettingsStore>) -> Library {
    store.get().memory_folder.map(|f| library(Path::new(&f))).unwrap_or_default()
}

#[tauri::command]
pub fn studio_powers(store: State<SettingsStore>) -> Powers {
    powers(store.get().cli, &|name| key::load(name).is_some())
}

/// Lets the page show files from the memory folder (and only there) through the asset protocol.
pub fn allow_memory_folder(app: &AppHandle) {
    if let Some(folder) = app.state::<SettingsStore>().get().memory_folder {
        let _ = app.asset_protocol_scope().allow_directory(Path::new(&folder), true);
    }
}

/// Where the helpers leave their notices (castle-common.ps1 writes here).
pub fn notices_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("MemoryCore Castle").join("notices.jsonl"))
}

/// The room a notice belongs to: a world prepare is WORLD's, a drawing for the companion's look
/// (a file under appearance/) is APPEARANCE's, every other picture, video, song or voice STUDIO's.
pub fn notice_room(notice: &serde_json::Value) -> &'static str {
    if notice.get("kind").and_then(|k| k.as_str()) == Some("world") {
        return "world";
    }
    let file = notice.get("file").and_then(|f| f.as_str()).unwrap_or("").replace('\\', "/").to_lowercase();
    if file.contains("/appearance/") { "appearance" } else { "studio" }
}

/// Watches notices.jsonl and sends each new line to the page as a toast. Lines written before
/// Castle started are not repeated.
pub fn start_notice_watcher(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let Some(path) = notices_file() else { return };
        let mut offset = path.metadata().map(|m| m.len()).unwrap_or(0);
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let Ok(len) = path.metadata().map(|m| m.len()) else { continue };
            if len < offset {
                offset = 0;
            }
            if len == offset {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let fresh = String::from_utf8_lossy(&bytes[offset as usize..]).to_string();
                let settings = app.state::<SettingsStore>().get();
                for line in fresh.lines().filter(|l| !l.trim().is_empty()) {
                    if let Ok(notice) = serde_json::from_str::<serde_json::Value>(line) {
                        // a room switched off in Settings holds its notices back (the line stays in the file)
                        if settings.room_on(notice_room(&notice)) {
                            let _ = app.emit(NOTICE_EVENT, notice);
                        }
                    }
                }
                offset = len;
            }
            let _ = SystemTime::now();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notice_belongs_to_the_room_that_made_it() {
        let n = |v: &str| serde_json::from_str::<serde_json::Value>(v).unwrap();
        assert_eq!(notice_room(&n(r#"{"kind":"image","file":"D:\\Memory\\appearance\\wardrobe\\coat\\turnaround.prompt.md"}"#)), "appearance");
        assert_eq!(notice_room(&n(r#"{"kind":"song","file":"D:\\Memory\\music\\a\\x.lyrics.md"}"#)), "studio");
        assert_eq!(notice_room(&n(r#"{"kind":"world","file":"D:\\Memory\\worlds\\x\\hd"}"#)), "world");
    }

    #[test]
    fn made_things_are_sorted_into_their_shelves() {
        let root = std::env::temp_dir().join(format!("castle-studio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, file) in [
            ("media-generation/image-generation", "lake.png"),
            ("media-generation/image-generation", "lake.prompt.md"),
            ("media-generation/video-generation", "waves.mp4"),
            ("media-generation/voice", "hello.mp3"),
            ("music/sky-songs/audio", "01-first-light.mp3"),
            ("music/sky-songs", "night.lyrics.md"),
            ("adventures/celestial/scenes", "01-gate.png"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(file), b"x").unwrap();
        }
        let lib = library(&root);
        assert_eq!(lib.images.len(), 2);
        assert_eq!(lib.videos.len(), 1);
        assert_eq!(lib.voices.len(), 1);
        assert_eq!(lib.songs.len(), 1);
        assert_eq!(lib.songs[0].group.as_deref(), Some("sky-songs"));
        assert_eq!(lib.saved.len(), 2);
        assert!(lib.images.iter().any(|m| m.group.as_deref() == Some("celestial")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn powers_follow_the_keys_and_the_cli() {
        let only_suno = |n: &str| n == "suno";
        let p = powers(Cli::Claude, &only_suno);
        assert_eq!((p.image, p.video, p.song, p.voice), ("prompt", "prompt", "ready", "prompt"));
        assert_eq!(powers(Cli::Codex, &only_suno).image, "builtin");
    }
}
