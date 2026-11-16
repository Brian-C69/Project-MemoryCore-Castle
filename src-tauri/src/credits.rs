//! HOME's credit widgets: what is left on the member's OpenAI, Seedance, Suno and ElevenLabs
//! accounts.
//!
//! Suno and ElevenLabs answer with the key Castle already keeps. OpenAI and BytePlus have no
//! balance a key can read: their figure comes from the member's own signed-in billing page
//! (`page_reader`), or from a figure the member types. Between two readings Castle counts that
//! figure on with the spend its own helpers log (one line per real render, written by
//! `castle-common.ps1`). A key never appears in an answer or an error.

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const SUNO_CREDIT: &str = "https://api.sunoapi.org/api/v1/generate/credit";
const ELEVEN_SUBSCRIPTION: &str = "https://api.elevenlabs.io/v1/user/subscription";
/// Suno and ElevenLabs are asked at most this often.
const EVERY: Duration = Duration::from_secs(10 * 60);
/// The event that tells the widgets to read again.
pub const CHANGED_EVENT: &str = "castle://credits-changed";

/// A figure read from a billing page or typed by the member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Figure {
    pub usd: f64,
    /// Milliseconds since 1970.
    pub at: i64,
    /// "openai", "byteplus" (read from the page) or "hand" (typed).
    pub source: String,
    /// The month the figure belongs to ("YYYY-MM"); Seedance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
}

/// How the last page read went: "signed-in", "signed-out" or "no-answer".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadState {
    pub state: String,
    pub at: i64,
}

/// What Castle keeps between runs, in its settings file. Changed only by this module's commands.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreditsSaved {
    pub openai: Option<Figure>,
    pub seedance: Option<Figure>,
    pub openai_state: Option<ReadState>,
    pub byteplus_state: Option<ReadState>,
    /// Suno's whole for the bar: the highest balance since the last top-up, and the last one seen.
    pub suno_peak: Option<f64>,
    pub suno_last: Option<f64>,
    /// The member's monthly Seedance budget; no budget, no bar.
    pub seedance_budget: Option<f64>,
}

/// One widget's line, ready for the page.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub provider: String,
    pub has_key: bool,
    /// USD for OpenAI (left) and Seedance (spent this month), credits for Suno, characters left
    /// for ElevenLabs. None when unknown.
    pub value: Option<f64>,
    /// True when the value is counted on from spend logged since the figure, or typed by hand.
    pub estimate: bool,
    /// The bar: the value's whole (OpenAI, Suno, ElevenLabs) or the budget (Seedance). None, no bar.
    pub whole: Option<f64>,
    /// ElevenLabs: characters used and the plan's limit.
    pub used: Option<f64>,
    pub limit: Option<f64>,
    pub resets_at: Option<i64>,
    /// The figure the estimate starts from, when it was read or typed, and where from.
    pub base: Option<f64>,
    pub at: Option<i64>,
    pub source: Option<String>,
    /// Renders logged since the figure.
    pub calls_since: u32,
    /// The page reader's last word (OpenAI and Seedance).
    pub state: Option<ReadState>,
    pub note: Option<String>,
}

// ---------- the spend log ----------

/// The helpers' spend folder (`castle-common.ps1` writes the same place).
fn spend_folder() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(app_data).join("MemoryCore Castle").join("spend"))
}

fn spend_log(provider: &str) -> String {
    spend_folder().and_then(|dir| std::fs::read_to_string(dir.join(format!("{provider}-spend.jsonl"))).ok()).unwrap_or_default()
}

/// The USD and the number of renders logged after `after` (and at or after `from`).
pub fn spent_since(log: &str, after: i64, from: i64) -> (f64, u32) {
    let (mut sum, mut calls) = (0.0, 0u32);
    for line in log.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
        let (Some(at), Some(usd)) = (v.get("at").and_then(Value::as_str).and_then(crate::quota::iso_ms), v.get("usd").and_then(Value::as_f64)) else {
            continue;
        };
        if at > after && at >= from {
            sum += usd;
            calls += 1;
        }
    }
    (sum, calls)
}

// ---------- the countdowns (pure) ----------

fn cents(usd: f64) -> f64 {
    (usd * 100.0).round() / 100.0
}

/// OpenAI: the last figure minus the spend logged since it; the bar's whole is that figure.
pub fn openai_line(figure: Option<&Figure>, log: &str) -> Credit {
    let Some(f) = figure else { return Credit { provider: "openai".into(), ..Credit::default() } };
    let (spent, calls) = spent_since(log, f.at, i64::MIN);
    Credit {
        provider: "openai".into(),
        value: Some(cents((f.usd - spent).max(0.0))),
        estimate: calls > 0 || f.source == "hand",
        whole: (f.usd > 0.0).then_some(f.usd),
        base: Some(f.usd),
        at: Some(f.at),
        source: Some(f.source.clone()),
        calls_since: calls,
        ..Credit::default()
    }
}

