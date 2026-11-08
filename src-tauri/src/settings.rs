//! Castle's settings for this machine: one JSON file in the app config folder.
//! Keys never live here; they are kept by Windows Credential Manager (key.rs).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const FILE_NAME: &str = "settings.json";

/// Which AI CLI runs inside Castle. Each signs in with the member's own account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Cli {
    #[default]
    Claude,
    Codex,
}

impl Cli {
    /// The command name looked up on PATH.
    pub fn command(self) -> &'static str {
        match self {
            Cli::Claude => "claude",
            Cli::Codex => "codex",
        }
    }

    /// The name shown on the companion's chip.
    pub fn label(self) -> &'static str {
        match self {
            Cli::Claude => "Claude Code",
            Cli::Codex => "Codex",
        }
    }
}

/// A button the member added to the quick-action row: its label and the words it types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickAction {
    pub label: String,
    pub command: String,
}

/// The WORLD room: 3D isles or the 2D slideshow, and its music.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorldSettings {
    /// "3d" or "2d".
    pub mode: String,
    pub music: bool,
    /// 0.0 to 1.0.
    pub volume: f32,
    /// Extra picture folders for the 2D slideshow, beside the memory folder's own pictures.
    pub folders: Vec<String>,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self { mode: "3d".into(), music: true, volume: 0.5, folders: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The member's MemoryCore folder: the companion's session runs here.
    pub memory_folder: Option<String>,
    pub cli: Cli,
    pub companion_name: Option<String>,
    pub member_name: Option<String>,
    pub relationship_style: Option<String>,
    pub theme: Option<String>,
    /// Off until the member turns it on.
    pub start_with_windows: bool,
    /// Start the companion's session when Castle opens.
    pub start_session_on_launch: bool,
    /// The MemoryCore Foundation room's address, e.g. https://example.com - no trailing slash, no /api.
    pub room_base_url: Option<String>,
    pub world: WorldSettings,
    /// True once the first-run setup has finished.
    pub setup_done: bool,
    /// The member's own quick-action buttons, after Castle's built-in ones.
    pub quick_actions: Vec<QuickAction>,
    /// Back up the memory to the member's private GitHub repository after every Save diary.
    pub backup_after_diary: bool,
    /// The companion in the Foundation chat room: "off", "named" (answers when its name is
    /// said) or "joins" (joins in now and then).
    pub room_companion_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory_folder: None,
            cli: Cli::Claude,
            companion_name: None,
            member_name: None,
            relationship_style: None,
            theme: None,
            start_with_windows: false,
            start_session_on_launch: true,
            room_base_url: None,
            world: WorldSettings::default(),
            setup_done: false,
            quick_actions: Vec::new(),
            backup_after_diary: false,
            room_companion_mode: "off".into(),
        }
    }
}

impl Settings {
    /// Trims every text field; an empty one means unset. The room address loses its trailing slash.
    pub fn normalized(mut self) -> Self {
        fn clean(value: Option<String>) -> Option<String> {
            value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
        }
        self.memory_folder = clean(self.memory_folder);
        self.companion_name = clean(self.companion_name);
        self.member_name = clean(self.member_name);
        self.relationship_style = clean(self.relationship_style);
        self.theme = clean(self.theme);
        self.room_base_url = clean(self.room_base_url).map(|v| v.trim_end_matches('/').to_string());
        self.world.volume = self.world.volume.clamp(0.0, 1.0);
        self.quick_actions.retain(|a| !a.label.trim().is_empty() && !a.command.trim().is_empty());
        for action in &mut self.quick_actions {
            action.label = action.label.trim().to_string();
            // a quick action types one line: no line breaks may sneak in
            action.command = action.command.replace(['\r', '\n'], " ").trim().to_string();
        }
        if !["off", "named", "joins"].contains(&self.room_companion_mode.as_str()) {
            self.room_companion_mode = "off".into();
        }
        if self.world.mode != "2d" {
            self.world.mode = "3d".into();
        }
        self
    }

    /// A plain reason when the settings cannot be kept, else None.
    pub fn problem(&self) -> Option<String> {
        if let Some(url) = &self.room_base_url {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Some("The room address must start with https:// or http://".into());
            }
        }
        None
    }

    /// The companion's name for the screen, or a gentle default before setup.
    pub fn companion_label(&self) -> String {
        self.companion_name.clone().unwrap_or_else(|| "Your companion".into())
    }
}

/// Reads the file. A missing file gives the defaults; a broken one gives the defaults and a reason.
pub fn load(path: &Path) -> (Settings, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Settings>(&text) {
            Ok(settings) => (settings.normalized(), None),
            Err(_) => (
                Settings::default(),
                Some("The settings file could not be read, so the defaults are in use.".into()),
            ),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => (Settings::default(), None),
        Err(_) => (
            Settings::default(),
            Some("The settings file could not be opened, so the defaults are in use.".into()),
        ),
    }
}

