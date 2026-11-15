//! LIBRARY in the memory database: the member's own books, one row each (`library_books`).
//!
//! ```text
//!  library_books   shelf · slug · title · body · source_file · created_at · updated_at
//! ```
//!
//! The old markdown (a `library/` folder, or the folder master memory names, one shelf per
//! sub-folder, loose files on "general") is taken in once: each file kept exactly in
//! `imported_files` first, then the rows, then the files go. Export writes them back (`to_files`).

use mysql::prelude::Queryable;
use mysql::TxOpts;
use serde::Serialize;
use std::path::{Path, PathBuf};

const SHELF_MAX: usize = 80;
const TITLE_MAX: usize = 200;
const BODY_MAX: usize = 4_000_000;
pub const LOOSE_SHELF: &str = "general";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: u64,
    pub shelf: String,
    pub slug: String,
    pub title: String,
    /// the first words, for the card
    pub excerpt: String,
    pub words: usize,
    pub updated_at: u64,
}

fn sql(what: &'static str) -> impl Fn(mysql::Error) -> String {
    move |_| format!("{what} failed in the memory database.")
}

fn one_line(text: &str, max: usize) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max).collect::<String>().trim().to_string()
}

/// A shelf's name as stored: lower-case words joined by dashes ("Design Patterns" -> "design-patterns").
pub fn shelf_slug(name: &str) -> String {
    if name.trim().is_empty() {
        return LOOSE_SHELF.to_string();
    }
    match crate::theme::theme_id(name) {
        s if s.is_empty() => LOOSE_SHELF.to_string(),
        s => s.chars().take(SHELF_MAX).collect(),
    }
}

/// A book's title: its first "# " heading, else its name in words. Pure.
pub fn title_of(body: &str, fallback: &str) -> String {
    body.lines()
        .find_map(|l| l.trim().strip_prefix("# ").map(|h| h.trim().to_string()))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback.replace(['-', '_'], " "))
}

/// The card's line: the first words that are not a heading. Pure.
pub fn excerpt_of(body: &str) -> String {
    let text: String = body.lines().filter(|l| !l.trim_start().starts_with('#')).collect::<Vec<_>>().join(" ");
    one_line(&text, 160)
}

fn book_of(row: (u64, String, String, String, String, Option<u64>)) -> Book {
    let (id, shelf, slug, title, body, at) = row;
    Book { id, shelf, slug, title, excerpt: excerpt_of(&body), words: body.split_whitespace().count(), updated_at: at.unwrap_or(0) }
}

/// Every book, by shelf then title.
pub fn books(conn: &mut impl Queryable) -> Result<Vec<Book>, String> {
    let rows: Vec<(u64, String, String, String, String, Option<u64>)> = conn
        .query("SELECT id, shelf, slug, title, body, CAST(UNIX_TIMESTAMP(updated_at) AS UNSIGNED) FROM library_books ORDER BY shelf, title")
        .map_err(sql("Reading the library"))?;
    Ok(rows.into_iter().map(book_of).collect())
}

/// One book's whole text.
pub fn read(conn: &mut impl Queryable, id: u64) -> Result<Option<(Book, String)>, String> {
    let row: Option<(u64, String, String, String, String, Option<u64>)> = conn
        .exec_first("SELECT id, shelf, slug, title, body, CAST(UNIX_TIMESTAMP(updated_at) AS UNSIGNED) FROM library_books WHERE id = ?", (id,))
        .map_err(sql("Reading a book"))?;
    Ok(row.map(|r| {
        let body = r.4.clone();
        (book_of(r), body)
    }))
}

