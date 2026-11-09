//! The member's own 3D worlds, created from the WORLD room with their companion:
//!
//!   <memory>/worlds/<slug>/world.json   Castle's draft (pieces, roles, sizes, orbits); the
//!                                       companion polishes its words and colour only
//!   <memory>/worlds/<slug>/request.md   the member's words
//!   <memory>/worlds/<slug>/picture.*    a picture for the look (optional)
//!   <memory>/worlds/<slug>/design/      the companion's concept and one picture per piece
//!   <memory>/worlds/<slug>/hd/          the member's exports from Tripo (GLB, named by number)
//!   <memory>/worlds/<slug>/pieces/      the prepared pieces the world windows load
//!
//! Preparing runs tools/castle-world.ps1 (no session, no tokens).

use crate::session;
use crate::settings::SettingsStore;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, State};

const FOLDER: &str = "worlds";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyWorld {
    pub slug: String,
    pub folder: String,
    pub manifest: Value,
    /// pieces prepared and ready to load
    pub have: usize,
    /// pieces whose export is in hd/
    pub exports: usize,
    pub total: usize,
    /// the concept picture, else the member's picture
    pub preview: Option<String>,
    /// the companion has painted the piece pictures (design/ holds at least one)
    pub painted: bool,
}

/// A folder name from the member's world name: lowercase letters, digits and dashes.
pub fn slug_of(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(40).collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "my-world".into() } else { slug }
}

fn plain_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.len() <= 48 && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !slug.starts_with('-')
}

/// Castle's first draft of world.json for the layout the member chose.
pub fn draft(slug: &str, name: &str, description: &str, layout: &str) -> Value {
    let line: String = description.trim().chars().take(160).collect();
    if layout == "archipelago" {
        json!({
            "slug": slug, "title": name.trim(), "line": line, "layout": "archipelago",
            "camera": { "default": "director", "scale": 1 }, "glow": "#8fe6ff",
            "pieces": [
                { "file": "01-heart-isle.glb", "role": "isle", "isle": "heart", "size": 22 },
                { "file": "02-north-isle.glb", "role": "isle", "isle": "north", "size": 17 },
                { "file": "03-east-isle.glb", "role": "isle", "isle": "east", "size": 16 },
                { "file": "04-west-isle.glb", "role": "isle", "isle": "west", "size": 15 },
                { "file": "05-south-isle.glb", "role": "isle", "isle": "south", "size": 14 },
                { "file": "06-small-islets.glb", "role": "prop", "size": 8, "at": [18, 9, 20] }
            ]
        })
    } else {
        json!({
            "slug": slug, "title": name.trim(), "line": line, "layout": "centrepiece",
            "camera": { "default": "orbit", "scale": 3 }, "glow": "#8fe6ff",
            "pieces": [
                { "file": "01-base-isle.glb", "role": "base", "size": 40, "at": [0, 0, 0] },
                { "file": "02-centrepiece.glb", "role": "companion", "size": 9 },
                { "file": "03-islet-one.glb", "role": "islet", "size": 9, "orbit": { "radius": 38, "height": -4, "lap": 240, "phase": 0 } },
                { "file": "04-islet-two.glb", "role": "islet", "size": 8, "orbit": { "radius": 54, "height": 2, "lap": 300, "phase": 2.1 } },
                { "file": "05-islet-three.glb", "role": "islet", "size": 7, "orbit": { "radius": 46, "height": 8, "lap": 260, "phase": 4.2 } }
            ]
        })
    }
}

fn piece_files(manifest: &Value) -> Vec<String> {
    manifest["pieces"].as_array().map(|p| p.iter().filter_map(|x| x["file"].as_str().map(String::from)).collect()).unwrap_or_default()
}

