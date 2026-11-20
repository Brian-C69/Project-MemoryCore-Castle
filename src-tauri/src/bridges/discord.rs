//! The Discord adapter for MemoryCore in your pocket: a bot the member DMs from their phone,
//! answered by their own companion's real session (`bridges::send_and_await`). Off until switched
//! on in the MESSAGING room; the bot token lives in Windows Credential Manager like every other
//! key. Discord has no polling equivalent for DMs - this keeps one Gateway WebSocket open and
//! heartbeats it itself, reconnecting from scratch on any trouble (no Resume in v1: a reconnect
//! means a short gap, never lost state, which is fine for a personal bridge).
//! The first DM channel to message the bot is held "pending" until the member approves it on the
//! MESSAGING card - same reasoning as Telegram: a leaked token would otherwise mean a stranger
//! reaching the member's own machine, not just spending tokens.

use crate::settings::SettingsStore;
use serde::Serialize;
use serde_json::{json, Value};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use tungstenite::{connect, stream::MaybeTlsStream, Message};

const ACCOUNT: &str = "discord-bridge";
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const API: &str = "https://discord.com/api/v10";
/// DIRECT_MESSAGES (1<<12) | MESSAGE_CONTENT (1<<15, privileged - the member must also switch
/// this on for their bot in Discord's own Developer Portal).
const INTENTS: u32 = (1 << 12) | (1 << 15);
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const REPLY_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Default)]
pub struct DiscordState {
    connected: Mutex<bool>,
    problem: Mutex<Option<String>>,
    /// A DM channel that messaged the bot and is not yet approved: (channel_id, sender's name).
    pending: Mutex<Option<(String, String)>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscordPending {
    channel_id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordStatus {
    on: bool,
    connected: bool,
    has_token: bool,
    paired: bool,
    pending: Option<DiscordPending>,
    problem: Option<String>,
}

fn set_read_timeout(stream: &MaybeTlsStream<TcpStream>, timeout: Option<Duration>) {
    let plain = match stream {
        MaybeTlsStream::Plain(s) => Some(s),
        MaybeTlsStream::Rustls(s) => Some(s.get_ref()),
        _ => None,
    };
    if let Some(s) = plain {
        let _ = s.set_read_timeout(timeout);
    }
}

/// The channel id, the sender's text and the name Discord gave, from one MESSAGE_CREATE's data -
/// or None for anything that is not a plain-text DM from a person (a bot, a guild message, an
/// empty edit, ...).
fn incoming(data: &Value) -> Option<(String, String, String)> {
    if data.pointer("/channel_type").and_then(Value::as_i64) != Some(1) {
        return None;
    }
    if data.pointer("/author/bot").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let channel_id = data.get("channel_id").and_then(Value::as_str)?.to_string();
    let text = data.get("content").and_then(Value::as_str)?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let name = data.pointer("/author/username").and_then(Value::as_str).unwrap_or("Someone").to_string();
    Some((channel_id, text, name))
}

fn send_message(client: &reqwest::blocking::Client, token: &str, channel_id: &str, text: &str) -> Result<(), String> {
    let response = client
        .post(format!("{API}/channels/{channel_id}/messages"))
        .header("Authorization", format!("Bot {token}"))
        .json(&json!({ "content": text }))
        .send()
        .map_err(|_| "Discord did not take the reply.".to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err("Discord refused the reply.".to_string())
    }
}

/// One incoming DM: approved, it goes to the companion's real session (in its own thread, so a
/// slow reply never stalls the Gateway's heartbeat); not yet approved, the sender is told so and
/// held pending; a different, already-ignored channel is left alone.
fn handle(app: &AppHandle, token: &str, channel_id: String, text: String, name: String) {
    let app = app.clone();
    let token = token.to_string();
    std::thread::spawn(move || {
        let Ok(client) = reqwest::blocking::Client::builder().timeout(Duration::from_secs(15)).build() else { return };
        let store = app.state::<SettingsStore>();
        let settings = store.get();
        let approved = settings.discord_bridge_channel_id.as_deref();
        match approved {
            Some(id) if id == channel_id => {
                let reply = crate::bridges::send_and_await(&app, &store, &text, REPLY_TIMEOUT).unwrap_or_else(|problem| problem);
                let _ = send_message(&client, &token, &channel_id, &reply);
            }
            Some(_) => {}
            None => {
                *app.state::<DiscordState>().pending.lock().unwrap() = Some((channel_id.clone(), name));
                let _ = send_message(&client, &token, &channel_id, "Hi! Approve this chat on Castle's MESSAGING room before I can answer.");
            }
        }
    });
}

/// One run of the Gateway connection: connects, identifies, then reads until it is told to stop
/// or something goes wrong - either end returns to `start`'s loop to reconnect from scratch.
fn run(app: &AppHandle, token: &str) -> Result<(), String> {
    let (mut socket, _) = connect(GATEWAY_URL).map_err(|_| "Discord's Gateway did not answer.".to_string())?;
    set_read_timeout(socket.get_ref(), Some(READ_TIMEOUT));

    let mut heartbeat_interval: Option<Duration> = None;
    let mut last_heartbeat = Instant::now();
    let mut identified = false;

    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                let payload: Value = serde_json::from_str(&text).map_err(|_| "Discord sent something unreadable.".to_string())?;
                let op = payload.get("op").and_then(Value::as_i64);
                match op {
                    // Hello: learn the heartbeat interval, then Identify
                    Some(10) => {
                        let millis = payload.pointer("/d/heartbeat_interval").and_then(Value::as_u64).unwrap_or(41250);
                        heartbeat_interval = Some(Duration::from_millis(millis));
                        let identify = json!({ "op": 2, "d": { "token": token, "intents": INTENTS, "properties": { "os": "windows", "browser": "castle", "device": "castle" } } });
                        socket.send(Message::Text(identify.to_string().into())).map_err(|_| "Discord did not accept the sign-in.".to_string())?;
                        identified = true;
                    }
                    // Heartbeat ACK: nothing to do, the loop below resends on its own schedule
                    Some(11) => {}
                    // Reconnect / Invalid session: start over
                    Some(7) | Some(9) => return Err("Discord asked for a fresh connection.".into()),
                    // Dispatch
                    Some(0) => {
                        if payload.get("t").and_then(Value::as_str) == Some("MESSAGE_CREATE") {
                            if let Some(data) = payload.get("d") {
                                if let Some((channel_id, text, name)) = incoming(data) {
                                    handle(app, token, channel_id, text, name);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Message::Close(_)) => return Err("Discord closed the connection.".into()),
            Ok(_) => {}
            // A read timeout is expected - it is what lets this loop reach the heartbeat check below
            Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return Err("The connection to Discord was lost.".into()),
        }

        if !identified {
            continue;
        }
        if let Some(interval) = heartbeat_interval {
            if last_heartbeat.elapsed() >= interval {
                let beat = json!({ "op": 1, "d": Value::Null });
                socket.send(Message::Text(beat.to_string().into())).map_err(|_| "The heartbeat could not be sent.".to_string())?;
                last_heartbeat = Instant::now();
            }
        }
    }
}

/// Starts the bridge with Castle; it only connects while the switch in the MESSAGING room is on.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            let state = app.state::<DiscordState>();
            let settings = app.state::<SettingsStore>().get();
            let token = crate::key::load(ACCOUNT);
            let (true, Some(token)) = (settings.discord_bridge_on, token) else {
                *state.connected.lock().unwrap() = false;
                *state.problem.lock().unwrap() = None;
                std::thread::sleep(Duration::from_secs(3));
                continue;
            };
            *state.connected.lock().unwrap() = true;
            *state.problem.lock().unwrap() = None;
            let outcome = run(&app, &token);
            *state.connected.lock().unwrap() = false;
            *state.problem.lock().unwrap() = outcome.err();
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

#[tauri::command]
pub fn discord_bridge_status(store: State<SettingsStore>, state: State<DiscordState>) -> DiscordStatus {
    let settings = store.get();
    DiscordStatus {
        on: settings.discord_bridge_on,
        connected: *state.connected.lock().unwrap(),
        has_token: crate::key::load(ACCOUNT).is_some(),
        paired: settings.discord_bridge_channel_id.is_some(),
        pending: state.pending.lock().unwrap().clone().map(|(channel_id, name)| DiscordPending { channel_id, name }),
        problem: state.problem.lock().unwrap().clone(),
    }
}

/// Approves the DM channel currently held pending (if any) as the only one the bridge answers.
#[tauri::command]
pub fn discord_bridge_approve(store: State<SettingsStore>, state: State<DiscordState>) -> Result<(), String> {
    let Some((channel_id, _)) = state.pending.lock().unwrap().take() else {
        return Err("There is no chat waiting to be approved.".into());
    };
    let mut next = store.get();
    next.discord_bridge_channel_id = Some(channel_id);
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

/// Forgets the approved channel, so a new one can pair.
#[tauri::command]
pub fn discord_bridge_forget(store: State<SettingsStore>) -> Result<(), String> {
    let mut next = store.get();
    next.discord_bridge_channel_id = None;
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(channel_type: i64, bot: bool, name: &str, text: &str) -> Value {
        json!({ "channel_type": channel_type, "channel_id": "99", "content": text, "author": { "username": name, "bot": bot } })
    }

    #[test]
    fn a_persons_dm_is_read_and_everything_else_is_left_out() {
        let got = incoming(&message(1, false, "Rin", " hello ")).unwrap();
        assert_eq!(got, ("99".to_string(), "hello".to_string(), "Rin".to_string()));
        assert!(incoming(&message(0, false, "Rin", "in a server")).is_none(), "not a DM");
        assert!(incoming(&message(1, true, "Bot", "hi")).is_none(), "a bot's own message");
        assert!(incoming(&message(1, false, "Rin", "  ")).is_none(), "empty");
        let anon = incoming(&json!({ "channel_type": 1, "channel_id": "1", "content": "hi", "author": {} })).unwrap();
        assert_eq!(anon.2, "Someone");
    }
}
