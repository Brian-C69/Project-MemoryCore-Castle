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

#[derive(Deserialize)]
struct IncomingMessage {
    #[serde(rename = "chatId")]
    chat_id: String,
    name: String,
    text: String,
}

fn door(path: &str) -> String {
    format!("http://127.0.0.1:{PORT}{path}")
}

fn auth_dir(store: &SettingsStore) -> PathBuf {
    store.folder().join("whatsapp-auth")
}

fn script_path(app: &AppHandle) -> Option<PathBuf> {
    let path = app.path().resource_dir().ok()?.join("tools").join("castle-whatsapp.cjs");
    path.is_file().then_some(path)
}

fn running(state: &WhatsAppState) -> bool {
    state.child.lock().unwrap().is_some()
}

fn ensure_started(app: &AppHandle, store: &SettingsStore, state: &WhatsAppState) -> Result<(), String> {
    if running(state) {
        return Ok(());
    }
    let script = script_path(app).ok_or("The WhatsApp helper is missing from this install.")?;
    let child = Command::new(crate::env::node())
        .arg(&script)
        .env("WA_AUTH_DIR", auth_dir(store))
        .env("WA_PORT", PORT.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|_| "The WhatsApp helper could not be started.".to_string())?;
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
                let _ = client.post(door("/send")).json(&json!({ "to": chat_id, "text": reply })).send();
            }
            Some(_) => {}
            None => {
                *app.state::<WhatsAppState>().pending.lock().unwrap() = Some((chat_id.clone(), name));
                let _ = client.post(door("/send")).json(&json!({ "to": chat_id, "text": "Hi! Approve this chat on Castle's MESSAGING room before I can answer." })).send();
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
}