/// The number a piece or an export starts with ("01-base.glb" -> 1, "1 my export.glb" -> 1).
fn number_of(name: &str) -> Option<u32> {
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// One world folder read back (None when it has no readable world.json).
pub fn read_world(dir: &Path) -> Option<MyWorld> {
    let slug = dir.file_name()?.to_str()?.to_string();
    if !plain_slug(&slug) {
        return None;
    }
    let text = fs::read_to_string(dir.join("world.json")).ok()?;
    let manifest: Value = serde_json::from_str(&text).ok()?;
    let files = piece_files(&manifest);
    let have = files.iter().filter(|f| dir.join("pieces").join(f).is_file()).count();
    let exported: Vec<u32> = fs::read_dir(dir.join("hd"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("glb")))
                .filter_map(|e| number_of(&e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default();
    let exports = files.iter().filter(|f| number_of(f).is_some_and(|n| exported.contains(&n))).count();
    let concept = dir.join("design").join("00-concept.png");
    let picture = ["png", "jpg", "jpeg", "webp"].iter().map(|e| dir.join(format!("picture.{e}"))).find(|p| p.is_file());
    let preview = if concept.is_file() { Some(concept) } else { picture };
    let painted = fs::read_dir(dir.join("design")).map(|mut e| e.next().is_some()).unwrap_or(false);
    Some(MyWorld {
        slug,
        folder: dir.display().to_string(),
        total: files.len(),
        manifest,
        have,
        exports,
        preview: preview.map(|p| p.display().to_string()),
        painted,
    })
}

pub fn list(memory: &Path) -> Vec<MyWorld> {
    let mut worlds: Vec<MyWorld> = fs::read_dir(memory.join(FOLDER))
        .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).filter_map(|e| read_world(&e.path())).collect())
        .unwrap_or_default();
    worlds.sort_by(|a, b| a.slug.cmp(&b.slug));
    worlds
}

/// Makes the world's folder with Castle's draft. A name already taken gets -2, -3, ...
pub fn create(memory: &Path, name: &str, description: &str, layout: &str, picture: Option<&Path>) -> Result<MyWorld, String> {
    if name.trim().is_empty() {
        return Err("Give the world a name.".into());
    }
    let base = slug_of(name);
    let root = memory.join(FOLDER);
    let mut slug = base.clone();
    let mut n = 2;
    while root.join(&slug).exists() {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let dir = root.join(&slug);
    for sub in ["design", "hd", "pieces"] {
        fs::create_dir_all(dir.join(sub)).map_err(|_| "The world's folder could not be made.".to_string())?;
    }
    let manifest = draft(&slug, name, description, layout);
    fs::write(dir.join("world.json"), serde_json::to_string_pretty(&manifest).unwrap_or_default()).map_err(|_| "world.json could not be written.".to_string())?;
    let request = format!(
        "# {}\n\n{}\n\nLayout: {}\n",
        name.trim(),
        if description.trim().is_empty() { "(no description given)" } else { description.trim() },
        if layout == "archipelago" { "an archipelago - five isles joined by bridges of light, small islets, the crystal walking between them" } else { "one centrepiece - a great base isle, the companion's centrepiece glowing at its heart, three islets circling" }
    );
    fs::write(dir.join("request.md"), request).map_err(|_| "request.md could not be written.".to_string())?;
    if let Some(picture) = picture {
        let ext = picture.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).filter(|e| ["png", "jpg", "jpeg", "webp"].contains(&e.as_str()));
        if let Some(ext) = ext {
            let _ = fs::copy(picture, dir.join(format!("picture.{ext}")));
        }
    }
    read_world(&dir).ok_or_else(|| "The new world could not be read back.".to_string())
}

fn memory_of(store: &SettingsStore) -> Result<PathBuf, String> {
    store.get().memory_folder.map(PathBuf::from).ok_or_else(|| "No MemoryCore folder is chosen yet.".to_string())
}

fn world_dir(store: &SettingsStore, slug: &str) -> Result<PathBuf, String> {
    if !plain_slug(slug) {
        return Err("That is not one of your worlds.".into());
    }
    let dir = memory_of(store)?.join(FOLDER).join(slug);
    if dir.is_dir() { Ok(dir) } else { Err("That world's folder is gone.".into()) }
}

#[tauri::command]
pub fn worlds_mine(store: State<SettingsStore>) -> Vec<MyWorld> {
    memory_of(&store).map(|m| list(&m)).unwrap_or_default()
}

#[tauri::command]
pub fn world_new(name: String, description: String, layout: String, picture: Option<String>, store: State<SettingsStore>) -> Result<MyWorld, String> {
    crate::rooms::need(&store, "world")?;
    let memory = memory_of(&store)?;
    create(&memory, &name, &description, &layout, picture.as_deref().map(Path::new))
}

#[tauri::command]
pub fn world_delete(slug: String, store: State<SettingsStore>) -> Result<(), String> {
    let dir = world_dir(&store, &slug)?;
    fs::remove_dir_all(dir).map_err(|_| "The world could not be removed (is a file in it open?).".to_string())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    slug: String,
    line: String,
    done: bool,
    ok: bool,
}

/// Prepares the member's exports in hd/ into pieces/, telling the room each line as it goes
/// (`world-prepare` events). No session and no tokens: Castle runs the helper itself.
#[tauri::command]
pub async fn world_prepare(app: AppHandle, slug: String, store: State<'_, SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "world")?;
    let dir = world_dir(&store, &slug)?;
    let helper = session::tools_folder(&app).join("castle-world.ps1");
    if !helper.is_file() {
        return Err("Castle's world helper is missing - reinstall Castle.".into());
    }
    let tell = move |app: &AppHandle, slug: &str, line: String, done: bool, ok: bool| {
        let _ = app.emit("world-prepare", Progress { slug: slug.to_string(), line, done, ok });
    };
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&helper)
            .arg("-Prepare")
            .arg("-Manifest")
            .arg(dir.join("world.json"))
            .arg("-In")
            .arg(dir.join("hd"))
            .arg("-Out")
            .arg(dir.join("pieces"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // no console window
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return tell(&app2, &slug, "PowerShell could not be started.".into(), true, false),
        };
        let mut last = String::new();
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                last = line.clone();
                tell(&app2, &slug, line, false, false);
            }
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        tell(&app2, &slug, last, true, ok);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("castle-worlds-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_world_name_becomes_a_plain_folder_name() {
        assert_eq!(slug_of("Star Garden!"), "star-garden");
        assert_eq!(slug_of("  ../../Windows  "), "windows");
        assert_eq!(slug_of("???"), "my-world");
        assert!(plain_slug(&slug_of("A Very Long Name That Goes On And On And On Beyond Forty Letters")));
        assert!(!plain_slug("../x"));
        assert!(!plain_slug("a\\b"));
    }

    #[test]
    fn the_draft_follows_the_layout() {
        let centre = draft("s", "Star Garden", "a garden of stars", "centrepiece");
        assert_eq!(centre["layout"], "centrepiece");
        assert_eq!(centre["camera"]["default"], "orbit");
        let roles: Vec<&str> = centre["pieces"].as_array().unwrap().iter().map(|p| p["role"].as_str().unwrap()).collect();
        assert_eq!(roles, vec!["base", "companion", "islet", "islet", "islet"]);
        let isles = draft("s", "Isles", "", "archipelago");
        assert_eq!(isles["pieces"].as_array().unwrap().iter().filter(|p| p["role"] == "isle").count(), 5);
        assert!(isles["pieces"].as_array().unwrap().iter().filter(|p| p["role"] == "isle").all(|p| p["isle"].is_string()));
    }

    #[test]
    fn creating_makes_the_folders_and_counts_the_pieces() {
        let memory = temp("create");
        let world = create(&memory, "Star Garden", "stars and moss", "centrepiece", None).unwrap();
        assert_eq!(world.slug, "star-garden");
        assert_eq!((world.have, world.exports, world.total), (0, 0, 5));
        assert!(memory.join("worlds/star-garden/request.md").is_file());
        // the same name again gets its own folder
        assert_eq!(create(&memory, "Star Garden", "", "centrepiece", None).unwrap().slug, "star-garden-2");
        // an export named by its number counts, a prepared piece counts
        let dir = memory.join("worlds/star-garden");
        fs::write(dir.join("hd/1 my base.glb"), b"x").unwrap();
        fs::write(dir.join("hd/notes.txt"), b"x").unwrap();
        fs::write(dir.join("pieces/02-centrepiece.glb"), b"x").unwrap();
        let back = read_world(&dir).unwrap();
        assert_eq!((back.have, back.exports, back.total), (1, 1, 5));
        assert_eq!(list(&memory).len(), 2);
    }

    #[test]
    fn a_folder_with_an_odd_name_or_no_world_json_is_not_a_world() {
        let memory = temp("odd");
        fs::create_dir_all(memory.join("worlds/Not Plain")).unwrap();
        fs::write(memory.join("worlds/Not Plain/world.json"), "{}").unwrap();
        fs::create_dir_all(memory.join("worlds/empty")).unwrap();
        assert!(list(&memory).is_empty());
    }
}
