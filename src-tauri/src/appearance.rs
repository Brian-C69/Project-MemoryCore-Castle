//! APPEARANCE: the companion's look. The words live in the memory database (`appearance_db`:
//! the look, its art style, what is worn today, the wardrobe and hairstyles); the pictures stay
//! files in the memory folder so they belong to the memory:
//!   appearance/portrait.png          the standard appearance (also the panel's avatar)
//!   appearance/wardrobe/<slug>/      turnaround.png, reference.png
//!   appearance/hairstyles/<slug>/    turnaround.png, reference.png
//!   appearance/try-on/<outfit>__<hair>.png   the drawn picture of one outfit with one hairstyle,
//!                                    kept so it is never drawn twice unless something it is made
//!                                    from has changed since

use crate::appearance_db::{self, ItemRow, LookRow};
use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// the slug: its picture folder's name
    pub id: String,
    pub name: String,
    /// its words
    pub note: String,
    /// the turnaround sheet, when it has been drawn (absolute path)
    pub sheet: Option<String>,
    pub reference: Option<String>,
    /// a saved prompt waiting for a key (absolute path)
    pub saved_prompt: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Current {
    pub outfit: Option<String>,
    pub hair: Option<String>,
}

/// The saved try-on picture for what is worn today.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TryOn {
    /// where the picture is (or will be) kept, absolute
    pub path: String,
    pub exists: bool,
    /// the outfit, hairstyle or standard look changed after the picture was drawn
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    pub folder: String,
    /// the look in words (empty until the member writes it)
    pub words: String,
    pub art_style: String,
    pub portrait: Option<String>,
    pub wardrobe: Vec<Item>,
    pub hairstyles: Vec<Item>,
    pub current: Current,
    /// None until an outfit or a hairstyle is worn
    pub try_on: Option<TryOn>,
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

/// The picture folders (the words are in the database).
pub fn ensure(memory: &Path) -> std::io::Result<()> {
    let root = root(memory);
    std::fs::create_dir_all(root.join("wardrobe"))?;
    std::fs::create_dir_all(root.join("hairstyles"))
}

fn shelf(kind: &str) -> &'static str {
    if kind == "outfit" { "wardrobe" } else { "hairstyles" }
}

/// One item as the room shows it: its words from the database, its pictures from its folder.
fn item_of(memory: &Path, row: &ItemRow) -> Item {
    let p = root(memory).join(shelf(&row.kind)).join(&row.slug);
    let sheet = ["turnaround.png", "turnaround.jpg", "turnaround.webp"].iter().map(|n| p.join(n)).find(|f| f.is_file());
    let prompt = p.join("turnaround.prompt.md");
    Item {
        id: row.slug.clone(),
        name: row.name.clone(),
        note: row.words.clone(),
        sheet: sheet.map(|s| s.display().to_string()),
        reference: p.join("reference.png").is_file().then(|| p.join("reference.png").display().to_string()),
        saved_prompt: prompt.is_file().then(|| prompt.display().to_string()),
    }
}

/// The newest change to any picture in a folder (the reference, the drawn sheet).
fn newest_in(dir: &Path) -> Option<SystemTime> {
    std::fs::read_dir(dir).ok()?.flatten().filter_map(|e| e.metadata().ok()?.modified().ok()).max()
}

fn at(secs: u64) -> Option<SystemTime> {
    (secs > 0).then(|| UNIX_EPOCH + Duration::from_secs(secs))
}

/// An outfit or hairstyle id is a folder name Castle made itself: letters, digits and dashes only,
/// so it can never climb out of its shelf (no dots, slashes or drive letters).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Where the picture of this outfit with this hairstyle is kept: one file per combination.
/// None when an id is not a plain folder name - such a file is never read.
pub fn try_on_path(memory: &Path, current: &Current) -> Option<PathBuf> {
    let part = |id: &Option<String>, none: &str| match id.as_deref() {
        None | Some("") => Some(none.to_string()),
        Some(id) => valid_id(id).then(|| id.to_string()),
    };
    let outfit = part(&current.outfit, "no-outfit")?;
    let hair = part(&current.hair, "no-hair")?;
    Some(root(memory).join("try-on").join(format!("{outfit}__{hair}.png")))
}

