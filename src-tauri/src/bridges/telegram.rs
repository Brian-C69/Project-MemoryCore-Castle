//! The Telegram adapter for MemoryCore in your pocket: a bot the member messages from their
//! phone, answered by their own companion's real session (`bridges::send_and_await`). Off until
//! switched on in Settings > Messaging; the bot token lives in Windows Credential Manager like
//! every other key (Settings > Keys > Telegram bot token).
//! The first chat to message the bot is held as "pending" until the member approves it from
//! Settings - after that, only the approved chat gets replies. A leaked token or a found bot
//! would otherwise mean a stranger reaching the member's own machine, not just spending tokens.

use crate::settings::SettingsStore;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

const ACCOUNT: &str = "telegram-bridge";
/// Telegram holds `getUpdates` open this long when nothing is waiting; the request's own timeout
/// must be longer still, or Castle would cut the long poll off itself.
const LONG_POLL_SECS: u64 = 25;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
/// How long a reply is waited for before Telegram is told it took too long.
const REPLY_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Default)]
pub struct TelegramState {
    connected: Mutex<bool>,
    problem: Mutex<Option<String>>,
    /// A chat id that messaged the bot and is not yet approved, with the name Telegram gave for it.
    pending: Mutex<Option<(i64, String)>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TelegramPending {
    chat_id: i64,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramStatus {
    on: bool,
    connected: bool,
    has_token: bool,
    paired: bool,
    pending: Option<TelegramPending>,
    problem: Option<String>,
}

fn api(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}

/// The chat id, its sender's text and the name Telegram gave, from one `getUpdates` item - or
/// None for anything that is not a plain text message (a photo, an edit, a channel post, ...).
fn incoming(update: &Value) -> Option<(i64, String, String)> {
    let message = update.get("message")?;
    let chat_id = message.pointer("/chat/id")?.as_i64()?;
    let text = message.get("text")?.as_str()?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let name = message.pointer("/from/first_name").and_then(Value::as_str).unwrap_or("Someone").to_string();
    Some((chat_id, text, name))
}

fn send_message(client: &reqwest::blocking::Client, token: &str, chat_id: i64, text: &str) -> Result<(), String> {
    let response = client
        .post(api(token, "sendMessage"))
        .json(&json!({ "chat_id": chat_id, "text": text }))
        .send()
        .map_err(|_| "Telegram did not take the reply.".to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err("Telegram refused the reply.".to_string())
    }
}

/// One incoming message: approved, it goes to the companion's real session and the answer (or
/// the trouble reaching it) is sent back; not yet approved, the sender is told so and held
/// pending for Settings to approve; a different, already-rejected-by-silence chat is ignored.
fn handle(app: &AppHandle, client: &reqwest::blocking::Client, token: &str, chat_id: i64, text: String, name: String) -> Result<(), String> {
    let store = app.state::<SettingsStore>();
    let settings = store.get();
    let approved = settings.telegram_bridge_chat_id.as_deref().and_then(|id| id.parse::<i64>().ok());
    match approved {
        Some(id) if id == chat_id => {
            let reply = crate::bridges::send_and_await(app, &store, &text, REPLY_TIMEOUT).unwrap_or_else(|problem| problem);
            send_message(client, token, chat_id, &reply)
        }
        Some(_) => Ok(()),
        None => {
            *app.state::<TelegramState>().pending.lock().unwrap() = Some((chat_id, name));
            send_message(client, token, chat_id, "Hi! Ask in Castle's Settings > Messaging to approve this chat before I can answer.")
        }
    }
}

/// The first look after the bridge starts: asks for no waiting, takes note of the newest update
/// Telegram holds and answers nothing before it. Quick, so the card can say "Connected" at once,
/// and a message that waited while the bridge was off is not answered late.
fn catch_up(client: &reqwest::blocking::Client, token: &str) -> Result<i64, String> {
    let url = format!("{}?timeout=0&offset=-1", api(token, "getUpdates"));
    let body: Value = client.get(url).send().map_err(|_| "Telegram did not answer.".to_string())?.json().map_err(|_| "Telegram sent something unreadable.".to_string())?;
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        let message = body.get("description").and_then(Value::as_str).unwrap_or("Telegram refused the request.");
        return Err(message.to_string());
    }
    let newest = body["result"].as_array().and_then(|u| u.iter().filter_map(|x| x.get("update_id").and_then(Value::as_i64)).max());
    Ok(newest.map_or(0, |id| id + 1))
}

fn poll(app: &AppHandle, client: &reqwest::blocking::Client, token: &str, offset: &mut i64) -> Result<(), String> {
    let url = format!("{}?timeout={LONG_POLL_SECS}&offset={offset}", api(token, "getUpdates"));
    let body: Value = client.get(url).send().map_err(|_| "Telegram did not answer.".to_string())?.json().map_err(|_| "Telegram sent something unreadable.".to_string())?;
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        let message = body.get("description").and_then(Value::as_str).unwrap_or("Telegram refused the request.");
        return Err(message.to_string());
    }
    for update in body["result"].as_array().cloned().unwrap_or_default() {
        let Some(update_id) = update.get("update_id").and_then(Value::as_i64) else { continue };
        *offset = update_id + 1;
        if let Some((chat_id, text, name)) = incoming(&update) {
            let _ = handle(app, client, token, chat_id, text, name);
        }
    }
    Ok(())
}

