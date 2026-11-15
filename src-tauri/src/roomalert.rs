//! A new line in the Foundation room: a sound and a desktop notification when the member is not
//! looking at the CHAT tab. The notification is on by default and the sound off (the sound ignores
//! Windows quiet hours); each has its own switch in Settings > Chat, with the choice of every line
//! or only the lines that name the member or their companion.
//! The room reader (roomread.rs) runs from the moment Castle starts, also while Castle is hidden in
//! the tray, so this reaches the member then too. An alert is best effort: it never stops a read.

use crate::settings::{Settings, SettingsStore};
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const PREVIEW: usize = 80;
/// Told to the page when the member clicks the notification, so it shows the CHAT tab.
#[cfg(windows)]
const OPEN_CHAT_EVENT: &str = "castle-open-chat";

/// The notification for a batch of new lines, leaving out lines the member posted from Castle
/// (`mine`, by id): (title, body), or None when nothing is left to tell. Pure, tested.
/// The line answers one of `mine`. Pure.
pub fn replies_to(line: &Value, mine: &[u64]) -> bool {
    line.pointer("/replyTo/id").and_then(Value::as_u64).is_some_and(|id| mine.contains(&id))
}

fn reaction(line: &Value, emoji: &str) -> (u64, bool) {
    line.get("reactions")
        .and_then(Value::as_array)
        .and_then(|r| r.iter().find(|x| x.get("emoji").and_then(Value::as_str) == Some(emoji)))
        .map(|x| (x.get("count").and_then(Value::as_u64).unwrap_or(0), x.get("mine").and_then(Value::as_bool).unwrap_or(false)))
        .unwrap_or((0, false))
}

/// The news of reactions on the member's own lines (before, after): only a count that grew by
/// someone else's tap, never the member's own. Pure, tested.
pub fn reaction_news(changed: &[(Value, Value)], mine: &[u64]) -> Option<(String, String)> {
    let mut told = Vec::new();
    for (before, after) in changed {
        if !after.get("id").and_then(Value::as_u64).is_some_and(|id| mine.contains(&id)) {
            continue;
        }
        for r in after.get("reactions").and_then(Value::as_array).cloned().unwrap_or_default() {
            let emoji = r.get("emoji").and_then(Value::as_str).unwrap_or("").to_string();
            let (now, mine_now) = reaction(after, &emoji);
            let (was, mine_before) = reaction(before, &emoji);
            let by_me = u64::from(mine_now && !mine_before);
            if now > was + by_me {
                let said = after.get("text").and_then(Value::as_str).unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
                let what = if said.is_empty() { "your picture or voice line".to_string() } else { preview(&said) };
                told.push(format!("{emoji} on \"{what}\""));
            }
        }
    }
    let first = told.first()?.clone();
    let body = if told.len() > 1 { format!("{} new reactions. Latest: {}", told.len(), told.last().unwrap()) } else { format!("Someone reacted {first}") };
    Some(("A reaction in the room".to_string(), body))
}

/// Reactions to the member's own lines, told only when Settings > Chat > Reaction alerts is on.
pub fn reactions(app: &AppHandle, changed: &[(Value, Value)], chat_shown: bool) {
    let store = app.state::<SettingsStore>();
    let settings = store.get();
    if !settings.alert_reactions || !(settings.alert_sound || settings.alert_notify) || looking(app, chat_shown) {
        return;
    }
    let Some((title, body)) = reaction_news(changed, &crate::chatroom::my_line_ids(&store)) else { return };
    if settings.alert_sound {
        beep();
    }
    if settings.alert_notify {
        toast(app, &title, &body);
    }
}

pub fn summary(fresh: &[Value], mine: &[u64]) -> Option<(String, String)> {
    let others: Vec<&Value> = fresh.iter().filter(|l| l.get("id").and_then(Value::as_u64).is_none_or(|id| !mine.contains(&id))).collect();
    let last = others.last()?;
    let who = last.pointer("/author/name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Someone");
    let said = last.get("text").and_then(Value::as_str).unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
    let said = if said.is_empty() { "sent a picture or a voice line".to_string() } else { preview(&said) };
    let body = if others.len() > 1 { format!("{} new messages. Latest, {who}: {said}", others.len()) } else { format!("{who}: {said}") };
    Some(("New in the room".to_string(), body))
}

fn preview(text: &str) -> String {
    let mut short: String = text.chars().take(PREVIEW).collect();
    if text.chars().count() > PREVIEW {
        short.push('…');
    }
    short
}

/// Windows' own notification sound, so it follows the member's sound scheme and volume.
#[cfg(windows)]
fn beep() -> bool {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
    let name: Vec<u16> = "SystemNotification\0".encode_utf16().collect();
    unsafe {
        PlaySoundW(name.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC | SND_NODEFAULT) != 0
    }
}