/// The saved try-on for what is worn, and whether it is still good. It is out of date once the
/// outfit's or hairstyle's words or pictures, or the standard look, changed after it was drawn.
pub fn try_on(memory: &Path, current: &Current, outfits: &[ItemRow], hairs: &[ItemRow]) -> Option<TryOn> {
    if current.outfit.is_none() && current.hair.is_none() {
        return None;
    }
    let root = root(memory);
    // try_on_path has checked both ids, so the folders below stay inside the shelves
    let path = try_on_path(memory, current)?;
    let drawn = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let mut inputs = vec![];
    for (id, rows, dir) in [(&current.outfit, outfits, "wardrobe"), (&current.hair, hairs, "hairstyles")] {
        if let Some(id) = id {
            inputs.push(rows.iter().find(|r| &r.slug == id).and_then(|r| at(r.updated_at)));
            inputs.push(newest_in(&root.join(dir).join(id)));
        }
    }
    for name in ["portrait.png", "portrait.jpg", "portrait.webp"] {
        inputs.push(std::fs::metadata(root.join(name)).ok().and_then(|m| m.modified().ok()));
    }
    let changed = inputs.into_iter().flatten().max();
    let stale = matches!((drawn, changed), (Some(drawn), Some(changed)) if changed > drawn);
    Some(TryOn { path: path.display().to_string(), exists: drawn.is_some(), stale })
}

/// The room's whole view: the rows, and the pictures found on disk.
pub fn build_look(memory: &Path, look: &LookRow, outfits: &[ItemRow], hairs: &[ItemRow]) -> Look {
    let root = root(memory);
    let current = Current { outfit: look.outfit.clone(), hair: look.hair.clone() };
    let portrait = ["portrait.png", "portrait.jpg", "portrait.webp"].iter().map(|n| root.join(n)).find(|p| p.is_file());
    Look {
        folder: root.display().to_string(),
        words: look.words.clone(),
        art_style: look.art_style.clone(),
        portrait: portrait.map(|p| p.display().to_string()),
        wardrobe: outfits.iter().map(|r| item_of(memory, r)).collect(),
        hairstyles: hairs.iter().map(|r| item_of(memory, r)).collect(),
        try_on: try_on(memory, &current, outfits, hairs),
        current,
    }
}

