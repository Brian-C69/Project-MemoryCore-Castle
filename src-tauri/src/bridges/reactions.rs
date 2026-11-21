//! A heart on the companion's bubble: when the member reacts on the phone to a reply the bridge
//! sent, Chat shows that emoji on the companion's own bubble. The reaction is only shown - it is
//! never typed into the session, so it never wakes the companion or costs a turn.
//!
//! The bridge remembers the last replies it sent per platform (the platform's message id and the
//! first words of the reply) in `phone-reactions.json` in the settings folder, beside the
//! reactions themselves. A reaction names a message id; the words it finds there are what Chat
//! matches against the companion's bubbles.

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

/// How many sent replies are remembered per platform.
const SENT_KEEP: usize = 50;
/// How many reactions are kept at all; the oldest go first.
const REACTIONS_KEEP: usize = 200;
/// How much of a reply's words a reaction carries (what Chat matches a bubble by).
pub const WORDS_KEEP: usize = 60;
pub const EVENT: &str = "castle://phone-reaction";

/// One file, read and written by one hand at a time.
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    pub platform: String,
    pub id: String,
    pub words: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PhoneReaction {
    pub platform: String,
    /// the first words of the reply it is on
    pub words: String,
    pub emoji: String,
    /// seconds since 1970
    pub at: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    #[serde(default)]
    pub sent: Vec<Sent>,
    #[serde(default)]
    pub reactions: Vec<PhoneReaction>,
}

/// The first words of a reply, as a reaction carries them.
pub fn opening(words: &str) -> String {
    words.trim().chars().take(WORDS_KEEP).collect()
}

impl Book {
    /// Notes a reply that was just sent; only the newest `SENT_KEEP` per platform stay.
    pub fn remember(&mut self, platform: &str, id: &str, words: &str) {
        if id.is_empty() || words.trim().is_empty() {
            return;
        }
        self.sent.retain(|s| !(s.platform == platform && s.id == id));
        self.sent.push(Sent { platform: platform.into(), id: id.into(), words: opening(words) });
        let mine = self.sent.iter().filter(|s| s.platform == platform).count();
        if mine > SENT_KEEP {
            let mut drop = mine - SENT_KEEP;
            self.sent.retain(|s| {
                if drop > 0 && s.platform == platform {
                    drop -= 1;
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Puts a reaction on the reply with that message id: a newer one replaces the older, an
    /// empty emoji takes it back. None when the id is not one of the remembered replies.
    pub fn react(&mut self, platform: &str, id: &str, emoji: &str, at: u64) -> Option<PhoneReaction> {
        let words = self.sent.iter().find(|s| s.platform == platform && s.id == id)?.words.clone();
        self.reactions.retain(|r| !(r.platform == platform && r.words == words));
        let reaction = PhoneReaction { platform: platform.into(), words, emoji: emoji.trim().into(), at };
        if !reaction.emoji.is_empty() {
            self.reactions.push(reaction.clone());
            if self.reactions.len() > REACTIONS_KEEP {
                let extra = self.reactions.len() - REACTIONS_KEEP;
                self.reactions.drain(..extra);
            }
        }
        Some(reaction)
    }
}

fn path(store: &SettingsStore) -> PathBuf {
    store.folder().join("phone-reactions.json")
}

fn read(store: &SettingsStore) -> Book {
    std::fs::read_to_string(path(store)).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn write(store: &SettingsStore, book: &Book) {
    if let Ok(text) = serde_json::to_string_pretty(book) {
        let _ = std::fs::write(path(store), text);
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Remembers a reply the bridge just sent to the member, so a reaction on it can be placed.
pub fn remember(app: &AppHandle, platform: &str, id: &str, words: &str) {
    let store = app.state::<SettingsStore>();
    let _guard = LOCK.lock().unwrap();
    let mut book = read(&store);
    book.remember(platform, id, words);
    write(&store, &book);
}

/// The member reacted on the phone: kept, and Chat is told at once.
pub fn record(app: &AppHandle, platform: &str, id: &str, emoji: &str) {
    let store = app.state::<SettingsStore>();
    let reaction = {
        let _guard = LOCK.lock().unwrap();
        let mut book = read(&store);
        let Some(reaction) = book.react(platform, id, emoji, now()) else { return };
        write(&store, &book);
        reaction
    };
    let _ = app.emit(EVENT, reaction);
}

/// Every reaction kept, for Chat to place when it opens.
#[tauri::command]
pub fn phone_reactions(store: State<SettingsStore>) -> Vec<PhoneReaction> {
    let _guard = LOCK.lock().unwrap();
    read(&store).reactions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reaction_lands_on_the_remembered_reply_and_a_newer_one_replaces_it() {
        let mut book = Book::default();
        book.remember("telegram", "11", "Good evening! The shop opens at ten tomorrow, and I will remind you.");
        let first = book.react("telegram", "11", "❤", 5).unwrap();
        assert_eq!(first.words, "Good evening! The shop opens at ten tomorrow, and I will rem");
        assert_eq!(first.words.chars().count(), WORDS_KEEP);
        book.react("telegram", "11", "👍", 6).unwrap();
        assert_eq!(book.reactions.len(), 1);
        assert_eq!(book.reactions[0].emoji, "👍");
    }

    #[test]
    fn an_empty_emoji_takes_the_reaction_back_and_an_unknown_id_is_left_alone() {
        let mut book = Book::default();
        book.remember("whatsapp", "ABC", "Hi");
        book.react("whatsapp", "ABC", "❤️", 1).unwrap();
        let back = book.react("whatsapp", "ABC", "", 2).unwrap();
        assert!(back.emoji.is_empty());
        assert!(book.reactions.is_empty());
        assert!(book.react("whatsapp", "nope", "❤️", 3).is_none());
        // the same id on another platform is another message
        assert!(book.react("telegram", "ABC", "❤️", 3).is_none());
    }

    #[test]
    fn only_the_newest_fifty_replies_per_platform_are_remembered() {
        let mut book = Book::default();
        book.remember("discord", "keep", "a discord reply");
        for n in 0..60 {
            book.remember("telegram", &n.to_string(), &format!("reply {n}"));
        }
        assert_eq!(book.sent.iter().filter(|s| s.platform == "telegram").count(), SENT_KEEP);
        assert!(book.react("telegram", "0", "❤", 1).is_none(), "the oldest was let go");
        assert!(book.react("telegram", "59", "❤", 1).is_some());
        assert!(book.react("discord", "keep", "❤", 1).is_some(), "another platform's replies stay");
        // nothing to remember without an id or words
        book.remember("telegram", "", "words");
        book.remember("telegram", "x", "   ");
        assert!(book.react("telegram", "x", "❤", 1).is_none());
    }

    #[test]
    fn the_book_reads_back_what_it_wrote_and_an_old_file_without_fields_is_fine() {
        let mut book = Book::default();
        book.remember("telegram", "1", "hello");
        book.react("telegram", "1", "❤", 9);
        let text = serde_json::to_string(&book).unwrap();
        assert_eq!(serde_json::from_str::<Book>(&text).unwrap(), book);
        assert_eq!(serde_json::from_str::<Book>("{}").unwrap(), Book::default());
    }
}