#[cfg(not(windows))]
fn beep() -> bool {
    false
}

/// The member is looking at the room when CHAT is the tab on screen in a visible, focused Castle.
fn looking(app: &AppHandle, chat_shown: bool) -> bool {
    chat_shown && app.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
}

/// Does the text name one of `names` as a whole word (or phrase), case and spacing aside?
/// "Ali" never matches inside "Alice", and "Kiyo," or "@Kiyo" still name Kiyo. Pure, tested.
pub fn names_someone(text: &str, names: &[String]) -> bool {
    let squeeze = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let text = squeeze(text);
    names.iter().map(|n| squeeze(n)).filter(|n| !n.is_empty()).any(|name| {
        let mut from = 0;
        while let Some(at) = text[from..].find(&name) {
            let start = from + at;
            let end = start + name.len();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            if !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric) {
                return true;
            }
            from = start + text[start..].chars().next().map_or(1, char::len_utf8);
        }
        false
    })
}

/// The member's display name in the room (their Foundation name), asked once and kept until
/// sign-in, sign-out or a display-name change forgets it.
static ROOM_NAME: Mutex<Option<String>> = Mutex::new(None);

fn room_name() -> Option<String> {
    if let Some(name) = ROOM_NAME.lock().unwrap().clone() {
        return Some(name);
    }
    let token = crate::key::load("member")?;
    let me = crate::chatroom::Room::new(&crate::signin::portfolio(), &token).me().ok()?;
    let name = me.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty())?.to_string();
    *ROOM_NAME.lock().unwrap() = Some(name.clone());
    Some(name)
}

/// The kept room name is out of date (signed in or out, or the display name changed).
pub fn forget_name() {
    *ROOM_NAME.lock().unwrap() = None;
}

/// The names that make a line worth an alert in "named" mode: the member's name in Settings,
/// their name in the room, and their companion's name.
fn names_to_hear(settings: &Settings) -> Vec<String> {
    let mut names: Vec<String> = [settings.member_name.clone(), room_name(), settings.companion_name.clone()].into_iter().flatten().collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by_key(|n| n.to_lowercase());
    names
}

/// Called with the lines a read just found (never the first read after start or sign-in).
pub fn new_lines(app: &AppHandle, fresh: &[Value], chat_shown: bool) {
    let store = app.state::<SettingsStore>();
    let settings = store.get();
    if !(settings.alert_sound || settings.alert_notify) || looking(app, chat_shown) {
        return;
    }
    // the member's own companion speaking is not news to tell them about
    let mut fresh: Vec<Value> = fresh.iter().filter(|l| !crate::roomturn::companion_said(&settings, l)).cloned().collect();
    if settings.alert_when == "named" {
        let names = names_to_hear(&settings);
        let mine = crate::chatroom::my_line_ids(&store);
        // a reply to one of the member's own lines counts as naming them
        fresh.retain(|l| l.get("text").and_then(Value::as_str).is_some_and(|t| names_someone(t, &names)) || replies_to(l, &mine));
    }
    let Some((title, body)) = summary(&fresh, &crate::chatroom::my_line_ids(&store)) else { return };
    if settings.alert_sound {
        beep();
    }
    if settings.alert_notify {
        // No `.sound(..)`: with tauri-plugin-notification 2.8 / notify-rust 4.19 a Windows toast built
        // without one is `<audio silent="true"/>`, so the Sound switch is the only sound (checked in
        // the dependency source, not by ear).
        toast(app, &title, &body);
    }
}