/// Writes beside the file first and then swaps it in, so a crash never leaves half a file.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let partial = path.with_extension("json.partial");
    fs::write(&partial, text)?;
    fs::rename(&partial, path)
}

/// Test builds only: a separate settings folder, so testing never touches the installed Castle's.
pub const TEST_CONFIG_VAR: &str = "CASTLE_CONFIG_DIR";

/// Reads a test-only switch; the installed Castle never does.
pub fn test_override(name: &str) -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
    } else {
        None
    }
}

/// The settings the running Castle holds, and where they are kept.
pub struct SettingsStore {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
    pub warning: Mutex<Option<String>>,
    /// Why Start with Windows is not in effect, when it is not.
    pub startup_note: Mutex<Option<String>>,
}

impl SettingsStore {
    pub fn open(folder: &Path) -> Self {
        let path = folder.join(FILE_NAME);
        let (current, warning) = load(&path);
        Self {
            path,
            current: Mutex::new(current),
            warning: Mutex::new(warning),
            startup_note: Mutex::new(None),
        }
    }

    /// The folder that holds settings.json, notices.jsonl, themes and the avatar.
    pub fn folder(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("castle-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_missing_file_gives_the_defaults_with_start_up_off() {
        let folder = temp_folder("missing");
        let (settings, warning) = load(&folder.join(FILE_NAME));
        assert_eq!(settings, Settings::default());
        assert!(!settings.start_with_windows);
        assert!(!settings.setup_done);
        assert_eq!(settings.cli, Cli::Claude);
        assert!(warning.is_none());
    }

    #[test]
    fn settings_survive_a_save_and_a_load() {
        let folder = temp_folder("roundtrip");
        let path = folder.join(FILE_NAME);
        let settings = Settings {
            memory_folder: Some("D:\\MemoryCore".into()),
            cli: Cli::Codex,
            companion_name: Some("Lyra".into()),
            member_name: Some("Rin".into()),
            relationship_style: Some("Friendly partner".into()),
            theme: Some("constellation".into()),
            start_with_windows: true,
            start_session_on_launch: false,
            room_base_url: Some("https://example.test".into()),
            world: WorldSettings { mode: "2d".into(), music: false, volume: 0.25, folders: vec!["D:\\Pictures".into()] },
            setup_done: true,
            quick_actions: vec![QuickAction { label: "Plan my day".into(), command: "plan my day".into() }],
            backup_after_diary: true,
            room_companion_mode: "named".into(),
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), (settings, None));
        assert!(!folder.join("settings.json.partial").exists());
    }

    #[test]
    fn the_cli_is_written_in_lowercase() {
        let text = serde_json::to_string(&Settings { cli: Cli::Codex, ..Settings::default() }).unwrap();
        assert!(text.contains("\"cli\":\"codex\""));
        assert_eq!(Cli::Codex.command(), "codex");
        assert_eq!(Cli::Claude.label(), "Claude Code");
    }

    #[test]
    fn a_broken_file_gives_the_defaults_and_a_reason() {
        let folder = temp_folder("broken");
        let path = folder.join(FILE_NAME);
        fs::write(&path, "{ not json").unwrap();
        let (settings, warning) = load(&path);
        assert_eq!(settings, Settings::default());
        assert!(warning.is_some());
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let folder = temp_folder("partial");
        let path = folder.join(FILE_NAME);
        fs::write(&path, r#"{"memoryFolder":"D:\\MemoryCore","somethingNew":1}"#).unwrap();
        let (settings, _) = load(&path);
        assert_eq!(settings.memory_folder.as_deref(), Some("D:\\MemoryCore"));
        assert!(settings.start_session_on_launch);
        assert_eq!(settings.world, WorldSettings::default());
    }

    #[test]
    fn text_is_trimmed_and_odd_values_are_brought_back_in_range() {
        let settings = Settings {
            room_base_url: Some("  https://example.test/ ".into()),
            companion_name: Some("   ".into()),
            world: WorldSettings { mode: "4d".into(), volume: 3.0, ..WorldSettings::default() },
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.room_base_url.as_deref(), Some("https://example.test"));
        assert_eq!(settings.companion_name, None);
        assert_eq!(settings.world.mode, "3d");
        assert_eq!(settings.world.volume, 1.0);
        assert_eq!(settings.companion_label(), "Your companion");
    }

    #[test]
    fn quick_actions_are_one_clean_line_each() {
        let settings = Settings {
            quick_actions: vec![
                QuickAction { label: " Review ".into(), command: "review\r\nmy week ".into() },
                QuickAction { label: "".into(), command: "x".into() },
            ],
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.quick_actions, vec![QuickAction { label: "Review".into(), command: "review  my week".into() }]);
    }

    #[test]
    fn a_room_address_without_a_scheme_is_refused() {
        let settings = Settings { room_base_url: Some("example.test".into()), ..Settings::default() };
        assert!(settings.problem().is_some());
        assert!(Settings::default().problem().is_none());
    }
}
