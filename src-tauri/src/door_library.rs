//! The door's library verbs (`castle-memory library list | show | add | set`), kept apart from
//! door.rs: the companion's "save library" keeps a book as a row, never a markdown file.

use crate::door::Ask;
use crate::library_db;
use mysql::Conn;
use std::path::Path;

/// The verbs may run before the room ever opened: the table made, the old files taken in.
fn ready(conn: &mut Conn, memory: &Path) -> Result<(), String> {
    crate::memdb::migrate(conn)?;
    library_db::take_in(conn, memory).map(|_| ())
}

fn id(ask: &Ask) -> Result<u64, String> {
    ask.get("id").and_then(|v| v.trim().parse().ok()).ok_or("--id is needed (the number `castle-memory library list` prints).".into())
}

pub fn library_list(conn: &mut Conn, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let books = library_db::books(conn)?;
    if books.is_empty() {
        return Ok("LIBRARY empty".into());
    }
    Ok(books.iter().map(|b| format!("BOOK {} [{}] {} - {} words", b.id, b.shelf, b.title, b.words)).collect::<Vec<_>>().join("\n"))
}

pub fn library_show(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let (book, body) = library_db::read(conn, id(ask)?)?.ok_or("No such book. `castle-memory library list` lists them.")?;
    Ok(format!("BOOK {} [{}] {}\n\n{}", book.id, book.shelf, book.title, body))
}

/// `--shelf pattern --title "..." --file <path>` (or `--body "..."`).
pub fn library_add(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let body = ask.text()?.ok_or("--file <path> (or --body \"...\") is needed: the book's words.")?;
    let title = ask.get("title").map(str::to_string).unwrap_or_else(|| library_db::title_of(&body, "book"));
    let shelf = ask.get("shelf").unwrap_or(library_db::LOOSE_SHELF);
    let id = library_db::add(conn, shelf, &title, &body)?;
    Ok(format!("BOOK {id} kept on {}", library_db::shelf_slug(shelf)))
}

pub fn library_set(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let id = id(ask)?;
    let body = ask.text()?.ok_or("--file <path> (or --body \"...\") is needed: the new words.")?;
    library_db::set(conn, id, ask.get("title"), &body)?;
    Ok(format!("BOOK {id} saved"))
}
