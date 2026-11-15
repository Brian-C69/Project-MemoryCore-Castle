//! APPEARANCE in the memory database: the look in words, the art style, what is worn today
//! (`appearance_look`, one row) and the wardrobe and hairstyles (`appearance_items`).
//!
//! ```text
//!  appearance_look   words · art_style · outfit_slug · hair_slug · updated_at
//!  appearance_items  kind (outfit|hair) · slug (its picture folder) · name · words · updated_at
//! ```
//!
//! The pictures stay files in `appearance/` (the portrait, `wardrobe/<slug>/turnaround.png`...).
//! The old markdown (`companion.md`, `outfit.md`, `hair.md`, `current.json`) is taken in once:
//! each file is kept exactly in `imported_files` first, then the rows are made, then the files go.
//! Export writes the same files back (`to_files`).

use mysql::prelude::Queryable;
use mysql::TxOpts;
use serde::Serialize;
use std::path::Path;

pub const KINDS: [&str; 2] = ["outfit", "hair"];
const NAME_MAX: usize = 80;
const WORDS_MAX: usize = 4000;
const STYLE_MAX: usize = 120;
/// The words Castle used to start companion.md with: they describe nobody, so they become no words.
pub const STARTING_WORDS: &str = "Describe the look here";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookRow {
    pub words: String,
    pub art_style: String,
    pub outfit: Option<String>,
    pub hair: Option<String>,
    /// seconds since 1970 (the try-on "changed since" check)
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRow {
    pub kind: String,
    pub slug: String,
    pub name: String,
    pub words: String,
    pub updated_at: u64,
}

fn sql(what: &'static str) -> impl Fn(mysql::Error) -> String {
    move |_| format!("{what} failed in the memory database.")
}

fn cut(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect::<String>().trim().to_string()
}

fn one_line(text: &str, max: usize) -> String {
    cut(&text.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

fn folder_of(kind: &str) -> &'static str {
    if kind == "outfit" { "wardrobe" } else { "hairstyles" }
}

fn note_of(kind: &str) -> &'static str {
    if kind == "outfit" { "outfit.md" } else { "hair.md" }
}

// ---------- the notes as they were on disk (pure) ----------

/// A note's heading and its words: `# Winter coat\n\nA long cream coat.` -> (Some("Winter coat"), "A long cream coat.").
pub fn split_note(text: &str) -> (Option<String>, String) {
    let mut lines = text.lines();
    let mut name = None;
    let mut rest: Vec<&str> = Vec::new();
    if let Some(first) = lines.next() {
        match first.trim_start().strip_prefix("# ") {
            Some(h) => name = Some(h.trim().to_string()),
            None => rest.push(first),
        }
    }
    rest.extend(lines);
    (name, rest.join("\n").trim().to_string())
}

/// companion.md's words without its heading; Castle's starting words become none.
pub fn canon_words(text: &str) -> String {
    let (_, body) = split_note(text);
    if body.starts_with(STARTING_WORDS) { String::new() } else { body }
}

/// The files Export writes for the look, as Castle used to keep them.
pub fn to_files(look: &LookRow, items: &[ItemRow], companion: &str) -> Vec<(String, String)> {
    let mut words = look.words.trim().to_string();
    if !look.art_style.trim().is_empty() {
        words = format!("{words}\n\nArt style: {}", look.art_style.trim()).trim().to_string();
    }
    let mut files = vec![("appearance/companion.md".to_string(), format!("# {companion}'s look\n\n{words}\n"))];
    for item in items {
        files.push((format!("appearance/{}/{}/{}", folder_of(&item.kind), item.slug, note_of(&item.kind)), format!("# {}\n\n{}\n", item.name, item.words.trim())));
    }
    if look.outfit.is_some() || look.hair.is_some() {
        let current = serde_json::json!({ "outfit": look.outfit, "hair": look.hair });
        files.push(("appearance/current.json".to_string(), serde_json::to_string_pretty(&current).unwrap_or_default()));
    }
    files
}

/// An "Art style: ..." line in the words (as Export writes it, or a member wrote it) is the art style.
pub fn art_style_line(words: &str) -> (String, String) {
    let mut style = String::new();
    let kept: Vec<&str> = words
        .lines()
        .filter(|l| match l.trim().strip_prefix("Art style:").or_else(|| l.trim().strip_prefix("art style:")) {
            Some(s) if style.is_empty() => {
                style = s.trim().trim_end_matches('.').to_string();
                false
            }
            _ => true,
        })
        .collect();
    (kept.join("\n").trim().to_string(), style)
}

// ---------- the rows ----------

pub fn read(conn: &mut impl Queryable) -> Result<Option<LookRow>, String> {
    // UNIX_TIMESTAMP converts in the server's own time zone, so it compares with file times
    let row: Option<(String, String, Option<String>, Option<String>, Option<u64>)> = conn
        .query_first("SELECT words, art_style, outfit_slug, hair_slug, CAST(UNIX_TIMESTAMP(updated_at) AS UNSIGNED) FROM appearance_look WHERE id = 1")
        .map_err(sql("Reading the look"))?;
    Ok(row.map(|(words, art_style, outfit, hair, at)| LookRow { words, art_style, outfit, hair, updated_at: at.unwrap_or(0) }))
}

pub fn items(conn: &mut impl Queryable, kind: &str) -> Result<Vec<ItemRow>, String> {
    let rows: Vec<(String, String, String, String, Option<u64>)> = conn
        .exec("SELECT kind, slug, name, words, CAST(UNIX_TIMESTAMP(updated_at) AS UNSIGNED) FROM appearance_items WHERE kind = ? ORDER BY name", (kind,))
        .map_err(sql("Reading the wardrobe"))?;
    Ok(rows.into_iter().map(|(kind, slug, name, words, at)| ItemRow { kind, slug, name, words, updated_at: at.unwrap_or(0) }).collect())
}

pub fn item(conn: &mut impl Queryable, kind: &str, slug: &str) -> Result<Option<ItemRow>, String> {
    Ok(items(conn, kind)?.into_iter().find(|i| i.slug == slug))
}

/// The look's words and art style (the row is made on the first save).
pub fn save_look(conn: &mut impl Queryable, words: &str, art_style: &str) -> Result<LookRow, String> {
    conn.exec_drop(
        "INSERT INTO appearance_look (id, words, art_style) VALUES (1, ?, ?) ON DUPLICATE KEY UPDATE words = VALUES(words), art_style = VALUES(art_style)",
        (cut(words, WORDS_MAX), one_line(art_style, STYLE_MAX)),
    )
    .map_err(sql("Saving the look"))?;
    read(conn)?.ok_or_else(|| "The look could not be read back.".to_string())
}

/// What is worn today; an id must be an item on its shelf.
pub fn wear(conn: &mut impl Queryable, outfit: Option<&str>, hair: Option<&str>) -> Result<(), String> {
    for (kind, slug) in [("outfit", outfit), ("hair", hair)] {
        if let Some(slug) = slug {
            if item(conn, kind, slug)?.is_none() {
                return Err("Choose an existing outfit or hairstyle.".into());
            }
        }
    }
    conn.exec_drop(
        "INSERT INTO appearance_look (id, words, outfit_slug, hair_slug) VALUES (1, '', ?, ?) ON DUPLICATE KEY UPDATE outfit_slug = VALUES(outfit_slug), hair_slug = VALUES(hair_slug)",
        (outfit, hair),
    )
    .map_err(sql("Saving today's look"))
}

/// A new outfit or hairstyle; returns its slug (a new picture folder's name).
pub fn add_item(conn: &mut impl Queryable, kind: &str, name: &str, words: &str) -> Result<String, String> {
    if !KINDS.contains(&kind) {
        return Err("Choose an outfit or a hairstyle.".into());
    }
    let name = one_line(name, NAME_MAX);
    if name.is_empty() {
        return Err("Give it a name.".into());
    }
    let base = match crate::theme::theme_id(&name) {
        b if b.is_empty() => kind.to_string(),
        b => b,
    };
    let taken: Vec<String> = conn.exec("SELECT slug FROM appearance_items WHERE kind = ?", (kind,)).map_err(sql("Reading the wardrobe"))?;
    let mut slug = base.clone();
    let mut n = 2;
    while taken.contains(&slug) {
        slug = format!("{base}-{n}");
        n += 1;
    }
    conn.exec_drop("INSERT INTO appearance_items (kind, slug, name, words) VALUES (?, ?, ?, ?)", (kind, &slug, &name, cut(words, WORDS_MAX)))
        .map_err(sql("Adding to the wardrobe"))?;
    Ok(slug)
}

/// New name or words for an item already on its shelf.
pub fn set_item(conn: &mut impl Queryable, kind: &str, slug: &str, name: &str, words: &str) -> Result<ItemRow, String> {
    let name = one_line(name, NAME_MAX);
    if name.is_empty() {
        return Err("Give it a name.".into());
    }
    if item(conn, kind, slug)?.is_none() {
        return Err("Choose an existing outfit or hairstyle.".into());
    }
    conn.exec_drop("UPDATE appearance_items SET name = ?, words = ? WHERE kind = ? AND slug = ?", (&name, cut(words, WORDS_MAX), kind, slug))
        .map_err(sql("Saving the words"))?;
    item(conn, kind, slug)?.ok_or_else(|| "The item could not be read back.".to_string())
}

/// castle-look.ps1's words file: the look, its art style, and the outfit and hairstyle named. Pure.
pub fn words_value(look: &LookRow, outfit: Option<&ItemRow>, hair: Option<&ItemRow>) -> serde_json::Value {
    let part = |i: Option<&ItemRow>| i.map(|i| serde_json::json!({ "name": i.name, "words": i.words }));
    serde_json::json!({ "words": look.words, "artStyle": look.art_style, "outfit": part(outfit), "hair": part(hair) })
}

/// The words file for a drawing of `outfit` with `hair`, read from the rows.
pub fn words_json(conn: &mut impl Queryable, outfit: Option<&str>, hair: Option<&str>) -> Result<serde_json::Value, String> {
    let look = read(conn)?.unwrap_or_default();
    let outfit = match outfit { Some(s) => item(conn, "outfit", s)?, None => None };
    let hair = match hair { Some(s) => item(conn, "hair", s)?, None => None };
    Ok(words_value(&look, outfit.as_ref(), hair.as_ref()))
}

// ---------- taking the old files in, once ----------

/// What the old files hold, read without changing anything: (relative path, body) of every file
/// that will be kept and removed, the look, and the items.
pub fn collect_files(memory: &Path) -> (Vec<(String, String)>, Option<LookRow>, Vec<(String, String, String, String)>) {
    let root = memory.join("appearance");
    let mut files = Vec::new();
    let mut look = None;
    if let Ok(text) = std::fs::read_to_string(root.join("companion.md")) {
        let (words, art_style) = art_style_line(&canon_words(&text));
        look = Some(LookRow { words, art_style, ..Default::default() });
        files.push(("appearance/companion.md".to_string(), text));
    }
    if let Ok(text) = std::fs::read_to_string(root.join("current.json")) {
        let current: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let id = |k: &str| current.get(k).and_then(|v| v.as_str()).filter(|s| crate::appearance::valid_id(s)).map(str::to_string);
        let row = look.get_or_insert_with(LookRow::default);
        row.outfit = id("outfit");
        row.hair = id("hair");
        files.push(("appearance/current.json".to_string(), text));
    }
    let mut items = Vec::new();
    for kind in KINDS {
        let Ok(entries) = std::fs::read_dir(root.join(folder_of(kind))) else { continue };
        let mut dirs: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for dir in dirs {
            let slug = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !crate::appearance::valid_id(&slug) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(dir.join(note_of(kind))) else { continue };
            let (name, words) = split_note(&text);
            items.push((kind.to_string(), slug.clone(), name.unwrap_or_else(|| slug.replace('-', " ")), words));
            files.push((format!("appearance/{}/{slug}/{}", folder_of(kind), note_of(kind)), text));
        }
    }
    (files, look, items)
}

/// The first time APPEARANCE opens in the database: the old notes become rows (each kept in
/// `imported_files` first, all in one transaction), then the files go. A memory with no notes
/// starts from master memory's "Character Appearance" when it has one. Returns whether it ran.
pub fn take_in(conn: &mut mysql::Conn, memory: &Path) -> Result<bool, String> {
    if read(conn)?.is_some() {
        return Ok(false);
    }
    let (files, look, items) = collect_files(memory);
    let look = look.unwrap_or_else(|| {
        let (words, art_style) = art_style_line(&crate::appearance::canon_from_memory(memory).unwrap_or_default());
        LookRow { words, art_style, ..Default::default() }
    });
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the database"))?;
    for (path, body) in &files {
        tx.exec_drop(
            "INSERT INTO imported_files (path, body, sha256) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE body = VALUES(body), sha256 = VALUES(sha256), imported_at = NOW()",
            (path, body, crate::db::sha256_hex(body.as_bytes())),
        )
        .map_err(sql("Keeping a file"))?;
    }
    for (kind, slug, name, words) in &items {
        tx.exec_drop(
            "INSERT INTO appearance_items (kind, slug, name, words) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name), words = VALUES(words)",
            (kind, slug, one_line(name, NAME_MAX), cut(words, WORDS_MAX)),
        )
        .map_err(sql("Taking in the wardrobe"))?;
    }
    tx.exec_drop(
        "INSERT INTO appearance_look (id, words, art_style, outfit_slug, hair_slug) VALUES (1, ?, ?, ?, ?)",
        (cut(&look.words, WORDS_MAX), one_line(&look.art_style, STYLE_MAX), &look.outfit, &look.hair),
    )
    .map_err(sql("Taking in the look"))?;
    tx.commit().map_err(sql("Keeping the look"))?;
    for (path, _) in &files {
        let _ = std::fs::remove_file(memory.join(path));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_its_heading_and_its_words() {
        assert_eq!(split_note("# Winter coat\n\nA long cream coat.\n"), (Some("Winter coat".into()), "A long cream coat.".into()));
        assert_eq!(split_note("Just words"), (None, "Just words".into()));
    }

    #[test]
    fn the_starting_words_become_none_and_the_art_style_is_its_own() {
        assert_eq!(canon_words("# Mira's look\n\nDescribe the look here: face, eyes.\n"), "");
        assert_eq!(art_style_line("Violet eyes.\nArt style: soft watercolor.\nSlender."), ("Violet eyes.\nSlender.".into(), "soft watercolor".into()));
        assert_eq!(art_style_line("Violet eyes."), ("Violet eyes.".into(), String::new()));
    }

    #[test]
    fn export_writes_the_files_castle_used_to_keep_and_they_read_back() {
        let look = LookRow { words: "Violet eyes.".into(), art_style: "anime".into(), outfit: Some("coat".into()), hair: None, updated_at: 0 };
        let items = vec![ItemRow { kind: "outfit".into(), slug: "coat".into(), name: "Winter coat".into(), words: "Cream wool.".into(), updated_at: 0 }];
        let files = to_files(&look, &items, "Mira");
        assert_eq!(files[0], ("appearance/companion.md".into(), "# Mira's look\n\nViolet eyes.\n\nArt style: anime\n".into()));
        assert_eq!(files[1], ("appearance/wardrobe/coat/outfit.md".into(), "# Winter coat\n\nCream wool.\n".into()));
        assert!(files[2].0 == "appearance/current.json" && files[2].1.contains("\"coat\""));
        assert_eq!(art_style_line(&canon_words(&files[0].1)), ("Violet eyes.".into(), "anime".into()));
    }

    #[test]
    fn the_words_file_names_the_look_and_the_items_worn() {
        let look = LookRow { words: "Violet eyes.".into(), art_style: "anime".into(), ..Default::default() };
        let coat = ItemRow { kind: "outfit".into(), slug: "coat".into(), name: "Winter coat".into(), words: "Cream wool.".into(), updated_at: 0 };
        let v = words_value(&look, Some(&coat), None);
        assert_eq!(v["words"], "Violet eyes.");
        assert_eq!(v["artStyle"], "anime");
        assert_eq!(v["outfit"]["name"], "Winter coat");
        assert!(v["hair"].is_null());
    }

    #[test]
    fn the_old_files_are_read_without_changing_anything() {
        let root = std::env::temp_dir().join(format!("castle-look-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = root.join("appearance");
        std::fs::create_dir_all(a.join("wardrobe/winter-coat")).unwrap();
        std::fs::create_dir_all(a.join("hairstyles/long-bob")).unwrap();
        std::fs::write(a.join("companion.md"), "# Mira's look\n\nViolet eyes.\nArt style: anime\n").unwrap();
        std::fs::write(a.join("current.json"), r#"{"outfit":"winter-coat","hair":"../x"}"#).unwrap();
        std::fs::write(a.join("wardrobe/winter-coat/outfit.md"), "# Winter coat\n\nCream wool.\n").unwrap();
        std::fs::write(a.join("hairstyles/long-bob/hair.md"), "# Long bob\n\nCurled in.\n").unwrap();
        let (files, look, items) = collect_files(&root);
        let look = look.unwrap();
        assert_eq!((look.words.as_str(), look.art_style.as_str(), look.outfit.as_deref(), look.hair.as_deref()), ("Violet eyes.", "anime", Some("winter-coat"), None));
        assert_eq!(items.len(), 2);
        assert_eq!(files.len(), 4);
        assert!(a.join("companion.md").is_file(), "collecting changes nothing");
        let _ = std::fs::remove_dir_all(root);
    }

    /// Against a real server, on demand (CASTLE_TEST_DB="<port>:<admin>:<password>"): the take-in
    /// keeps the files, makes the rows and removes the notes; save, add, edit and wear.
    #[test]
    #[ignore]
    fn live_the_look_taken_in_and_kept() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("set CASTLE_TEST_DB");
        let mut parts = spec.splitn(3, ':');
        let port: u16 = parts.next().unwrap().parse().unwrap();
        let (admin, admin_pw) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &admin, &admin_pw, &format!("Look Test {}", std::process::id())).unwrap();
        let mut conn = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut conn).unwrap();

        let root = std::env::temp_dir().join(format!("castle-look-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("appearance/wardrobe/winter-coat")).unwrap();
        std::fs::write(root.join("appearance/companion.md"), "# Mira's look\n\nViolet eyes.\n").unwrap();
        std::fs::write(root.join("appearance/wardrobe/winter-coat/outfit.md"), "# Winter coat\n\nCream wool.\n").unwrap();
        std::fs::write(root.join("appearance/wardrobe/winter-coat/turnaround.png"), b"x").unwrap();

        assert!(take_in(&mut conn, &root).unwrap());
        assert!(!take_in(&mut conn, &root).unwrap(), "only once");
        assert!(!root.join("appearance/companion.md").exists() && root.join("appearance/wardrobe/winter-coat/turnaround.png").is_file());
        let kept: Option<String> = conn.query_first("SELECT body FROM imported_files WHERE path = 'appearance/companion.md'").unwrap();
        assert!(kept.unwrap().contains("Violet eyes."));
        assert_eq!(read(&mut conn).unwrap().unwrap().words, "Violet eyes.");

        save_look(&mut conn, "Red hair, amber eyes.", "anime").unwrap();
        let slug = add_item(&mut conn, "hair", "Long bob", "Curled in.").unwrap();
        assert_eq!(slug, "long-bob");
        assert_eq!(add_item(&mut conn, "hair", "Long bob", "Again.").unwrap(), "long-bob-2");
        set_item(&mut conn, "hair", "long-bob", "Long bob", "Curled in, side part.").unwrap();
        wear(&mut conn, Some("winter-coat"), Some("long-bob")).unwrap();
        assert!(wear(&mut conn, Some("nothing"), None).is_err());
        let look = read(&mut conn).unwrap().unwrap();
        assert_eq!((look.art_style.as_str(), look.outfit.as_deref(), look.hair.as_deref()), ("anime", Some("winter-coat"), Some("long-bob")));
        assert_eq!(items(&mut conn, "hair").unwrap()[0].words, "Curled in, side part.");

        let _ = std::fs::remove_dir_all(root);
        conn.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
    }
}
