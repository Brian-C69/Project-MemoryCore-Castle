//! The member's keys, kept encrypted by Windows Credential Manager (tied to their Windows sign-in).
//! A key never touches a file and never reaches the page except masked, or when the member
//! asks to see it with Show.

use keyring::Entry;
use serde::Serialize;

const SERVICE: &str = "com.memorycore.castle";

/// Every key Castle can keep: (account, label shown in Settings).
pub const KEYS: [(&str, &str); 6] = [
    ("openai", "ChatGPT (OpenAI)"),
    ("seedance", "Seedance"),
    ("suno", "Suno"),
    ("elevenlabs", "ElevenLabs"),
    ("room", "MemoryCore room key"),
    ("room-companion", "Companion room key"),
];

fn known(account: &str) -> Result<(), String> {
    if KEYS.iter().any(|(name, _)| *name == account) {
        Ok(())
    } else {
        Err(format!("Castle does not keep a key called {account}."))
    }
}

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, account).map_err(|_| "Windows Credential Manager could not be opened.".to_string())
}

fn save_as(account: &str, key: &str) -> Result<(), String> {
    entry(account)?
        .set_password(key)
        .map_err(|_| "The key could not be kept in Windows Credential Manager.".to_string())
}

fn load_as(account: &str) -> Option<String> {
    entry(account).ok()?.get_password().ok().filter(|key| !key.is_empty())
}

fn forget_as(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("The key could not be removed from Windows Credential Manager.".to_string()),
    }
}

/// Shows only the ends of a key: "sk-p…a9F2". Short keys show only the last two characters.
pub fn mask(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        let tail: String = chars[chars.len().saturating_sub(2)..].iter().collect();
        return format!("…{tail}");
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// What Settings shows for one key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub name: String,
    pub label: String,
    pub set: bool,
    pub masked: Option<String>,
}

pub fn statuses() -> Vec<KeyStatus> {
    KEYS.iter()
        .map(|(name, label)| {
            let key = load_as(name);
            KeyStatus {
                name: (*name).into(),
                label: (*label).into(),
                set: key.is_some(),
                masked: key.as_deref().map(mask),
            }
        })
        .collect()
}

pub fn save(account: &str, key: &str) -> Result<(), String> {
    known(account)?;
    let key = key.trim();
    if key.is_empty() {
        return Err("The key is empty.".into());
    }
    save_as(account, key)
}

/// The key itself, for the CLI's environment or the room client. Never sent to the page
/// except through `reveal`.
pub fn load(account: &str) -> Option<String> {
    known(account).ok()?;
    load_as(account)
}

pub fn forget(account: &str) -> Result<(), String> {
    known(account)?;
    forget_as(account)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_kept_read_back_and_forgotten() {
        let account = format!("castle-test-{}", std::process::id());
        assert_eq!(load_as(&account), None);
        save_as(&account, "sk-test-only-0000").unwrap();
        assert_eq!(load_as(&account).as_deref(), Some("sk-test-only-0000"));
        forget_as(&account).unwrap();
        assert_eq!(load_as(&account), None);
        // Forgetting twice is fine.
        forget_as(&account).unwrap();
    }

    #[test]
    fn masks_show_only_the_ends() {
        assert_eq!(mask("sk-proj-abcdefgh1234a9F2"), "sk-p…a9F2");
        assert_eq!(mask("abc"), "…bc");
        assert_eq!(mask(""), "…");
    }

    #[test]
    fn only_known_keys_are_kept() {
        assert!(save("not-a-key", "x").is_err());
        assert!(load("not-a-key").is_none());
        assert!(save("openai", "   ").is_err());
        assert_eq!(statuses().len(), 6);
    }
}
