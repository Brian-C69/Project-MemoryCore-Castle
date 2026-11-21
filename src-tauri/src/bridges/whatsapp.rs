//! The WhatsApp adapter for MemoryCore in your pocket: a small local sidecar
//! (`resources/tools/castle-whatsapp.cjs`, Castle's bundled Node running Baileys - a WhatsApp Web
//! client, the same library OpenClaw and Hermes both use) that Castle spawns and talks to over a
//! localhost-only HTTP door. Off until switched on in MESSAGING; no token - the "credential" is
//! the QR-paired session itself, kept in Castle's settings folder so a restart does not ask for
//! the QR again.
//!
//! **Unofficial, ban risk**: WhatsApp has no public API for this. A secondary number is the
//! right one to pair, never a primary personal one, same as Hermes' own docs disclose.
//! v1 answers one-to-one chats only - the sidecar script itself drops groups and broadcasts.

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

const PORT: u16 = 51847;
const REPLY_TIMEOUT: Duration = Duration::from_secs(180);
const TICK: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct WhatsAppState {
    child: Mutex<Option<Child>>,
    connected: Mutex<bool>,
    paired: Mutex<bool>,
    qr: Mutex<Option<String>>,
    problem: Mutex<Option<String>>,
    /// A WhatsApp chat id (JID) that messaged in and is not yet approved: (jid, sender's name).
    pending: Mutex<Option<(String, String)>>,
}