/// The desktop notification. On Windows a click on it brings Castle forward (out of the tray, or
/// minimised) and opens the CHAT tab, which the plugin's notification cannot do. Elsewhere it is the
/// plugin's plain notification.
#[cfg(windows)]
fn toast(app: &AppHandle, title: &str, body: &str) {
    use tauri::Emitter;
    use tauri_winrt_notification::{Sound, Toast};
    // the id the plugin uses: Castle's own when installed, PowerShell's while running from a build folder
    let app_id = match std::env::current_exe() {
        Ok(exe) if exe.parent().is_some_and(|d| d.ends_with("target/release") || d.ends_with("target/debug")) => Toast::POWERSHELL_APP_ID.to_string(),
        _ => app.config().identifier.clone(),
    };
    let clicked = app.clone();
    let shown = Toast::new(&app_id)
        .title(title)
        .text1(body)
        // silent: the Sound switch is the only sound (a toast without this plays Windows' own chime)
        .sound(None::<Sound>)
        .on_activated(move |_| {
            crate::tray::show_main(&clicked);
            let _ = clicked.emit(OPEN_CHAT_EVENT, ());
            Ok(())
        })
        .show();
    if shown.is_err() {
        // the toast could not be made: the plugin's plain one still tells the member
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

#[cfg(not(windows))]
fn toast(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line(id: u64, who: &str, text: &str) -> Value {
        json!({ "id": id, "text": text, "author": { "name": who, "kind": "member" } })
    }

    /// Plays the sound once, out loud: `cargo test plays_the_windows -- --ignored`.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn plays_the_windows_notification_sound() {
        assert!(beep(), "Windows refused to play its notification sound");
    }

    #[test]
    fn one_line_names_who_said_it() {
        let (title, body) = summary(&[line(5, "Kiyo", "Boleh   tengok\nPR tu")], &[]).unwrap();
        assert_eq!(title, "New in the room");
        assert_eq!(body, "Kiyo: Boleh tengok PR tu");
    }

    #[test]
    fn several_lines_are_counted_and_the_latest_is_shown() {
        let (_, body) = summary(&[line(1, "Kiyo", "a"), line(2, "Alice", "b")], &[]).unwrap();
        assert_eq!(body, "2 new messages. Latest, Alice: b");
    }

    #[test]
    fn the_members_own_lines_never_alert() {
        assert!(summary(&[line(7, "Bernard", "hi")], &[7]).is_none());
        let (_, body) = summary(&[line(7, "Bernard", "hi"), line(8, "Kiyo", "hey")], &[7]).unwrap();
        assert_eq!(body, "Kiyo: hey");
        assert!(summary(&[], &[]).is_none());
    }

    #[test]
    fn long_and_empty_lines_are_handled() {
        let long = "x".repeat(200);
        let (_, body) = summary(&[line(1, "Kiyo", &long)], &[]).unwrap();
        assert_eq!(body.chars().count(), "Kiyo: ".len() + PREVIEW + 1);
        assert!(body.ends_with('…'));
        let (_, body) = summary(&[json!({ "id": 2, "text": "", "author": { "name": "  " } })], &[]).unwrap();
        assert_eq!(body, "Someone: sent a picture or a voice line");
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn a_name_counts_only_as_a_whole_word() {
        let heard = names(&["Kiyo", "Ali"]);
        assert!(names_someone("hi Kiyo, how are you?", &heard));
        assert!(names_someone("@kiyo look at this", &heard), "case aside, an @ in front is fine");
        assert!(names_someone("thanks KIYO!", &heard));
        assert!(!names_someone("Kiyoraka pushed the PR", &heard), "not inside a longer word");
        assert!(!names_someone("Alice said hello", &heard), "Ali is not inside Alice");
        assert!(names_someone("Ali: are you there", &heard));
    }

    #[test]
    fn a_name_of_two_words_is_heard_as_a_phrase() {
        let heard = names(&["Alice Test"]);
        assert!(names_someone("what do you think, alice   test?", &heard), "spacing aside");
        assert!(!names_someone("Alice is a test", &heard));
        assert!(!names_someone("Alice Tester", &heard));
    }

    #[test]
    fn no_names_hear_nothing() {
        assert!(!names_someone("hello everyone", &[]));
        assert!(!names_someone("hello everyone", &names(&["", "   "])), "empty names never match everything");
        assert!(!names_someone("", &names(&["Kiyo"])));
    }

    #[test]
    fn a_reply_to_my_line_counts_and_only_others_reactions_are_news() {
        use serde_json::json;
        assert!(replies_to(&json!({"id": 9, "replyTo": {"id": 4, "name": "Me", "text": "hi"}}), &[4]));
        assert!(!replies_to(&json!({"id": 9, "replyTo": {"id": 5}}), &[4]));
        assert!(!replies_to(&json!({"id": 9}), &[4]));
        let before = json!({"id": 4, "text": "my line", "reactions": [{"emoji": "👍", "count": 1, "mine": false}]});
        let after = json!({"id": 4, "text": "my line", "reactions": [{"emoji": "👍", "count": 2, "mine": false}]});
        let (title, body) = reaction_news(&[(before.clone(), after)], &[4]).unwrap();
        assert_eq!(title, "A reaction in the room");
        assert!(body.contains("👍") && body.contains("my line"));
        // the member's own tap is not news
        let mine = json!({"id": 4, "text": "my line", "reactions": [{"emoji": "👍", "count": 2, "mine": true}]});
        assert!(reaction_news(&[(before.clone(), mine)], &[4]).is_none());
        // someone else's line is not the member's news
        let other = json!({"id": 7, "text": "x", "reactions": [{"emoji": "👍", "count": 5, "mine": false}]});
        assert!(reaction_news(&[(json!({"id": 7}), other)], &[4]).is_none());
    }
}