/// A book on its shelf by its title (the slug is made from the title); a second with the same
/// title on the same shelf gets "-2". Returns the new book's id.
pub fn add(conn: &mut impl Queryable, shelf: &str, title: &str, body: &str) -> Result<u64, String> {
    let shelf = shelf_slug(shelf);
    let title = one_line(title, TITLE_MAX);
    if title.is_empty() {
        return Err("Give the book a title.".into());
    }
    let base = match crate::theme::theme_id(&title) {
        b if b.is_empty() => "book".to_string(),
        b => b.chars().take(100).collect(),
    };
    let taken: Vec<String> = conn.exec("SELECT slug FROM library_books WHERE shelf = ?", (&shelf,)).map_err(sql("Reading the shelf"))?;
    let mut slug = base.clone();
    let mut n = 2;
    while taken.contains(&slug) {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let body: String = body.chars().take(BODY_MAX).collect();
    conn.exec_drop("INSERT INTO library_books (shelf, slug, title, body) VALUES (?, ?, ?, ?)", (&shelf, &slug, &title, &body))
        .map_err(sql("Keeping the book"))?;
    let id: Option<u64> = conn.exec_first("SELECT id FROM library_books WHERE shelf = ? AND slug = ?", (&shelf, &slug)).map_err(sql("Reading the book back"))?;
    id.ok_or_else(|| "The book could not be read back.".to_string())
}

/// New words (and title) for a book.
pub fn set(conn: &mut impl Queryable, id: u64, title: Option<&str>, body: &str) -> Result<(), String> {
    let (book, _) = read(conn, id)?.ok_or("No such book.")?;
    let title = title.map(|t| one_line(t, TITLE_MAX)).filter(|t| !t.is_empty()).unwrap_or(book.title);
    let body: String = body.chars().take(BODY_MAX).collect();
    conn.exec_drop("UPDATE library_books SET title = ?, body = ? WHERE id = ?", (&title, &body, id)).map_err(sql("Saving the book"))
}

pub fn delete(conn: &mut impl Queryable, id: u64) -> Result<(), String> {
    conn.exec_drop("DELETE FROM library_books WHERE id = ?", (id,)).map_err(sql("Removing the book"))
}

/// The files Export writes: `library/<shelf>/<slug>.md`, as a memory kept them.
pub fn to_files(conn: &mut impl Queryable) -> Result<Vec<(String, String)>, String> {
    let rows: Vec<(String, String, String)> = conn.query("SELECT shelf, slug, body FROM library_books ORDER BY shelf, slug").map_err(sql("Reading the library"))?;
    Ok(rows.into_iter().map(|(shelf, slug, body)| (format!("library/{shelf}/{slug}.md"), body)).collect())
}

// ---------- taking the old files in, once ----------

fn md_under(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() && depth < 4 {
            md_under(&p, depth + 1, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("md") {
            out.push(p);
        }
    }
}

/// What the old library folder holds, read without changing anything: (relative path, shelf,
/// slug, title, body) for each book. A README at the top is not a book.
pub fn collect_files(memory: &Path) -> Vec<(String, String, String, String, String)> {
    let Some(root) = crate::memory::library_folder(memory) else { return Vec::new() };
    let mut files = Vec::new();
    md_under(&root, 0, &mut files);
    let mut found = Vec::new();
    for path in files {
        let Ok(body) = std::fs::read_to_string(&path) else { continue };
        let inside = path.strip_prefix(&root).unwrap_or(&path);
        let is_top = inside.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        if is_top && stem.eq_ignore_ascii_case("readme") {
            continue;
        }
        let shelf = if is_top { LOOSE_SHELF.to_string() } else { shelf_slug(&inside.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_default()) };
        let rel = path.strip_prefix(memory).map(|p| p.display().to_string().replace('\\', "/")).unwrap_or_default();
        found.push((rel, shelf, crate::theme::theme_id(&stem), title_of(&body, &stem), body));
    }
    found
}

/// The first time the library opens in the database: the old markdown becomes rows (each file
/// kept in `imported_files` first, in one transaction), then the files go. Runs once: a library
/// that already has rows is never taken in again. Returns how many books came in.
pub fn take_in(conn: &mut mysql::Conn, memory: &Path) -> Result<usize, String> {
    let have: Option<u64> = conn.query_first("SELECT COUNT(*) FROM library_books").map_err(sql("Reading the library"))?;
    let found = collect_files(memory);
    if have.unwrap_or(0) > 0 || found.is_empty() {
        return Ok(0);
    }
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the database"))?;
    for (rel, shelf, slug, title, body) in &found {
        tx.exec_drop(
            "INSERT INTO imported_files (path, body, sha256) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE body = VALUES(body), sha256 = VALUES(sha256), imported_at = NOW()",
            (rel, body, crate::db::sha256_hex(body.as_bytes())),
        )
        .map_err(sql("Keeping a file"))?;
        let slug = if slug.is_empty() { "book".to_string() } else { slug.clone() };
        tx.exec_drop(
            "INSERT INTO library_books (shelf, slug, title, body, source_file) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE title = VALUES(title), body = VALUES(body)",
            (shelf, &slug, one_line(title, TITLE_MAX), body, rel),
        )
        .map_err(sql("Taking in a book"))?;
    }
    tx.commit().map_err(sql("Keeping the library"))?;
    for (rel, ..) in &found {
        let _ = std::fs::remove_file(memory.join(rel));
    }
    Ok(found.len())
}

// ---------- the room's commands ----------

/// The member's memory folder, once the Library room is on.
fn memory_of(store: &crate::settings::SettingsStore) -> Result<PathBuf, String> {
    crate::rooms::need(store, "library")?;
    store.get().memory_folder.map(PathBuf::from).filter(|f| f.is_dir()).ok_or_else(|| "Choose your MemoryCore folder first.".to_string())
}

/// The memory database, migrated, with the old library taken in the first time.
fn open_at(memory: &Path) -> Result<mysql::Conn, String> {
    let mut conn = crate::store::source(memory).ok_or("Your library lives in the memory database. Move your memory in first (Settings > Database).")?;
    crate::memdb::migrate(&mut conn)?;
    take_in(&mut conn, memory)?;
    Ok(conn)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| "The library could not finish.".to_string())?
}

/// Every book of the member's own library (the old files taken in the first time).
#[tauri::command]
pub async fn library_books(store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<Vec<Book>, String> {
    let memory = memory_of(&store)?;
    blocking(move || books(&mut open_at(&memory)?)).await
}

/// One book's whole text.
#[tauri::command]
pub async fn library_read(id: u64, store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<String, String> {
    let memory = memory_of(&store)?;
    blocking(move || read(&mut open_at(&memory)?, id)?.map(|(_, body)| body).ok_or_else(|| "That book is gone.".to_string())).await
}

#[tauri::command]
pub async fn library_delete(id: u64, store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<Vec<Book>, String> {
    let memory = memory_of(&store)?;
    blocking(move || {
        let mut conn = open_at(&memory)?;
        delete(&mut conn, id)?;
        books(&mut conn)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_is_named_by_its_heading_and_carded_by_its_first_words() {
        assert_eq!(title_of("# Keys cannot destroy\n\nAn agent's key...", "x"), "Keys cannot destroy");
        assert_eq!(title_of("no heading", "ai-agent_keys"), "ai agent keys");
        assert_eq!(excerpt_of("# Title\n\nFirst   words here.\n## Next\nmore"), "First words here. more");
        assert_eq!(shelf_slug("Design Patterns"), "design-patterns");
        assert_eq!(shelf_slug("  "), "general");
    }

    #[test]
    fn the_old_library_is_read_shelf_by_shelf_without_changing_anything() {
        let root = std::env::temp_dir().join(format!("castle-library-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("library/pattern")).unwrap();
        std::fs::write(root.join("library/README.md"), "# The library\n").unwrap();
        std::fs::write(root.join("library/loose-note.md"), "# A loose note\nwords").unwrap();
        std::fs::write(root.join("library/pattern/ai-agent-keys.md"), "# Keys cannot destroy\nwords").unwrap();
        let found = collect_files(&root);
        assert_eq!(found.len(), 2, "the README is not a book");
        let pattern = found.iter().find(|f| f.1 == "pattern").unwrap();
        assert_eq!((pattern.0.as_str(), pattern.2.as_str(), pattern.3.as_str()), ("library/pattern/ai-agent-keys.md", "ai-agent-keys", "Keys cannot destroy"));
        assert!(found.iter().any(|f| f.1 == "general" && f.3 == "A loose note"));
        assert!(root.join("library/pattern/ai-agent-keys.md").is_file(), "collecting changes nothing");
        let _ = std::fs::remove_dir_all(root);
    }

    /// Against a real server, on demand (CASTLE_TEST_DB="<port>:<admin>:<password>"): take-in,
    /// add, set, delete and Export's files.
    #[test]
    #[ignore]
    fn live_the_library_taken_in_and_kept() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("set CASTLE_TEST_DB");
        let mut parts = spec.splitn(3, ':');
        let port: u16 = parts.next().unwrap().parse().unwrap();
        let (admin, admin_pw) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &admin, &admin_pw, &format!("Library Test {}", std::process::id())).unwrap();
        let mut conn = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut conn).unwrap();
        let root = std::env::temp_dir().join(format!("castle-library-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("library/pattern")).unwrap();
        std::fs::write(root.join("library/pattern/keys.md"), "# Keys\nwords").unwrap();
        assert_eq!(take_in(&mut conn, &root).unwrap(), 1);
        assert!(!root.join("library/pattern/keys.md").exists());
        let id = add(&mut conn, "Pattern", "Keys", "# Keys\nagain").unwrap();
        assert_eq!(read(&mut conn, id).unwrap().unwrap().0.slug, "keys-2");
        set(&mut conn, id, Some("Keys, again"), "# Keys, again\nnew").unwrap();
        assert_eq!(books(&mut conn).unwrap().len(), 2);
        delete(&mut conn, id).unwrap();
        assert_eq!(to_files(&mut conn).unwrap()[0].0, "library/pattern/keys.md");
        let _ = std::fs::remove_dir_all(root);
        conn.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
    }
}