/// Seedance: this month's figure (when it belongs to this month) plus the renders logged after it,
/// this month only; the bar's whole is the member's budget.
pub fn seedance_line(figure: Option<&Figure>, log: &str, month: &str, month_start: i64, budget: Option<f64>) -> Credit {
    let this_month = figure.filter(|f| f.month.as_deref() == Some(month));
    let after = this_month.map(|f| f.at).unwrap_or(i64::MIN);
    let (spent, calls) = spent_since(log, after, month_start);
    let base = this_month.map(|f| f.usd).unwrap_or(0.0);
    Credit {
        provider: "seedance".into(),
        value: Some(cents(base + spent)),
        estimate: calls > 0 || this_month.is_none_or(|f| f.source == "hand"),
        whole: budget.filter(|b| *b > 0.0),
        base: this_month.map(|f| f.usd),
        at: this_month.map(|f| f.at),
        source: this_month.map(|f| f.source.clone()),
        calls_since: calls,
        ..Credit::default()
    }
}

/// Suno's bar: a rise in credits is a top-up and starts a new whole; otherwise the whole is the
/// highest balance seen since. Returns (peak, last) to keep.
pub fn suno_whole(credits: f64, peak: Option<f64>, last: Option<f64>) -> (f64, f64) {
    let peak = match (peak, last) {
        (Some(p), Some(l)) if credits <= l => p.max(credits),
        _ => credits,
    };
    (peak, credits)
}

// ---------- the key-read balances ----------

/// Suno's answer: `data` as a number or numeric text, or data.credits / credit / remaining.
pub fn parse_suno(answer: &Value) -> Option<f64> {
    let data = answer.get("data")?;
    data.as_f64()
        .or_else(|| data.as_str().and_then(|s| s.trim().parse().ok()))
        .or_else(|| ["credits", "credit", "remaining"].iter().find_map(|k| data.get(*k).and_then(Value::as_f64)))
}

/// ElevenLabs' subscription answer: (used, limit, resets_at in ms).
pub fn parse_eleven(answer: &Value) -> (Option<f64>, Option<f64>, Option<i64>) {
    (
        answer.get("character_count").and_then(Value::as_f64),
        answer.get("character_limit").and_then(Value::as_f64),
        answer.get("next_character_count_reset_unix").and_then(Value::as_i64).map(|s| s * 1000),
    )
}

/// What an ElevenLabs refusal means: a key made for voices only cannot read the plan.
pub fn eleven_refusal(status: u16, body: &str) -> String {
    if body.contains("user_read") {
        "This ElevenLabs key can make voices but cannot read the plan. Give it User: Read on elevenlabs.io (API keys).".into()
    } else if status == 401 {
        "ElevenLabs refused this key. Check it in Settings > Keys.".into()
    } else {
        format!("ElevenLabs answered {status}.")
    }
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()
        .ok()
}

#[derive(Debug, Clone, Default)]
struct Asked {
    suno: Option<f64>,
    suno_note: Option<String>,
    eleven: (Option<f64>, Option<f64>, Option<i64>),
    eleven_note: Option<String>,
    at: i64,
}

fn ask_suno(key: &str) -> Result<f64, String> {
    let client = client().ok_or("Castle could not ask Suno.")?;
    match client.get(SUNO_CREDIT).bearer_auth(key).send() {
        Ok(r) if r.status().is_success() => r.json::<Value>().ok().as_ref().and_then(parse_suno).ok_or_else(|| "Suno answered in a shape Castle does not know.".to_string()),
        Ok(r) if r.status().as_u16() == 401 => Err("Suno refused this key. Check it in Settings > Keys.".into()),
        Ok(r) => Err(format!("Suno answered {}.", r.status().as_u16())),
        Err(_) => Err("Suno could not be reached.".into()),
    }
}

fn ask_eleven(key: &str) -> Result<(Option<f64>, Option<f64>, Option<i64>), String> {
    let client = client().ok_or("Castle could not ask ElevenLabs.")?;
    match client.get(ELEVEN_SUBSCRIPTION).header("xi-api-key", key).send() {
        Ok(r) if r.status().is_success() => r.json::<Value>().map(|v| parse_eleven(&v)).map_err(|_| "ElevenLabs' answer could not be read.".to_string()),
        Ok(r) => {
            let status = r.status().as_u16();
            Err(eleven_refusal(status, &r.text().unwrap_or_default()))
        }
        Err(_) => Err("ElevenLabs could not be reached.".into()),
    }
}

static LAST: Mutex<Option<(Instant, Asked)>> = Mutex::new(None);