impl Drop for WhatsAppState {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppPending {
    chat_id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppStatus {
    on: bool,
    connected: bool,
    paired: bool,
    qr: Option<String>,
    pending: Option<WhatsAppPending>,
    problem: Option<String>,
}

#[derive(Deserialize)]
struct SidecarStatus {
    paired: bool,
    connected: bool,
    qr: Option<String>,
    problem: Option<String>,
}

#[derive(Deserialize)]
struct SidecarIncoming {
    messages: Vec<IncomingMessage>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct IncomingMessage {
    chat_id: String,
    name: String,
    #[serde(default)]
    text: String,
    /// "text", "media" (a picture, voice note or video, as a line) or "reaction"
    #[serde(default = "text_kind")]
    kind: String,
    /// a reaction's emoji; empty when it was taken back
    #[serde(default)]
    emoji: String,
    /// the id of the message a reaction is on
    #[serde(default)]
    on_id: Option<String>,
    /// the phone number behind the chat, when WhatsApp gives it
    #[serde(default)]
    pn: Option<String>,
}

fn text_kind() -> String {
    "text".into()
}

#[derive(Deserialize)]
struct SentReply {
    id: Option<String>,
}

/// Sends one message through the helper; the id WhatsApp gave it, so a reaction can be placed.
fn send(client: &reqwest::blocking::Client, to: &str, text: &str) -> Option<String> {
    let response = client.post(door("/send")).json(&json!({ "to": to, "text": text })).send().ok()?;
    response.json::<SentReply>().ok()?.id
}

fn door(path: &str) -> String {
    format!("http://127.0.0.1:{PORT}{path}")
}

fn auth_dir(store: &SettingsStore) -> PathBuf {
    store.folder().join("whatsapp-auth")
}

fn script_path(app: &AppHandle) -> Option<PathBuf> {
    let path = app.path().resource_dir().ok()?.join("tools").join("castle-whatsapp.cjs");
    // Windows hands back a path that starts with `\\?\`; Node cannot resolve that form ("EISDIR ... lstat 'C:'"), so the plain path is used.
    let text = path.to_string_lossy().into_owned();
    let plain = text.strip_prefix("\\\\?\\").map(PathBuf::from).unwrap_or(path);
    plain.is_file().then_some(plain)
}

/// Whether the helper is alive. One that has exited (its port was taken, it crashed) is forgotten,
/// so the loop starts a new one instead of waiting for ever on a helper that is not there.
fn running(state: &WhatsAppState) -> bool {
    let mut child = state.child.lock().unwrap();
    let Some(c) = child.as_mut() else { return false };
    match c.try_wait() {
        Ok(None) => true,
        other => {
            *child = None;
            let how = match other {
                Ok(Some(status)) => format!(" (exit {})", status.code().map_or("?".to_string(), |c| c.to_string())),
                _ => String::new(),
            };
            *state.problem.lock().unwrap() = Some(format!("The WhatsApp helper stopped{how}; starting it again."));
            false
        }
    }
}

fn ensure_started(app: &AppHandle, store: &SettingsStore, state: &WhatsAppState) -> Result<(), String> {
    if running(state) {
        return Ok(());
    }
    let script = script_path(app).ok_or("The WhatsApp helper is missing from this install.")?;
    // What the helper says goes to a log beside its pairing, so a helper that stops can be explained.
    let log = std::fs::File::create(store.folder().join("whatsapp.log")).map_err(|_| "The WhatsApp helper's log could not be opened.".to_string())?;
    let mut command = Command::new(crate::env::node());
    command
        .arg(&script)
        // Node resolves the script's real path from where it starts; Castle's own folder can make that fail ("EISDIR ... lstat 'C:'").
        .current_dir(store.folder())
        .env("WA_AUTH_DIR", auth_dir(store))
        .env("WA_PORT", PORT.to_string())
        .stdout(log.try_clone().map(Stdio::from).unwrap_or_else(|_| Stdio::null()))
        .stderr(Stdio::from(log))
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window: the helper is invisible, its words go to the log
    }
    let child = command.spawn().map_err(|_| "The WhatsApp helper could not be started.".to_string())?;
    *state.child.lock().unwrap() = Some(child);
    Ok(())
}

fn stop(state: &WhatsAppState) {
    if let Some(mut child) = state.child.lock().unwrap().take() {
        let _ = child.kill();
    }
    *state.connected.lock().unwrap() = false;
    *state.paired.lock().unwrap() = false;
    *state.qr.lock().unwrap() = None;
}

/// One incoming chat: approved, it goes to the companion's real session (in its own thread, so a
/// slow reply never stalls the poll loop); not yet approved, the sender is told so and held
/// pending; a different, already-ignored chat is left alone.
fn handle(app: &AppHandle, client: &reqwest::blocking::Client, chat_id: String, text: String, name: String) {
    let app = app.clone();
    let client = client.clone();
    std::thread::spawn(move || {
        let store = app.state::<SettingsStore>();
        let settings = store.get();
        let approved = settings.whatsapp_bridge_chat_id.as_deref();
        match approved {
            Some(id) if id == chat_id => {
                let reply = crate::bridges::send_and_await(&app, &store, &text, REPLY_TIMEOUT).unwrap_or_else(|problem| problem);
                if let Some(id) = send(&client, &chat_id, &reply) {
                    crate::bridges::reactions::remember(&app, "whatsapp", &id, &reply);
                }
            }
            Some(_) => {}
            None => {
                *app.state::<WhatsAppState>().pending.lock().unwrap() = Some((chat_id.clone(), name));
                let _ = send(&client, &chat_id, "Hi! Approve this chat on Castle's MESSAGING room before I can answer.");
            }
        }
    });
}

fn poll(app: &AppHandle, client: &reqwest::blocking::Client, state: &WhatsAppState) {
    let Ok(response) = client.get(door("/status")).send() else { return }; // the sidecar is still starting up - not an error worth surfacing
    let Ok(status) = response.json::<SidecarStatus>() else { return };
    *state.connected.lock().unwrap() = status.connected;
    *state.paired.lock().unwrap() = status.paired;
    *state.qr.lock().unwrap() = status.qr;
    *state.problem.lock().unwrap() = status.problem;

    let Ok(response) = client.get(door("/incoming")).send() else { return };
    let Ok(incoming) = response.json::<SidecarIncoming>() else { return };
    for m in incoming.messages {
        if m.kind == "reaction" {
            // only the approved chat's hearts are shown
            let approved = app.state::<SettingsStore>().get().whatsapp_bridge_chat_id;
            if let (Some(on), true) = (m.on_id.as_deref(), approved.as_deref() == Some(m.chat_id.as_str())) {
                crate::bridges::reactions::record(app, "whatsapp", on, &m.emoji);
            }
            continue;
        }
        if m.kind == "media" {
            continue; // the companion's own session takes words only
        }
        handle(app, client, m.chat_id, m.text, m.name);
    }
}

/// Starts the bridge with Castle; the sidecar itself only runs while the switch in the MESSAGING
/// room is on, and is killed the moment it is switched off.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let Ok(client) = reqwest::blocking::Client::builder().timeout(Duration::from_secs(10)).build() else { return };
        loop {
            std::thread::sleep(TICK);
            let state = app.state::<WhatsAppState>();
            let store = app.state::<SettingsStore>();
            let settings = store.get();
            if !settings.whatsapp_bridge_on {
                if running(&state) {
                    stop(&state);
                }
                continue;
            }
            if let Err(problem) = ensure_started(&app, &store, &state) {
                *state.problem.lock().unwrap() = Some(problem);
                continue;
            }
            poll(&app, &client, &state);
        }
    });
}