/// Validate and keep a local PNG copy. Replacements retain the previous image as a backup.
pub fn import_picture(memory: &Path, kind: &str, id: Option<&str>, source: &Path) -> Result<(), String> {
    let target = match kind {
        "standard" if id.is_none() => root(memory).join("portrait.png"),
        "outfit" | "hair" => {
            let id = id.filter(|s| valid_id(s))
                .ok_or("Choose an existing outfit or hairstyle.")?;
            let dir = if kind == "outfit" { "wardrobe" } else { "hairstyles" };
            root(memory).join(dir).join(id).join("reference.png")
        }
        _ => return Err("Choose a standard look, outfit or hairstyle.".into()),
    };
    let fence = memory.canonicalize().map_err(|_| "The memory folder is gone.")?;
    let parent = target.parent().unwrap().canonicalize().map_err(|_| "The appearance item is gone.")?;
    if !parent.starts_with(&fence) { return Err("The image destination is outside your memory.".into()); }
    if target.exists() {
        crate::files::fenced(memory, &target.display().to_string())?;
    }
    let size = std::fs::metadata(source).map_err(|_| "The picture is gone.")?.len();
    if size > 25 * 1024 * 1024 { return Err("Choose a picture smaller than 25 MB.".into()); }
    let mut reader = image::ImageReader::open(source).map_err(|_| "The picture could not be opened.")?
        .with_guessed_format().map_err(|_| "That file is not a supported picture.")?;
    if !matches!(reader.format(), Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)) {
        return Err("Choose a PNG, JPEG or WebP picture.".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let picture = reader.decode().map_err(|_| "The picture is damaged or too large to decode.")?;
    let mut encoded = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut encoded, image::ImageFormat::Png).map_err(|_| "The picture could not be converted.")?;
    if target.exists() {
        let mut n = 1;
        loop {
            let backup = parent.join(format!("{}-previous-{n}.png", target.file_stem().unwrap().to_string_lossy()));
            match std::fs::OpenOptions::new().write(true).create_new(true).open(backup) {
                Ok(mut file) => {
                    use std::io::Write;
                    let old = std::fs::read(&target).map_err(|_| "The previous image could not be read.")?;
                    file.write_all(&old).map_err(|_| "The previous image could not be preserved.")?;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
                Err(_) => return Err("The previous image could not be preserved.".into()),
            }
        }
    }
    // Write fully before publishing, so an interrupted import does not truncate the old image.
    let mut n = 1;
    loop {
        let pending = parent.join(format!("{}-import-{n}.tmp", target.file_stem().unwrap().to_string_lossy()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&pending) {
            Ok(mut file) => {
                use std::io::Write;
                let result = file.write_all(encoded.get_ref()).and_then(|_| file.sync_all());
                drop(file);
                if result.is_err() {
                    let _ = std::fs::remove_file(&pending);
                    return Err("The picture could not be saved.".into());
                }
                if std::fs::rename(&pending, &target).is_err() {
                    let _ = std::fs::remove_file(&pending);
                    return Err("The picture could not be replaced. Close any app holding it and try again.".into());
                }
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(_) => return Err("The picture could not be saved.".into()),
        }
    }
}

fn memory_of(store: &SettingsStore) -> Result<PathBuf, String> {
    store.get().memory_folder.map(PathBuf::from).filter(|f| f.is_dir()).ok_or_else(|| "No MemoryCore folder is chosen yet.".to_string())
}

/// The memory database, migrated, with the old notes taken in the first time.
fn open(memory: &Path) -> Result<mysql::Conn, String> {
    let mut conn = crate::store::source(memory).ok_or("APPEARANCE lives in the memory database. Move your memory in first (Settings > Database).")?;
    crate::memdb::migrate(&mut conn)?;
    appearance_db::take_in(&mut conn, memory)?;
    Ok(conn)
}

/// The look as the room shows it, read from the database.
pub fn look_from(conn: &mut impl mysql::prelude::Queryable, memory: &Path) -> Result<Look, String> {
    let row = appearance_db::read(conn)?.unwrap_or_default();
    let outfits = appearance_db::items(conn, "outfit")?;
    let hairs = appearance_db::items(conn, "hair")?;
    Ok(build_look(memory, &row, &outfits, &hairs))
}

/// Runs `work` with the database off the main thread, then answers with the whole look.
async fn with_look(store: &SettingsStore, work: impl FnOnce(&mut mysql::Conn, &Path) -> Result<(), String> + Send + 'static) -> Result<Look, String> {
    crate::rooms::need(store, "appearance")?;
    let memory = memory_of(store)?;
    tauri::async_runtime::spawn_blocking(move || {
        ensure(&memory).map_err(|_| "The appearance folder could not be created.")?;
        let mut conn = open(&memory)?;
        work(&mut conn, &memory)?;
        look_from(&mut conn, &memory)
    })
    .await
    .map_err(|_| "APPEARANCE could not finish.".to_string())?
}

#[tauri::command]
pub async fn appearance_look(store: State<'_, SettingsStore>) -> Result<Look, String> {
    with_look(&store, |_, _| Ok(())).await
}

#[tauri::command]
pub async fn appearance_import(kind: String, id: Option<String>, source: String, store: State<'_, SettingsStore>) -> Result<Look, String> {
    with_look(&store, move |_, memory| import_picture(memory, &kind, id.as_deref(), Path::new(&source))).await
}

/// A new outfit or hairstyle: its row, and its picture folder.
#[tauri::command]
pub async fn appearance_new(kind: String, name: String, description: String, store: State<'_, SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "appearance")?;
    let memory = memory_of(&store)?;
    tauri::async_runtime::spawn_blocking(move || {
        ensure(&memory).map_err(|_| "The appearance folder could not be created.")?;
        let mut conn = open(&memory)?;
        let slug = appearance_db::add_item(&mut conn, &kind, &name, &description)?;
        std::fs::create_dir_all(root(&memory).join(shelf(&kind)).join(&slug)).map_err(|_| "The picture folder could not be created.")?;
        Ok(slug)
    })
    .await
    .map_err(|_| "APPEARANCE could not finish.".to_string())?
}

#[tauri::command]
pub async fn appearance_wear(outfit: Option<String>, hair: Option<String>, store: State<'_, SettingsStore>) -> Result<Look, String> {
    with_look(&store, move |conn, _| appearance_db::wear(conn, outfit.as_deref(), hair.as_deref())).await
}

/// The look in words and its art style, as the member wrote them.
#[tauri::command]
pub async fn appearance_save_look(words: String, art_style: String, store: State<'_, SettingsStore>) -> Result<Look, String> {
    with_look(&store, move |conn, _| appearance_db::save_look(conn, &words, &art_style).map(|_| ())).await
}

/// New name or words for one outfit or hairstyle.
#[tauri::command]
pub async fn appearance_save_item(kind: String, id: String, name: String, words: String, store: State<'_, SettingsStore>) -> Result<Look, String> {
    with_look(&store, move |conn, _| appearance_db::set_item(conn, &kind, &id, &name, &words).map(|_| ())).await
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

    fn row(kind: &str, slug: &str, name: &str, words: &str, updated_at: u64) -> ItemRow {
        ItemRow { kind: kind.into(), slug: slug.into(), name: name.into(), words: words.into(), updated_at }
    }

    #[test]
    fn the_memorys_own_appearance_words_are_found_and_no_note_is_written() {
        let memory = temp("seed");
        std::fs::write(
            memory.join("master-memory.md"),
            "# Master\n\n## Character Appearance\n- Silver hair, violet eyes\n- Soft anime style\n\n## Other\nx\n",
        )
        .unwrap();
        assert_eq!(canon_from_memory(&memory).as_deref(), Some("- Silver hair, violet eyes\n- Soft anime style"));
        ensure(&memory).unwrap();
        assert!(memory.join("appearance/wardrobe").is_dir() && !memory.join("appearance/companion.md").exists());
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn outfits_and_hairstyles_are_listed_with_their_sheets_and_today_is_kept() {
        let memory = temp("items");
        ensure(&memory).unwrap();
        std::fs::create_dir_all(memory.join("appearance/wardrobe/winter-coat")).unwrap();
        std::fs::write(memory.join("appearance/wardrobe/winter-coat/turnaround.png"), b"png").unwrap();
        let outfits = vec![row("outfit", "winter-coat", "Winter Coat", "a long navy coat", 0), row("outfit", "winter-coat-2", "Winter Coat", "again", 0)];
        let hairs = vec![row("hair", "twin-tails", "Twin Tails", "high twin tails", 0)];
        let look_row = LookRow { words: "Violet eyes.".into(), art_style: "anime".into(), outfit: Some("winter-coat".into()), hair: Some("twin-tails".into()), updated_at: 0 };
        let look = build_look(&memory, &look_row, &outfits, &hairs);
        assert_eq!((look.words.as_str(), look.art_style.as_str()), ("Violet eyes.", "anime"));
        assert_eq!(look.wardrobe.len(), 2);
        assert_eq!(look.wardrobe[0].name, "Winter Coat");
        assert!(look.wardrobe[0].sheet.is_some() && look.wardrobe[1].sheet.is_none());
        assert_eq!(look.hairstyles[0].note, "high twin tails");
        assert_eq!(look.current.outfit.as_deref(), Some("winter-coat"));
        assert!(look.portrait.is_none());
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn a_drawn_try_on_is_kept_per_combination_and_goes_stale_only_when_its_parts_change() {
        let memory = temp("tryon");
        ensure(&memory).unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let both = Current { outfit: Some("winter-coat".into()), hair: Some("twin-tails".into()) };
        let outfits = vec![row("outfit", "winter-coat", "Winter Coat", "a navy coat", now - 300)];
        let hairs = vec![row("hair", "twin-tails", "Twin Tails", "high twin tails", now - 300)];
        assert!(try_on(&memory, &Current::default(), &outfits, &hairs).is_none(), "nothing worn, nothing to try on");

        let none_yet = try_on(&memory, &both, &outfits, &hairs).unwrap();
        assert!(!none_yet.exists && !none_yet.stale);
        assert!(none_yet.path.replace('\\', "/").ends_with("appearance/try-on/winter-coat__twin-tails.png"));
        assert_ne!(try_on_path(&memory, &both), try_on_path(&memory, &Current { outfit: Some("winter-coat".into()), hair: None }));

        // the picture is drawn after its parts were written
        let picture = PathBuf::from(&none_yet.path);
        std::fs::create_dir_all(picture.parent().unwrap()).unwrap();
        std::fs::write(&picture, b"png").unwrap();
        std::fs::File::options().write(true).open(&picture).unwrap().set_modified(UNIX_EPOCH + Duration::from_secs(now - 100)).unwrap();
        let kept = try_on(&memory, &both, &outfits, &hairs).unwrap();
        assert!(kept.exists && !kept.stale, "same outfit and hair: reuse the picture");

        // the outfit's words change after the picture
        let changed = vec![row("outfit", "winter-coat", "Winter Coat", "a red coat", now)];
        assert!(try_on(&memory, &both, &changed, &hairs).unwrap().stale);
        // a different outfit is a different picture, not a stale one
        let different = try_on(&memory, &Current { outfit: Some("summer-dress".into()), hair: Some("twin-tails".into()) }, &outfits, &hairs).unwrap();
        assert!(!different.exists);
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn imported_images_survive_replacement_and_are_listed_as_references() {
        let memory = temp("import");
        ensure(&memory).unwrap();
        let source = memory.join("source.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255])).save(&source).unwrap();
        import_picture(&memory, "standard", None, &source).unwrap();
        let original = std::fs::read(memory.join("appearance/portrait.png")).unwrap();
        import_picture(&memory, "standard", None, &source).unwrap();
        assert_eq!(std::fs::read(memory.join("appearance/portrait-previous-1.png")).unwrap(), original);
        for (kind, dir) in [("outfit", "wardrobe"), ("hair", "hairstyles")] {
            std::fs::create_dir_all(memory.join("appearance").join(dir).join("imported")).unwrap();
            import_picture(&memory, kind, Some("imported"), &source).unwrap();
        }
        let view = build_look(&memory, &LookRow::default(), &[row("outfit", "imported", "Imported", "", 0)], &[row("hair", "imported", "Imported", "", 0)]);
        assert!(view.portrait.is_some());
        assert!(view.wardrobe[0].reference.is_some() && view.hairstyles[0].reference.is_some());
        assert!(view.wardrobe[0].sheet.is_none());
        assert!(source.exists());
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn invalid_images_and_destinations_do_not_replace_the_portrait() {
        let memory = temp("invalid-import");
        ensure(&memory).unwrap();
        let source = memory.join("not-a-picture.png");
        std::fs::write(&source, b"not an image").unwrap();
        let target = memory.join("appearance/portrait.png");
        std::fs::write(&target, b"original").unwrap();
        assert!(import_picture(&memory, "standard", None, &source).is_err());
        assert!(import_picture(&memory, "outfit", Some("../escape"), &source).is_err());
        assert!(import_picture(&memory, "outfit", Some("missing"), &source).is_err());
        assert!(import_picture(&memory, "invalid", None, &source).is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"original");
        let _ = std::fs::remove_dir_all(memory);
    }

    #[test]
    fn a_crafted_id_never_becomes_a_try_on_path() {
        let memory = temp("crafted-id");
        ensure(&memory).unwrap();
        for bad in ["../../secret", "..", "a/b", "a\\b", "C:", "coat.png", "x y"] {
            let outfit = Current { outfit: Some(bad.into()), hair: None };
            let hair = Current { outfit: None, hair: Some(bad.into()) };
            assert!(try_on_path(&memory, &outfit).is_none(), "{bad} as an outfit");
            assert!(try_on_path(&memory, &hair).is_none(), "{bad} as a hairstyle");
            assert!(try_on(&memory, &outfit, &[], &[]).is_none() && try_on(&memory, &hair, &[], &[]).is_none());
        }
        // plain ids still work, and an empty id counts as nothing worn
        let fine = Current { outfit: Some("winter-coat-2".into()), hair: Some(String::new()) };
        assert!(try_on_path(&memory, &fine).unwrap().ends_with("winter-coat-2__no-hair.png"));
        let _ = std::fs::remove_dir_all(memory);
    }
}