/// A key saved or removed: the next read asks again.
pub fn forget_cache() {
    *LAST.lock().unwrap() = None;
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn asked() -> Asked {
    if let Some((when, answer)) = LAST.lock().unwrap().as_ref() {
        if when.elapsed() < EVERY {
            return answer.clone();
        }
    }
    let mut answer = Asked { at: now_ms(), ..Asked::default() };
    if let Some(key) = crate::key::load("suno") {
        match ask_suno(&key) {
            Ok(credits) => answer.suno = Some(credits),
            Err(note) => answer.suno_note = Some(note),
        }
    }
    if let Some(key) = crate::key::load("elevenlabs") {
        match ask_eleven(&key) {
            Ok(plan) => answer.eleven = plan,
            Err(note) => answer.eleven_note = Some(note),
        }
    }
    *LAST.lock().unwrap() = Some((Instant::now(), answer.clone()));
    answer
}

// ---------- saving ----------

/// Changes the kept credit state and saves the settings file.
pub fn save_with(store: &SettingsStore, change: impl FnOnce(&mut CreditsSaved)) -> Result<CreditsSaved, String> {
    let mut next = store.get();
    change(&mut next.credits);
    crate::settings::save(&store.path, &next).map_err(|_| "The credits could not be saved.".to_string())?;
    let saved = next.credits.clone();
    *store.current.lock().unwrap() = next;
    Ok(saved)
}

/// Tells the widgets to read again.
pub fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, ());
}

fn has_key(account: &str) -> bool {
    crate::key::load(account).is_some()
}

/// Every widget's line. `month` ("YYYY-MM") and `month_start` (ms) are the member's own month,
/// from the page's clock.
pub fn read_all(store: &SettingsStore, month: &str, month_start: i64) -> Vec<Credit> {
    let answer = asked();
    let saved = store.get().credits;

    let mut openai = openai_line(saved.openai.as_ref(), &spend_log("openai"));
    openai.has_key = has_key("openai");
    openai.state = saved.openai_state.clone();

    let mut seedance = seedance_line(saved.seedance.as_ref(), &spend_log("seedance"), month, month_start, saved.seedance_budget);
    seedance.has_key = has_key("seedance");
    seedance.state = saved.byteplus_state.clone();

    let mut suno = Credit { provider: "suno".into(), has_key: has_key("suno"), note: answer.suno_note.clone(), ..Credit::default() };
    if let Some(credits) = answer.suno {
        let (peak, last) = suno_whole(credits, saved.suno_peak, saved.suno_last);
        if saved.suno_peak != Some(peak) || saved.suno_last != Some(last) {
            let _ = save_with(store, |c| {
                c.suno_peak = Some(peak);
                c.suno_last = Some(last);
            });
        }
        suno.value = Some(credits);
        suno.whole = (peak > 0.0).then_some(peak);
        suno.at = Some(answer.at);
        suno.source = Some("api".into());
    }

    let (used, limit, resets_at) = answer.eleven;
    let mut eleven = Credit { provider: "elevenlabs".into(), has_key: has_key("elevenlabs"), note: answer.eleven_note.clone(), ..Credit::default() };
    if let (Some(used), Some(limit)) = (used, limit) {
        eleven.value = Some((limit - used).max(0.0));
        eleven.whole = (limit > 0.0).then_some(limit);
        eleven.used = Some(used);
        eleven.limit = Some(limit);
        eleven.resets_at = resets_at;
        eleven.at = Some(answer.at);
        eleven.source = Some("api".into());
    }

    vec![openai, seedance, suno, eleven]
}

// ---------- commands ----------

#[tauri::command]
pub async fn credits_read(app: AppHandle, month: String, month_start: i64) -> Result<Vec<Credit>, String> {
    tauri::async_runtime::spawn_blocking(move || read_all(&app.state::<SettingsStore>(), &month, month_start))
        .await
        .map_err(|_| "The credits could not be read.".to_string())
}

/// The typed backup: the member's own figure for OpenAI (left) or Seedance (spent this month).
#[tauri::command]
pub fn credits_set(app: AppHandle, store: State<SettingsStore>, vendor: String, usd: f64, month: Option<String>) -> Result<(), String> {
    if !usd.is_finite() || !(0.0..=1_000_000.0).contains(&usd) {
        return Err("Type an amount in USD, like 12.50.".into());
    }
    let figure = Figure { usd: cents(usd), at: now_ms(), source: "hand".into(), month: None };
    match vendor.as_str() {
        "openai" => save_with(&store, |c| c.openai = Some(figure))?,
        "seedance" => {
            let month = month.filter(|m| m.len() == 7).ok_or("The month is missing.")?;
            save_with(&store, |c| c.seedance = Some(Figure { month: Some(month), ..figure }))?
        }
        _ => return Err("Only OpenAI and Seedance take a typed figure.".into()),
    };
    changed(&app);
    Ok(())
}

