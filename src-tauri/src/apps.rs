//! The Apps room: the member's Start-menu apps, opened through the Windows shell, with how often
//! each is used.

use base64::Engine;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const USAGE_FILE: &str = "apps-usage.json";
const ICON_FOLDER: &str = "icons";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct App {
    pub name: String,
    /// The shortcut's full path; also its id.
    pub path: String,
    pub uses: u32,
    pub icon: Option<String>,
}

/// The two Start menu folders: every user's, then the member's own (theirs wins a name clash).
pub fn start_menu_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Some(all) = std::env::var_os("ProgramData") {
        folders.push(PathBuf::from(all).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Some(mine) = std::env::var_os("APPDATA") {
        folders.push(PathBuf::from(mine).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    folders
}

/// A shortcut worth showing: a .lnk or .url that is not an uninstaller.
fn is_app(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_lowercase();
    if extension != "lnk" && extension != "url" {
        return None;
    }
    let name = path.file_stem()?.to_str()?.trim().to_string();
    let lower = name.to_lowercase();
    if name.is_empty() || lower.contains("uninstall") {
        return None;
    }
    Some(name)
}

fn walk(folder: &Path, depth: usize, found: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < 4 {
                walk(&path, depth + 1, found);
            }
        } else if let Some(name) = is_app(&path) {
            found.push((name, path));
        }
    }
}

/// Every app in the folders, one per name (a later folder wins), sorted by name.
pub fn scan(folders: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut by_name: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for folder in folders {
        let mut found = Vec::new();
        walk(folder, 0, &mut found);
        for (name, path) in found {
            by_name.insert(name.to_lowercase(), (name, path));
        }
    }
    by_name.into_values().collect()
}

pub fn load_usage(settings_folder: &Path) -> HashMap<String, u32> {
    fs::read_to_string(settings_folder.join(USAGE_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn count_use(settings_folder: &Path, name: &str) -> std::io::Result<()> {
    let mut usage = load_usage(settings_folder);
    *usage.entry(name.to_string()).or_insert(0) += 1;
    fs::create_dir_all(settings_folder)?;
    fs::write(settings_folder.join(USAGE_FILE), serde_json::to_string(&usage).unwrap_or_default())
}

/// The icon file for a shortcut, named by a short hash of its path.
pub fn icon_path(settings_folder: &Path, shortcut: &Path) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    shortcut.to_string_lossy().to_lowercase().hash(&mut hasher);
    settings_folder.join("cache").join(ICON_FOLDER).join(format!("{:016x}.png", hasher.finish()))
}

fn icon_data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// The apps with their use counts and any cached icons, most used first among the used ones.
pub fn list(settings_folder: &Path, folders: &[PathBuf]) -> Vec<App> {
    let usage = load_usage(settings_folder);
    scan(folders)
        .into_iter()
        .map(|(name, path)| App {
            uses: usage.get(&name).copied().unwrap_or(0),
            icon: icon_data_url(&icon_path(settings_folder, &path)),
            path: path.to_string_lossy().into_owned(),
            name,
        })
        .collect()
}

/// Draws the icons not yet cached, through PowerShell's icon reader, with no window.
/// Returns how many were drawn. A failure leaves letter tiles.
pub fn draw_missing_icons(settings_folder: &Path, folders: &[PathBuf]) -> usize {
    let missing: Vec<(PathBuf, PathBuf)> = scan(folders)
        .into_iter()
        .map(|(_, shortcut)| {
            let target = icon_path(settings_folder, &shortcut);
            (shortcut, target)
        })
        .filter(|(_, target)| !target.exists())
        .collect();
    if missing.is_empty() {
        return 0;
    }
    let Some(icons) = missing.first().and_then(|(_, t)| t.parent().map(Path::to_path_buf)) else {
        return 0;
    };
    if fs::create_dir_all(&icons).is_err() {
        return 0;
    }
    // Pairs of shortcut and target, one per line, read by the script.
    let list = icons.join("pending.txt");
    let text: String = missing
        .iter()
        .map(|(s, t)| format!("{}|{}\n", s.display(), t.display()))
        .collect();
    if fs::write(&list, text).is_err() {
        return 0;
    }
    let script = format!(
        "Add-Type -AssemblyName System.Drawing; \
         foreach ($pair in Get-Content -LiteralPath '{}' -Encoding UTF8) {{ \
           $s, $t = $pair -split '\\|', 2; \
           try {{ $i = [System.Drawing.Icon]::ExtractAssociatedIcon($s); \
                 if ($i) {{ $b = $i.ToBitmap(); $b.Save($t, [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); $i.Dispose() }} }} \
           catch {{ }} }}",
        list.display().to_string().replace('\'', "''")
    );
    let mut command = std::process::Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let _ = command.status();
    let _ = fs::remove_file(&list);
    missing.iter().filter(|(_, t)| t.exists()).count()
}

/// Opens a shortcut through the Windows shell, the way a Start-menu click does.
pub fn open(shortcut: &Path) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(shortcut)
        .spawn()
        .map(|_| ())
        .map_err(|_| "The app could not be opened.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("castle-apps-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn shortcuts_are_found_named_deduplicated_and_uninstallers_left_out() {
        let everyone = temp("everyone");
        let mine = temp("mine");
        fs::create_dir_all(everyone.join("Tools")).unwrap();
        for (folder, file) in [
            (&everyone, "Google Chrome.lnk"),
            (&everyone, "Tools/Blender.lnk"),
            (&everyone, "Tools/Uninstall Blender.lnk"),
            (&everyone, "readme.txt"),
            (&mine, "google chrome.lnk"),
            (&mine, "Some Game.url"),
        ] {
            fs::write(folder.join(file), b"").unwrap();
        }
        let apps = scan(&[everyone.clone(), mine.clone()]);
        let names: Vec<_> = apps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Blender", "google chrome", "Some Game"]);
        // The member's own folder wins the clash.
        assert!(apps[1].1.starts_with(&mine));
    }

    #[test]
    fn uses_are_counted_and_listed() {
        let settings = temp("usage");
        let menu = temp("usage-menu");
        fs::write(menu.join("Steam.lnk"), b"").unwrap();
        count_use(&settings, "Steam").unwrap();
        count_use(&settings, "Steam").unwrap();
        let apps = list(&settings, &[menu]);
        assert_eq!(apps[0].uses, 2);
        assert_eq!(apps[0].icon, None);
    }

    /// Reads this computer's real Start menu and draws icons into a scratch folder.
    /// Run by hand: cargo test real_start_menu -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_start_menu_has_apps_and_icons() {
        let folders = start_menu_folders();
        let apps = scan(&folders);
        println!("apps found: {}", apps.len());
        assert!(!apps.is_empty());
        let scratch = temp("real-icons");
        let drawn = draw_missing_icons(&scratch, &folders);
        println!("icons drawn: {drawn} of {}", apps.len());
        assert!(drawn > 0);
        let _ = fs::remove_dir_all(&scratch);
    }
}