#[tauri::command]
pub fn whatsapp_bridge_status(store: State<SettingsStore>, state: State<WhatsAppState>) -> WhatsAppStatus {
    let settings = store.get();
    WhatsAppStatus {
        on: settings.whatsapp_bridge_on,
        connected: *state.connected.lock().unwrap(),
        paired: *state.paired.lock().unwrap(),
        qr: state.qr.lock().unwrap().clone(),
        pending: state.pending.lock().unwrap().clone().map(|(chat_id, name)| WhatsAppPending { chat_id, name }),
        problem: state.problem.lock().unwrap().clone(),
    }
}

/// Approves the chat currently held pending (if any) as the only one the bridge answers.
#[tauri::command]
pub fn whatsapp_bridge_approve(store: State<SettingsStore>, state: State<WhatsAppState>) -> Result<(), String> {
    let Some((chat_id, _)) = state.pending.lock().unwrap().take() else {
        return Err("There is no chat waiting to be approved.".into());
    };
    let mut next = store.get();
    next.whatsapp_bridge_chat_id = Some(chat_id);
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

/// Forgets the approved chat, so a new one can pair. The WhatsApp pairing itself (the QR'd
/// session) is untouched - this only changes who the bridge answers, same as Telegram/Discord.
#[tauri::command]
pub fn whatsapp_bridge_forget(store: State<SettingsStore>) -> Result<(), String> {
    let mut next = store.get();
    next.whatsapp_bridge_chat_id = None;
    crate::settings::save(&store.path, &next).map_err(|_| "The choice could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_door_always_points_at_the_local_only_port() {
        assert_eq!(door("/status"), "http://127.0.0.1:51847/status");
        assert_eq!(door("/send"), "http://127.0.0.1:51847/send");
    }

    #[test]
    fn the_helpers_lines_read_with_or_without_the_newer_fields() {
        let old: IncomingMessage = serde_json::from_str(r#"{"chatId":"1@s.whatsapp.net","name":"Rin","text":"hi"}"#).unwrap();
        assert_eq!(old.kind, "text");
        assert_eq!(old.on_id, None);
        let heart: IncomingMessage = serde_json::from_str(r#"{"chatId":"1@s.whatsapp.net","pn":null,"name":"Rin","kind":"reaction","text":"","emoji":"❤️","onId":"ABC"}"#).unwrap();
        assert_eq!((heart.kind.as_str(), heart.emoji.as_str(), heart.on_id.as_deref()), ("reaction", "❤️", Some("ABC")));
    }
}