/// The member's monthly Seedance budget for the bar; None or 0 removes it.
#[tauri::command]
pub fn credits_budget(app: AppHandle, store: State<SettingsStore>, usd: Option<f64>) -> Result<(), String> {
    let budget = usd.filter(|u| u.is_finite() && *u > 0.0).map(cents);
    save_with(&store, |c| c.seedance_budget = budget)?;
    changed(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ms(text: &str) -> i64 {
        crate::quota::iso_ms(text).unwrap()
    }

    const LOG: &str = "{\"at\":\"2026-11-10T02:00:00Z\",\"usd\":0.04,\"kind\":\"image\"}\n\
                       not json\n\
                       {\"at\":\"2026-11-16T03:00:00Z\",\"usd\":0.25,\"kind\":\"image\"}\n\
                       {\"at\":\"2026-11-16T04:00:00Z\",\"usd\":0.5,\"kind\":\"image\"}\n";

    #[test]
    fn the_spend_after_a_figure_is_counted_once_per_line() {
        assert_eq!(spent_since(LOG, ms("2026-11-16T00:00:00Z"), i64::MIN), (0.75, 2));
        assert_eq!(spent_since(LOG, i64::MIN, i64::MIN), (0.79, 3));
        assert_eq!(spent_since("", 0, 0), (0.0, 0));
    }

    #[test]
    fn openai_counts_down_from_the_last_figure() {
        let read = Figure { usd: 18.22, at: ms("2026-11-16T00:00:00Z"), source: "openai".into(), month: None };
        let line = openai_line(Some(&read), LOG);
        assert_eq!((line.value, line.estimate, line.calls_since, line.whole), (Some(17.47), true, 2, Some(18.22)));
        let fresh = Figure { at: ms("2026-11-17T00:00:00Z"), ..read.clone() };
        let line = openai_line(Some(&fresh), LOG);
        assert_eq!((line.value, line.estimate), (Some(18.22), false));
        let typed = Figure { source: "hand".into(), ..fresh };
        assert!(openai_line(Some(&typed), LOG).estimate);
        assert_eq!(openai_line(None, LOG).value, None);
        let spent = Figure { usd: 0.5, at: 0, source: "openai".into(), month: None };
        assert_eq!(openai_line(Some(&spent), LOG).value, Some(0.0));
    }

    #[test]
    fn seedance_counts_this_month_only_and_starts_again_each_month() {
        let start = ms("2026-11-01T00:00:00Z");
        let read = Figure { usd: 2.0, at: ms("2026-11-16T03:30:00Z"), source: "byteplus".into(), month: Some("2026-11".into()) };
        let line = seedance_line(Some(&read), LOG, "2026-11", start, Some(10.0));
        assert_eq!((line.value, line.calls_since, line.whole, line.estimate), (Some(2.5), 1, Some(10.0), true));
        let old = Figure { month: Some("2026-10".into()), ..read.clone() };
        let line = seedance_line(Some(&old), LOG, "2026-11", start, None);
        assert_eq!((line.value, line.base, line.whole), (Some(0.79), None, None));
        let december = seedance_line(Some(&read), LOG, "2026-12", ms("2026-12-01T00:00:00Z"), None);
        assert_eq!(december.value, Some(0.0));
    }

    #[test]
    fn a_rise_in_suno_credits_starts_a_new_whole() {
        assert_eq!(suno_whole(768.0, None, None), (768.0, 768.0));
        assert_eq!(suno_whole(700.0, Some(768.0), Some(768.0)), (768.0, 700.0));
        assert_eq!(suno_whole(1500.0, Some(768.0), Some(700.0)), (1500.0, 1500.0));
    }

    #[test]
    fn the_key_read_answers_are_parsed() {
        assert_eq!(parse_suno(&json!({"code":200,"data":768})), Some(768.0));
        assert_eq!(parse_suno(&json!({"data":"42.5"})), Some(42.5));
        assert_eq!(parse_suno(&json!({"data":{"credits":9}})), Some(9.0));
        assert_eq!(parse_suno(&json!({"msg":"no"})), None);
        let plan = json!({"character_count":18258,"character_limit":37670,"next_character_count_reset_unix":1764720000,"tier":"creator"});
        assert_eq!(parse_eleven(&plan), (Some(18258.0), Some(37670.0), Some(1_764_720_000_000)));
        assert!(eleven_refusal(401, "{\"detail\":{\"status\":\"missing_permissions\",\"message\":\"user_read\"}}").contains("User: Read"));
        assert!(eleven_refusal(500, "").contains("500"));
    }
}
