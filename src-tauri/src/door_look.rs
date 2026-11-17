//! The door's look verbs (`castle-memory look show | json | set`, `look item add | set`), kept
//! apart from door.rs: the companion reads its look in words, and changes it when the member asks.
//! The pictures are drawn from APPEARANCE; these verbs only touch the words.

use crate::appearance_db::{self, ItemRow};
use crate::door::Ask;
use mysql::Conn;
use std::path::Path;

/// The look verbs may run before APPEARANCE ever opened: the table made, the old notes taken in.
fn ready(conn: &mut Conn, memory: &Path) -> Result<(), String> {
    crate::memdb::migrate(conn)?;
    appearance_db::take_in(conn, memory).map(|_| ())
}

/// The words of a verb: `--words "..."`, or `--file <path>` / `--body "..."` for longer ones.
fn words(ask: &Ask) -> Result<Option<String>, String> {
    if let Some(w) = ask.get("words") {
        return Ok(Some(w.to_string()));
    }
    ask.text()
}

fn kind(ask: &Ask) -> Result<String, String> {
    let kind = ask.get("kind").unwrap_or("").trim().to_lowercase();
    let kind = if kind == "hairstyle" { "hair".to_string() } else { kind };
    if appearance_db::KINDS.contains(&kind.as_str()) { Ok(kind) } else { Err("--kind outfit or --kind hair is needed.".into()) }
}

fn item_line(i: &ItemRow) -> String {
    let words = i.words.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{} {} - {}{}", i.kind.to_uppercase(), i.slug, i.name, if words.is_empty() { String::new() } else { format!(": {words}") })
}

/// The whole look as text: the words, the art style, what is worn, the shelves.
pub fn look_show(conn: &mut Conn, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let look = appearance_db::read(conn)?.unwrap_or_default();
    let mut out = vec![
        format!("LOOK {}", if look.words.trim().is_empty() { "(no words yet - the member writes them in APPEARANCE)".to_string() } else { look.words.trim().to_string() }),
        format!("ART STYLE {}", if look.art_style.is_empty() { "(not chosen)" } else { &look.art_style }),
        format!("WEARING outfit {} · hair {}", look.outfit.as_deref().unwrap_or("none"), look.hair.as_deref().unwrap_or("none")),
    ];
    for kind in appearance_db::KINDS {
        out.extend(appearance_db::items(conn, kind)?.iter().map(item_line));
    }
    Ok(out.join("\n"))
}

/// castle-look.mjs's words file for what is worn (or `--outfit` / `--hair`), as JSON.
pub fn look_json(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let look = appearance_db::read(conn)?.unwrap_or_default();
    let outfit = ask.get("outfit").map(str::to_string).or(look.outfit);
    let hair = ask.get("hair").map(str::to_string).or(look.hair);
    let value = appearance_db::words_json(conn, outfit.as_deref(), hair.as_deref())?;
    Ok(serde_json::to_string_pretty(&value).unwrap_or_default())
}

/// New words or a new art style for the look (what is not given stays).
pub fn look_set(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let now = appearance_db::read(conn)?.unwrap_or_default();
    let new_words = words(ask)?;
    let new_style = ask.get("art-style").map(str::to_string);
    if new_words.is_none() && new_style.is_none() {
        return Err("--words (or --file) and/or --art-style is needed.".into());
    }
    let saved = appearance_db::save_look(conn, &new_words.unwrap_or(now.words), &new_style.unwrap_or(now.art_style))?;
    Ok(format!("LOOK saved ({} characters{})", saved.words.chars().count(), if saved.art_style.is_empty() { String::new() } else { format!(", art style {}", saved.art_style) }))
}

/// A new outfit or hairstyle on its shelf.
pub fn look_item_add(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let kind = kind(ask)?;
    let name = ask.get("name").ok_or("--name is needed.")?;
    let slug = appearance_db::add_item(conn, &kind, name, &words(ask)?.unwrap_or_default())?;
    std::fs::create_dir_all(memory.join("appearance").join(if kind == "outfit" { "wardrobe" } else { "hairstyles" }).join(&slug)).map_err(|_| "Its picture folder could not be made.".to_string())?;
    Ok(format!("LOOK {} added: {slug}", kind.to_uppercase()))
}

/// New name or words for an outfit or hairstyle (`--id` is its slug, as `look show` prints it).
pub fn look_item_set(conn: &mut Conn, ask: &Ask, memory: &Path) -> Result<String, String> {
    ready(conn, memory)?;
    let kind = kind(ask)?;
    let id = ask.get("id").ok_or("--id is needed (as `castle-memory look show` prints it).")?;
    let now = appearance_db::item(conn, &kind, id)?.ok_or("No such outfit or hairstyle. `castle-memory look show` lists them.")?;
    let item = appearance_db::set_item(conn, &kind, id, ask.get("name").unwrap_or(&now.name), &words(ask)?.unwrap_or(now.words))?;
    Ok(format!("LOOK {} saved: {}", item.kind.to_uppercase(), item.slug))
}