/// Starts the bridge with Castle; it only calls out while the switch in Settings is on.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut offset: Option<i64> = None;
        loop {
            let state = app.state::<TelegramState>();
            let settings = app.state::<SettingsStore>().get();
            let token = crate::key::load(ACCOUNT);
            let (true, Some(token)) = (settings.telegram_bridge_on, token) else {
                offset = None; // switched on again: do not answer messages sent while it was off
                *state.connected.lock().unwrap() = false;
                *state.problem.lock().unwrap() = None;
                std::thread::sleep(Duration::from_secs(3));
                continue;
            };
            let Ok(client) = reqwest::blocking::Client::builder().timeout(REQUEST_TIMEOUT).build() else {
                *state.problem.lock().unwrap() = Some("The Telegram call could not start.".into());
                std::thread::sleep(Duration::from_secs(5));
                continue;
            };
            let Some(next) = offset else {
                let outcome = catch_up(&client, &token);
                *state.connected.lock().unwrap() = outcome.is_ok();
                match outcome {
                    Ok(next) => {
                        offset = Some(next);
                        *state.problem.lock().unwrap() = None;
                    }
                    Err(problem) => {
                        *state.problem.lock().unwrap() = Some(problem);
                        std::thread::sleep(Duration::from_secs(3));
                    }
                }
                continue;
            };
            let mut next = next;
            let outcome = poll(&app, &client, &token, &mut next);
            offset = Some(next);
            *state.connected.lock().unwrap() = outcome.is_ok();
            *state.problem.lock().unwrap() = outcome.err();
        }
    });
}

#[tauri::command]
pub fn telegram_bridge_status(store: State<SettingsStore>, state: State<TelegramState>) -> TelegramStatus {
    let settings = store.get();
    TelegramStatus {
        on: settings.telegram_bridge_on,
        connected: *state.connected.lock().unwrap(),
        has_token: crate::key::load(ACCOUNT).is_some(),
        paired: settings.telegram_bridge_chat_id.is_some(),
        pending: state.pending.lock().unwrap().clone().map(|(chat_id, name)| TelegramPending { chat_id, name }),
        problem: state.problem.lock().unwrap().clone(),
    }
}

/// Approves the chat currently held pending (if any) as the only one the bridge answers.
#[tauri::command]
pub fn telegram_bridge_approve(store: State<SettingsStore>, state: State<TelegramState>) -> Result<(), String> {
    let Some((chat_id, _)) = state.pending.lock().unwrap().take() else {
        return Err("There is no chat waiting to be approved.".into());
    };
    let mut next = store.get();
    next.telegram_bridge_chat_id = Some(chat_id.to_string());
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

/// Forgets the approved chat, so a new one can pair (e.g. the member switched phones).
#[tauri::command]
pub fn telegram_bridge_forget(store: State<SettingsStore>) -> Result<(), String> {
    let mut next = store.get();
    next.telegram_bridge_chat_id = None;
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(id: i64, chat_id: i64, name: &str, text: &str) -> Value {
        json!({ "update_id": id, "message": { "chat": { "id": chat_id }, "from": { "first_name": name }, "text": text } })
    }

    #[test]
    fn the_api_address_carries_the_token_and_the_method() {
        assert_eq!(api("abc:123", "getUpdates"), "https://api.telegram.org/botabc:123/getUpdates");
        assert_eq!(api("abc:123", "sendMessage"), "https://api.telegram.org/botabc:123/sendMessage");
    }

    #[test]
    fn a_text_message_is_read_and_anything_else_is_left_out() {
        let got = incoming(&update(1, 42, "Rin", " hello ")).unwrap();
        assert_eq!(got, (42, "hello".to_string(), "Rin".to_string()));
        assert!(incoming(&update(2, 42, "Rin", "  ")).is_none());
        assert!(incoming(&json!({ "update_id": 3, "message": { "chat": { "id": 1 } } })).is_none());
        assert!(incoming(&json!({ "update_id": 4, "edited_message": { "chat": { "id": 1 }, "text": "edit" } })).is_none());
        // no name given: a plain fallback, never a panic
        let anon = incoming(&json!({ "update_id": 5, "message": { "chat": { "id": 7 }, "text": "hi" } })).unwrap();
        assert_eq!(anon.2, "Someone");
    }
}
