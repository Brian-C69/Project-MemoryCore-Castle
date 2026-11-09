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
    // the member's sign-in to the MemoryCore Foundation room (made by Sign in, never pasted)
    ("member", "MemoryCore sign-in"),
    // the companion's own room key (made by Castle with the member's sign-in, never shown)
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

/// What a key test found, in words.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyTest {
    pub ok: bool,
    pub message: String,
}

/// Checks a kept key against the service's free account endpoint - never anything that costs.
pub fn test(account: &str) -> KeyTest {
    let said = |ok: bool, message: &str| KeyTest { ok, message: message.into() };
    let Some(key) = load(account) else { return said(false, "No key is kept for this yet.") };
    let (url, header) = match account {
        "openai" => ("https://api.openai.com/v1/models", "bearer"),
        "suno" => ("https://api.sunoapi.org/api/v1/generate/credit", "bearer"),
        "elevenlabs" => ("https://api.elevenlabs.io/v1/user", "xi-api-key"),
        "seedance" => return said(true, "Kept. Seedance has no free check, so Castle cannot test it without spending."),
        _ => return said(true, "Kept. The room checks it each time it is used."),
    };
    let client = match reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(_) => return said(false, "The check could not start."),
    };
    let request = client.get(url);
    let request = if header == "bearer" { request.bearer_auth(&key) } else { request.header(header, &key) };
    match request.send() {
        Ok(response) if response.status().is_success() => said(true, "The key works."),
        Ok(response) if response.status().as_u16() == 401 || response.status().as_u16() == 403 => said(false, "The service refused this key. Check it and save it again."),
        Ok(response) => said(false, &format!("The service answered {}. Try again later.", response.status().as_u16())),
        Err(_) => said(false, "The service could not be reached. Check the internet connection."),
    }
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
