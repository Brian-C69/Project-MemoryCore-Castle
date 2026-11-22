//! Automation notices: only the existing approved chat, plain text, bounded and rate limited.
use crate::settings::{Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
static SENDING: Mutex<()> = Mutex::new(());
const PER_HOUR: usize = 6;
#[derive(Default, Serialize, Deserialize)]
struct Budget(BTreeMap<String, Vec<u64>>);
pub fn target(s: &Settings, platform: &str) -> Result<String, String> {
    if !s.room_on("messaging") {
        return Err("MESSAGING is switched off.".into());
    }
    let (on, id) = match platform {
        "telegram" => (s.telegram_bridge_on, s.telegram_bridge_chat_id.as_deref()),
        "discord" => (s.discord_bridge_on, s.discord_bridge_channel_id.as_deref()),
        "whatsapp" => (s.whatsapp_bridge_on, s.whatsapp_bridge_chat_id.as_deref()),
        _ => return Err("Choose Telegram, Discord or WhatsApp.".into()),
    };
    if !on {
        return Err("This messaging bridge is switched off.".into());
    }
    let id = id
        .filter(|s| !s.is_empty() && s.len() <= 160 && !s.contains(['\r', '\n']))
        .ok_or("Approve a chat in MESSAGING before delivering to it.")?;
    if platform == "telegram" && id.parse::<i64>().ok().is_none_or(|n| n == 0) {
        return Err("Approve a valid Telegram chat first.".into());
    }
    if platform == "discord" && id.parse::<u64>().ok().is_none_or(|n| n == 0) {
        return Err("Approve a valid Discord chat first.".into());
    }
    Ok(id.into())
}
pub fn parts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut part = String::new();
    let mut units = 0;
    for c in text.chars() {
        if units + c.len_utf16() > 1800 {
            out.push(std::mem::take(&mut part));
            units = 0;
        }
        part.push(c);
        units += c.len_utf16();
    }
    if !part.is_empty() {
        out.push(part);
    }
    out
}
fn reserve(file: &Path, platform: &str, count: usize, now: u64) -> Result<(), String> {
    let mut b = match std::fs::read(file) {
        Ok(bytes) => serde_json::from_slice::<Budget>(&bytes).map_err(|_| {
            "The phone delivery limit could not be read; no message was sent.".to_string()
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Budget::default(),
        Err(_) => {
            return Err("The phone delivery limit could not be read; no message was sent.".into())
        }
    };
    for stamps in b.0.values_mut() {
        stamps.retain(|t| now.saturating_sub(*t) < 3600);
    }
    let stamps = b.0.entry(platform.into()).or_default();
    if stamps.len() + count > PER_HOUR {
        return Err(
            "Phone delivery limit reached: at most six message parts per platform each hour."
                .into(),
        );
    }
    stamps.extend(std::iter::repeat_n(now, count));
    let partial = file.with_extension("json.partial");
    let bytes = serde_json::to_vec(&b)
        .map_err(|_| "The phone delivery limit could not be saved.".to_string())?;
    std::fs::write(&partial, bytes)
        .and_then(|_| std::fs::rename(&partial, file))
        .map_err(|_| {
            "The phone delivery limit could not be saved; no message was sent.".to_string()
        })
}
fn dispatch(
    file: &Path,
    platform: &str,
    text: &str,
    now: u64,
    mut allowed: impl FnMut() -> bool,
    mut send: impl FnMut(&str) -> Result<(), String>,
) -> Result<usize, String> {
    let _guard = SENDING.lock().unwrap();
    if !allowed() {
        return Err("Delivery stopped: automations paused or the approved chat changed.".into());
    }
    let chunks = parts(text);
    if chunks.is_empty() {
        return Err("There is no text to deliver.".into());
    }
    reserve(file, platform, chunks.len(), now)?;
    for (i, part) in chunks.iter().enumerate() {
        if !allowed() {
            return Err(format!("Delivery stopped after {i} parts: automations paused or the approved chat changed."));
        }
        send(part).map_err(|e| format!("Delivery failed after {i} parts: {e}"))?;
    }
    Ok(chunks.len())
}
pub fn notify(app: &AppHandle, platform: &str, text: &str) -> Result<usize, String> {
    let store = app.state::<SettingsStore>();
    let expected = target(&store.get(), platform)?;
    if !ready(&store.get(), platform) {
        return Err("Finish this bridge setup in MESSAGING before delivery.".into());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "The clock could not be read.".to_string())?
        .as_secs();
    dispatch(
        &store.folder().join("automation-delivery-budget.json"),
        platform,
        text,
        now,
        || {
            let s = store.get();
            s.room_on("automations")
                && !s.automations_paused
                && target(&s, platform).ok().as_deref() == Some(expected.as_str())
        },
        |part| match platform {
            "telegram" => super::telegram::notify(&store, &expected, part),
            "discord" => super::discord::notify(&store, &expected, part),
            "whatsapp" => super::whatsapp::notify(app, &store, &expected, part),
            _ => unreachable!(),
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn off_unapproved_and_unknown_platforms_are_refused() {
        let mut s = Settings::default();
        assert!(target(&s, "telegram").is_err());
        s.rooms.insert("messaging".into(), true);
        s.telegram_bridge_on = true;
        assert!(target(&s, "telegram").is_err());
        s.telegram_bridge_chat_id = Some("123".into());
        assert_eq!(target(&s, "telegram").unwrap(), "123");
        assert!(target(&s, "other").is_err());
        s.telegram_bridge_on = false;
        assert!(target(&s, "telegram").is_err());
    }
    #[test]
    fn unicode_splits_are_bounded_and_lossless() {
        let text = "\u{1f30a}".repeat(2400);
        let pieces = parts(&text);
        assert_eq!(pieces.concat(), text);
        assert!(pieces.iter().all(|p| p.encode_utf16().count() <= 1800));
        assert!(parts("").is_empty());
    }
    #[test]
    fn persistent_budget_stops_flood_and_expires() {
        let f =
            std::env::temp_dir().join(format!("castle-delivery-limit-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        let mut sent = 0;
        for _ in 0..6 {
            dispatch(
                &f,
                "telegram",
                "ok",
                100,
                || true,
                |_| {
                    sent += 1;
                    Ok(())
                },
            )
            .unwrap();
        }
        assert!(dispatch(
            &f,
            "telegram",
            "no",
            101,
            || true,
            |_| panic!("must not send")
        )
        .is_err());
        assert_eq!(sent, 6);
        assert!(dispatch(&f, "telegram", "later", 3700, || true, |_| Ok(())).is_ok());
        let _ = std::fs::remove_file(&f);
    }
    #[test]
    fn kill_switch_and_partial_failure_do_not_continue() {
        let f =
            std::env::temp_dir().join(format!("castle-delivery-stop-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        assert!(dispatch(&f, "telegram", "x", 100, || false, |_| panic!()).is_err());
        assert!(!f.exists());
        let mut checks = 0;
        let mut sent = 0;
        let text = "x".repeat(4000);
        assert!(dispatch(
            &f,
            "telegram",
            &text,
            100,
            || {
                checks += 1;
                checks < 3
            },
            |_| {
                sent += 1;
                Ok(())
            }
        )
        .is_err());
        assert_eq!(sent, 1);
        let _ = std::fs::remove_file(&f);
    }
}

pub fn ready(s: &Settings, platform: &str) -> bool {
    target(s, platform).is_ok()
        && match platform {
            "telegram" => crate::key::load("telegram-bridge").is_some(),
            "discord" => crate::key::load("discord-bridge").is_some(),
            "whatsapp" => true,
            _ => false,
        }
}
