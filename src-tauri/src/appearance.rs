//! APPEARANCE: the companion's look, kept in the memory folder so it belongs to the memory:
//!   appearance/companion.md          the look in words (seeded from master-memory's
//!                                    "## Character Appearance" when Image Prompt is installed)
//!   appearance/portrait.png          the standard appearance (also the panel's avatar)
//!   appearance/current.json          { outfit, hair } worn today
//!   appearance/wardrobe/<id>/        outfit.md + turnaround.png
//!   appearance/hairstyles/<id>/      hair.md + turnaround.png

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub name: String,
    pub note: String,
    /// the turnaround sheet, when it has been drawn (absolute path)
    pub sheet: Option<String>,
    /// a saved prompt waiting for a key (absolute path)
    pub saved_prompt: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Current {
    pub outfit: Option<String>,
    pub hair: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    pub folder: String,
    pub canon: String,
    pub portrait: Option<String>,
    pub wardrobe: Vec<Item>,
    pub hairstyles: Vec<Item>,
    pub current: Current,
}

fn root(memory: &Path) -> PathBuf {
    memory.join("appearance")
}

/// The words under master-memory's "## Character Appearance" (MemoryCore's Image Prompt feature).
pub fn canon_from_memory(memory: &Path) -> Option<String> {
    let text = crate::mirror::read_memory_text(memory, "master-memory.md")?;
    let start = text.lines().position(|l| l.trim_start().starts_with('#') && l.to_lowercase().contains("character appearance"))?;
    let level = text.lines().nth(start)?.chars().take_while(|c| *c == '#').count();
    let body: Vec<&str> = text
        .lines()
        .skip(start + 1)
        .take_while(|l| {
            let hashes = l.chars().take_while(|c| *c == '#').count();
            !(hashes > 0 && hashes <= level && l.chars().nth(hashes) == Some(' '))
        })
        .collect();
    let body = body.join("\n").trim().to_string();
    (!body.is_empty()).then_some(body)
}

/// Makes appearance/ with companion.md the first time (from the memory's own words when it has
/// them); never overwrites what is there.
pub fn ensure(memory: &Path, companion: &str) -> std::io::Result<()> {
    let root = root(memory);
    std::fs::create_dir_all(root.join("wardrobe"))?;
    std::fs::create_dir_all(root.join("hairstyles"))?;
    let canon = root.join("companion.md");
    if !canon.exists() {
        let words = canon_from_memory(memory).unwrap_or_else(|| {
            "Describe the look here: face, eyes, hair colour, build, the art style you like (for example anime, watercolor, semi-realistic), and anything that always stays the same.".into()
        });
        std::fs::write(canon, format!("# {companion}'s look\n\n{words}\n"))?;
    }
    Ok(())
}

