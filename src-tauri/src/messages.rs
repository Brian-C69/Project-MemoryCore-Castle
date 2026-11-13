//! Messages accepted during a provider switch or image job, separate from either CLI queue.
use crate::{
    providers::{Control, Images},
    session::{MainHost, StoryHost},
    settings::SettingsStore,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};
#[derive(Default)]
pub struct Pending(pub Mutex<Vec<Message>>);
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: String,
    text: String,
    memory: PathBuf,
    pub state: String,
}
#[derive(Serialize)]
pub struct Sent {
    queued: bool,
}
fn memory(store: &SettingsStore) -> Result<PathBuf, String> {
    PathBuf::from(
        store
            .get()
            .memory_folder
            .ok_or("Choose a MemoryCore folder first.")?,
    )
    .canonicalize()
    .map_err(|_| "The memory folder is gone.".into())
}
fn save(store: &SettingsStore, messages: &[Message]) -> Result<(), String> {
    let path = store.folder().join("pending-messages.json");
    let temp = path.with_extension("pending");
    std::fs::write(
        &temp,
        serde_json::to_vec(messages).map_err(|_| "Message queue could not be encoded.")?,
    )
    .map_err(|_| "Message queue could not be saved.")?;
    std::fs::rename(temp, path).map_err(|_| "Message queue could not be published.".into())
}
fn payload(text: &str) -> String {
    format!("\u{1b}[200~{}\u{1b}[201~\r", text)
}

#[tauri::command]
pub fn send_message(text: String, room: Option<String>, app: AppHandle) -> Result<Sent, String> {
    if text.trim().is_empty() || text.len() > 128_000 {
        return Err("Use a nonempty message shorter than 128 KB.".into());
    }
    if room.as_deref() == Some("story") {
        app.state::<StoryHost>()
            .0
            .write(payload(&text).as_bytes())?;
        return Ok(Sent { queued: false });
    }
    let store = app.state::<SettingsStore>();
    let control = app.state::<Control>();
    let guard = control.0.try_lock().ok();
    let blocked = guard
        .as_ref()
        .is_none_or(|s| s.target.is_some() || s.phase == "switching")
        || app
            .state::<Images>()
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|j| j.phase == "working");
    let pending = app.state::<Pending>();
    let mut messages = pending.0.lock().unwrap();
    let selected = memory(&store)?;
    if blocked
        || messages
            .iter()
            .any(|m| m.memory == selected && m.state == "queued")
    {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();
        messages.push(Message {
            id,
            text,
            memory: selected,
            state: "queued".into(),
        });
        if let Err(e) = save(&store, &messages) {
            messages.pop();
            return Err(e);
        }
        return Ok(Sent { queued: true });
    }
    app.state::<MainHost>().0.write(payload(&text).as_bytes())?;
    Ok(Sent { queued: false })
}

#[tauri::command]
pub fn queued_messages(store: State<SettingsStore>, pending: State<Pending>) -> Vec<Message> {
    let Ok(selected) = memory(&store) else {
        return Vec::new();
    };
    pending
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|m| m.memory == selected)
        .cloned()
        .collect()
}

#[tauri::command]
pub fn queued_message_action(
    id: String,
    retry: bool,
    store: State<SettingsStore>,
    pending: State<Pending>,
) -> Result<(), String> {
    let selected = memory(&store)?;
    let mut messages = pending.0.lock().unwrap();
    let at = messages
        .iter()
        .position(|m| m.id == id && m.memory == selected)
        .ok_or("Message not found.")?;
    if retry {
        messages[at].state = "queued".into();
    } else {
        messages.remove(at);
    }
    save(&store, &messages)
}

pub fn start(app: &AppHandle) {
    let store = app.state::<SettingsStore>();
    let mut saved: Vec<Message> = std::fs::read(store.folder().join("pending-messages.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    // A crash may have occurred after sending but before removing the queue entry. Never replay blindly.
    for m in &mut saved {
        m.state = "held-after-restart".into();
    }
    *app.state::<Pending>().0.lock().unwrap() = saved;
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let control = app.state::<Control>();
        let Ok(guard) = control.0.try_lock() else {
            continue;
        };
        if guard.target.is_some()
            || guard.phase == "switching"
            || app
                .state::<Images>()
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|j| j.phase == "working")
        {
            continue;
        }
        let store = app.state::<SettingsStore>();
        let main = app.state::<MainHost>();
        if !main.0.is_running() || !crate::providers::safe_boundary(&store, &main.0) {
            continue;
        }
        let Ok(selected) = memory(&store) else {
            continue;
        };
        let pending = app.state::<Pending>();
        let mut messages = pending.0.lock().unwrap();
        let Some(at) = messages
            .iter()
            .position(|m| m.memory == selected && m.state == "queued")
        else {
            continue;
        };
        if main.0.write(payload(&messages[at].text).as_bytes()).is_ok() {
            // Retain as held if queue publication fails; do not resend on a subsequent tick.
            messages[at].state = "sent".into();
            let sent = messages.remove(at);
            if save(&store, &messages).is_err() {
                let mut held = sent;
                held.state = "held-check-chat".into();
                messages.insert(at, held);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sends_multiline_text_as_one_bracketed_paste() {
        assert_eq!(payload("one\ntwo"), "\u{1b}[200~one\ntwo\u{1b}[201~\r");
    }
}
