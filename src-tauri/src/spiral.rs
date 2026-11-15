//! The Spiral Library: Kiyo's published books on the Portfolio, pulled only by Castle, read-only.
//!   GET  {portfolio}/api/public/library            shelves and their Public book cards
//!   GET  {portfolio}/api/member/read/{ref}         a signed-in member (their purchases, or an
//!                                                  approved Foundation member: every priced book)
//!   GET  {portfolio}/api/public/library/read/{ref} anyone else: a free book, or a priced one's preview
//! Nothing is written to the Portfolio and nothing is kept beyond the answer.

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpiralShelf {
    pub key: String,
    pub label: String,
    pub blurb: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpiralCard {
    #[serde(rename = "ref")]
    pub reference: String,
    pub shelf: String,
    pub title: String,
    pub subtitle: String,
    pub summary: String,
    pub cover_url: String,
    pub word_count: u64,
    pub reading_minutes: u64,
    pub free: bool,
    pub price_cents: u64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpiralRoom {
    pub shelves: Vec<SpiralShelf>,
    pub books: Vec<SpiralCard>,
    /// a member's token was used for reading (signed in with the Library card)
    pub signed_in: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpiralBook {
    #[serde(rename = "ref")]
    pub reference: String,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    /// only part of the book (a priced book read by someone who does not own it)
    pub preview: bool,
    pub owned: bool,
    pub free: bool,
    /// a free book that asks the reader to sign in for the rest
    pub join_to_read: bool,
    pub word_count: u64,
    pub preview_words: u64,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}
fn number(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}
fn flag(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The room's answer mapped to what Castle shows. Pure, tested.
pub fn room_of(answer: &Value, signed_in: bool) -> SpiralRoom {
    let list = |k: &str| answer.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    SpiralRoom {
        shelves: list("shelves").iter().map(|s| SpiralShelf { key: text(s, "key"), label: text(s, "label"), blurb: text(s, "blurb"), color: text(s, "color") }).collect(),
        books: list("books")
            .iter()
            .filter(|b| !text(b, "ref").is_empty())
            .map(|b| SpiralCard {
                reference: text(b, "ref"),
                shelf: text(b, "shelf"),
                title: text(b, "title"),
                subtitle: text(b, "subtitle"),
                summary: text(b, "summary"),
                cover_url: text(b, "coverUrl"),
                word_count: number(b, "wordCount"),
                reading_minutes: number(b, "readingMinutes"),
                free: flag(b, "free"),
                price_cents: number(b, "priceCents"),
                currency: text(b, "currency"),
            })
            .collect(),
        signed_in,
    }
}

/// A book's answer mapped to what Castle shows. Pure, tested.
pub fn book_of(answer: &Value) -> SpiralBook {
    SpiralBook {
        reference: text(answer, "ref"),
        title: text(answer, "title"),
        subtitle: text(answer, "subtitle"),
        body: text(answer, "body"),
        preview: flag(answer, "preview"),
        owned: flag(answer, "owned"),
        free: flag(answer, "free"),
        join_to_read: flag(answer, "joinToRead"),
        word_count: number(answer, "wordCount"),
        preview_words: number(answer, "previewWords"),
    }
}

/// A book's ref as a URL path: each segment percent-encoded, the slashes kept. Pure, tested.
pub fn ref_path(reference: &str) -> String {
    reference
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(|s| {
            s.bytes()
                .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("MemoryCoreCastle")
        .build()
        .map_err(|_| "The Spiral Library could not be reached.".to_string())
}

fn get(url: &str, token: Option<&str>) -> Result<Value, String> {
    let mut request = client()?.get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let answer = request.send().map_err(|_| "The Spiral Library could not be reached. Check the connection and try Refresh.".to_string())?;
    let status = answer.status().as_u16();
    let body: Value = answer.json().unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let said = body.get("error").and_then(Value::as_str).unwrap_or("the library refused the request");
        return Err(format!("The Spiral Library said: {}", said.chars().take(200).collect::<String>()));
    }
    Ok(body)
}

fn member_token() -> Option<String> {
    crate::key::load("member")
}

#[tauri::command]
pub async fn spiral_room(store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<SpiralRoom, String> {
    crate::rooms::need(&store, "library")?;
    tauri::async_runtime::spawn_blocking(|| {
        let base = crate::signin::portfolio();
        let answer = get(&format!("{base}/api/public/library"), None)?;
        Ok(room_of(&answer, member_token().is_some()))
    })
    .await
    .map_err(|_| "The Spiral Library could not finish.".to_string())?
}

/// One book: whole for a member who may read it, else what a guest may read.
#[tauri::command]
pub async fn spiral_read(reference: String, store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<SpiralBook, String> {
    crate::rooms::need(&store, "library")?;
    tauri::async_runtime::spawn_blocking(move || {
        let base = crate::signin::portfolio();
        let path = ref_path(&reference);
        if path.is_empty() {
            return Err("That is not a book of the Spiral Library.".to_string());
        }
        let answer = match member_token() {
            Some(token) => get(&format!("{base}/api/member/read/{path}"), Some(&token)).or_else(|_| get(&format!("{base}/api/public/library/read/{path}"), None))?,
            None => get(&format!("{base}/api/public/library/read/{path}"), None)?,
        };
        Ok(book_of(&answer))
    })
    .await
    .map_err(|_| "The Spiral Library could not finish.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_room_maps_shelves_and_public_cards() {
        let answer = serde_json::json!({
            "shelves": [{ "key": "pattern", "label": "Patterns", "blurb": "ways that work", "color": "#88f", "bookCount": 1 }],
            "books": [
                { "ref": "pattern/keys.md", "shelf": "pattern", "title": "Keys", "subtitle": "s", "summary": "why", "coverUrl": "https://x/c.png", "wordCount": 1200, "readingMinutes": 6, "free": false, "priceCents": 300, "currency": "usd" },
                { "title": "no ref" }
            ],
            "storeOpen": true
        });
        let room = room_of(&answer, true);
        assert_eq!(room.shelves[0].label, "Patterns");
        assert_eq!(room.books.len(), 1, "a card without a ref is left out");
        assert_eq!((room.books[0].reference.as_str(), room.books[0].price_cents, room.books[0].free), ("pattern/keys.md", 300, false));
        assert!(room.signed_in);
    }

    #[test]
    fn a_book_says_when_it_is_only_a_preview() {
        let b = book_of(&serde_json::json!({ "ref": "a.md", "title": "A", "body": "first part", "preview": true, "owned": false, "free": false, "wordCount": 1000, "previewWords": 100 }));
        assert!(b.preview && !b.owned);
        assert_eq!((b.word_count, b.preview_words), (1000, 100));
    }

    #[test]
    fn a_ref_keeps_its_slashes_and_nothing_climbs_out() {
        assert_eq!(ref_path("pattern/ai agent keys.md"), "pattern/ai%20agent%20keys.md");
        assert_eq!(ref_path("../../etc/passwd"), "etc/passwd");
        assert_eq!(ref_path(""), "");
    }
}