fn items(dir: &Path, note_file: &str) -> Vec<Item> {
    let mut found: Vec<Item> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .map(|p| {
                    let note = std::fs::read_to_string(p.join(note_file)).unwrap_or_default();
                    let name = note
                        .lines()
                        .find_map(|l| l.trim().strip_prefix("# ").map(|h| h.trim().to_string()))
                        .unwrap_or_else(|| p.file_name().unwrap_or_default().to_string_lossy().replace('-', " "));
                    let body = note.lines().filter(|l| !l.trim_start().starts_with('#')).collect::<Vec<_>>().join(" ").trim().to_string();
                    let sheet = ["turnaround.png", "turnaround.jpg", "turnaround.webp"].iter().map(|n| p.join(n)).find(|f| f.is_file());
                    let prompt = p.join("turnaround.prompt.md");
                    Item {
                        id: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        name,
                        note: body,
                        sheet: sheet.map(|s| s.display().to_string()),
                        saved_prompt: prompt.is_file().then(|| prompt.display().to_string()),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

pub fn look(memory: &Path) -> Look {
    let root = root(memory);
    let portrait = ["portrait.png", "portrait.jpg", "portrait.webp"].iter().map(|n| root.join(n)).find(|p| p.is_file());
    Look {
        folder: root.display().to_string(),
        canon: std::fs::read_to_string(root.join("companion.md")).unwrap_or_default(),
        portrait: portrait.map(|p| p.display().to_string()),
        wardrobe: items(&root.join("wardrobe"), "outfit.md"),
        hairstyles: items(&root.join("hairstyles"), "hair.md"),
        current: std::fs::read_to_string(root.join("current.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(),
    }
}

/// A new outfit or hairstyle folder with its words; the companion draws its sheet into it.
pub fn new_item(memory: &Path, kind: &str, name: &str, description: &str) -> Result<String, String> {
    let (dir, note) = match kind {
        "outfit" => ("wardrobe", "outfit.md"),
        "hair" => ("hairstyles", "hair.md"),
        _ => return Err("Choose an outfit or a hairstyle.".into()),
    };
    if name.trim().is_empty() {
        return Err("Give it a name.".into());
    }
    let base = crate::theme::theme_id(name);
    let parent = root(memory).join(dir);
    let mut id = base.clone();
    let mut n = 2;
    while parent.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    std::fs::create_dir_all(parent.join(&id)).map_err(|_| "The folder could not be created.".to_string())?;
    std::fs::write(parent.join(&id).join(note), format!("# {}\n\n{}\n", name.trim(), description.trim()))
        .map_err(|_| "The description could not be written.".to_string())?;
    Ok(id)
}

pub fn set_current(memory: &Path, current: &Current) -> Result<(), String> {
    std::fs::create_dir_all(root(memory)).map_err(|_| "The appearance folder could not be created.".to_string())?;
    std::fs::write(root(memory).join("current.json"), serde_json::to_string_pretty(current).unwrap_or_default())
        .map_err(|_| "Today's look could not be saved.".to_string())
}

fn memory_of(store: &SettingsStore) -> Result<(PathBuf, String), String> {
    let s = store.get();
    let folder = s.memory_folder.clone().map(PathBuf::from).ok_or("No MemoryCore folder is chosen yet.")?;
    Ok((folder, s.companion_label()))
}

#[tauri::command]
pub fn appearance_look(store: State<SettingsStore>) -> Result<Look, String> {
    crate::rooms::need(&store, "appearance")?;
    let (memory, companion) = memory_of(&store)?;
    ensure(&memory, &companion).map_err(|_| "The appearance folder could not be created.".to_string())?;
    Ok(look(&memory))
}

#[tauri::command]
pub fn appearance_new(kind: String, name: String, description: String, store: State<SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "appearance")?;
    let (memory, companion) = memory_of(&store)?;
    ensure(&memory, &companion).map_err(|_| "The appearance folder could not be created.".to_string())?;
    new_item(&memory, &kind, &name, &description)
}

#[tauri::command]
pub fn appearance_wear(outfit: Option<String>, hair: Option<String>, store: State<SettingsStore>) -> Result<Look, String> {
    crate::rooms::need(&store, "appearance")?;
    let (memory, _) = memory_of(&store)?;
    set_current(&memory, &Current { outfit, hair })?;
    Ok(look(&memory))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("castle-look-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn the_memorys_own_appearance_words_seed_the_look_and_are_never_overwritten() {
        let memory = temp("seed");
        std::fs::write(
            memory.join("master-memory.md"),
            "# Master\n\n## Character Appearance\n- Silver hair, violet eyes\n- Soft anime style\n\n## Other\nx\n",
        )
        .unwrap();
        assert_eq!(canon_from_memory(&memory).as_deref(), Some("- Silver hair, violet eyes\n- Soft anime style"));
        ensure(&memory, "Lyra").unwrap();
        let canon = std::fs::read_to_string(memory.join("appearance").join("companion.md")).unwrap();
        assert!(canon.contains("Silver hair") && canon.starts_with("# Lyra's look"));
        std::fs::write(memory.join("appearance").join("companion.md"), "# mine\n").unwrap();
        ensure(&memory, "Lyra").unwrap();
        assert_eq!(std::fs::read_to_string(memory.join("appearance").join("companion.md")).unwrap(), "# mine\n");
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn outfits_and_hairstyles_are_listed_with_their_sheets_and_today_is_kept() {
        let memory = temp("items");
        ensure(&memory, "Lyra").unwrap();
        let id = new_item(&memory, "outfit", "Winter Coat", "a long navy coat").unwrap();
        assert_eq!(id, "winter-coat");
        assert_eq!(new_item(&memory, "outfit", "Winter Coat", "again").unwrap(), "winter-coat-2");
        new_item(&memory, "hair", "Twin Tails", "high twin tails").unwrap();
        std::fs::write(memory.join("appearance/wardrobe/winter-coat/turnaround.png"), b"png").unwrap();
        assert!(new_item(&memory, "hat", "x", "y").is_err());
        set_current(&memory, &Current { outfit: Some("winter-coat".into()), hair: Some("twin-tails".into()) }).unwrap();
        let look = look(&memory);
        assert_eq!(look.wardrobe.len(), 2);
        assert_eq!(look.wardrobe[0].name, "Winter Coat");
        assert!(look.wardrobe[0].sheet.is_some() && look.wardrobe[1].sheet.is_none());
        assert_eq!(look.hairstyles[0].note, "high twin tails");
        assert_eq!(look.current.outfit.as_deref(), Some("winter-coat"));
        assert!(look.portrait.is_none());
        let _ = std::fs::remove_dir_all(memory);
    }
}
