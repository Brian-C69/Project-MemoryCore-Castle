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

pub const DEFAULT_WORLD: &str = "3d/floating-isles";
pub const CAMERAS: [&str; 4] = ["director", "follow", "wide", "orbit"];
pub const SHOTS: [&str; 5] = ["follow", "wide", "close", "orbit", "crane"];
pub const GLIDES: [&str; 3] = ["glide", "cut", "mix"];

/// A world id: "3d/<slug>", "2d/<slug>" or "my/<slug>" - lowercase letters, digits and dashes.
pub fn is_world_id(id: &str) -> bool {
    let Some((kind, slug)) = id.split_once('/') else { return false };
    ["3d", "2d", "my"].contains(&kind)
        && (1..=48).contains(&slug.len())
        && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !slug.starts_with('-')
}

/// One screen's WORLD: which world, how the camera films it, and whether it is on. The screen is
/// found again by its name, then by its size and place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorldScreen {
    pub screen: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub world: String,
    pub camera: String,
    pub on: bool,
    /// Each camera view holds hold_min..hold_max seconds.
    pub hold_min: u32,
    pub hold_max: u32,
    /// "glide", "cut" or "mix".
    pub glide: String,
    /// The views the director may use.
    pub shots: Vec<String>,
    /// 2D: seconds per picture.
    pub seconds: u32,
    pub shuffle: bool,
}

impl Default for WorldScreen {
    fn default() -> Self {
        Self {
            screen: String::new(),
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            world: DEFAULT_WORLD.into(),
            camera: "director".into(),
            on: false,
            hold_min: 8,
            hold_max: 20,
            glide: "mix".into(),
            shots: SHOTS.iter().map(|s| s.to_string()).collect(),
            seconds: 15,
            shuffle: true,
        }
    }
}

impl WorldScreen {
    pub fn normalized(mut self) -> Self {
        if !is_world_id(&self.world) {
            self.world = DEFAULT_WORLD.into();
        }
        if !CAMERAS.contains(&self.camera.as_str()) {
            self.camera = "director".into();
        }
        if !GLIDES.contains(&self.glide.as_str()) {
            self.glide = "mix".into();
        }
        self.hold_min = self.hold_min.clamp(3, 120);
        self.hold_max = self.hold_max.clamp(3, 120);
        if self.hold_min > self.hold_max {
            std::mem::swap(&mut self.hold_min, &mut self.hold_max);
        }
        let mut shots: Vec<String> = SHOTS.iter().filter(|s| self.shots.iter().any(|t| t == *s)).map(|s| s.to_string()).collect();
        if shots.is_empty() {
            shots = SHOTS.iter().map(|s| s.to_string()).collect();
        }
        self.shots = shots;
        self.seconds = self.seconds.clamp(4, 120);
        self
    }
}

/// The WORLD room: each screen's world, and the music.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorldSettings {
    pub music: bool,
    /// 0.0 to 1.0.
    pub volume: f32,
    /// Extra picture folders for the 2D slideshow, beside the memory folder's own pictures.
    pub folders: Vec<String>,
    /// One entry per screen the member has set; a screen with no entry stays off.
    pub screens: Vec<WorldScreen>,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self { music: true, volume: 0.5, folders: Vec::new(), screens: Vec::new() }
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
        self.world.screens = std::mem::take(&mut self.world.screens).into_iter().map(WorldScreen::normalized).collect();
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
            world: WorldSettings {
                music: false,
                volume: 0.25,
                folders: vec!["D:\\Pictures".into()],
                screens: vec![WorldScreen { screen: "\\\\.\\DISPLAY5".into(), world: "3d/crystal-isle".into(), camera: "orbit".into(), on: true, ..WorldScreen::default() }],
            },
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
            world: WorldSettings { volume: 3.0, ..WorldSettings::default() },
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.room_base_url.as_deref(), Some("https://example.test"));
        assert_eq!(settings.companion_name, None);
        assert_eq!(settings.world.volume, 1.0);
        assert_eq!(settings.companion_label(), "Your companion");
    }

    #[test]
    fn a_screen_with_odd_world_settings_is_brought_back_in_range() {
        let screen = WorldScreen {
            world: "3d/../secrets".into(),
            camera: "drone".into(),
            glide: "teleport".into(),
            hold_min: 500,
            hold_max: 1,
            shots: vec!["selfie".into()],
            seconds: 0,
            ..WorldScreen::default()
        }
        .normalized();
        assert_eq!(screen.world, DEFAULT_WORLD);
        assert_eq!(screen.camera, "director");
        assert_eq!(screen.glide, "mix");
        assert_eq!((screen.hold_min, screen.hold_max), (3, 120));
        assert_eq!(screen.shots.len(), SHOTS.len());
        assert_eq!(screen.seconds, 4);
        let kept = WorldScreen { shots: vec!["crane".into(), "close".into(), "close".into()], ..WorldScreen::default() }.normalized();
        assert_eq!(kept.shots, vec!["close".to_string(), "crane".to_string()]);
    }

    #[test]
    fn world_ids_are_a_kind_and_a_plain_slug() {
        assert!(is_world_id("3d/crystal-isle"));
        assert!(is_world_id("my/star-garden-2"));
        assert!(is_world_id("2d/mine"));
        assert!(!is_world_id("3d/Crystal"));
        assert!(!is_world_id("my/a\\b"));
        assert!(!is_world_id("my/"));
        assert!(!is_world_id("my/-x"));
        assert!(!is_world_id("5d/x"));
        assert!(!is_world_id("crystal-isle"));
    }

    #[test]
    fn a_new_member_has_every_screen_off() {
        assert!(Settings::default().world.screens.is_empty());
        assert!(!WorldScreen::default().on);
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
