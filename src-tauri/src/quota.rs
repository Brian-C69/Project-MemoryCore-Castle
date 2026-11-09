//! How much of the member's weekly CLI limit is left - the great crystal shines with it.
//!
//! Claude Code: the endpoint behind Claude Code's `/usage`, asked with the login Claude Code
//! keeps in `~/.claude/.credentials.json`. It is not a documented API and may change: when it
//! fails the crystal simply keeps its normal glow. The token is read for each call, goes out only
//! in the Authorization header, and never reaches a page or a log.
//!
//! Codex: its session files carry the limits it was last told (`token_count` lines with
//! `rate_limits`); the newest one is read.

use crate::settings::{Cli, SettingsStore};
use crate::transcript::codex;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::State;

const CLAUDE_USAGE: &str = "https://api.anthropic.com/api/oauth/usage";
const EVERY: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quota {
    pub cli: String,
    /// 0..100 of the weekly limit still left.
    pub left_percent: f64,
    /// Milliseconds since 1970, when known.
    pub resets_at: Option<i64>,
    /// What the number is, in words ("weekly limit").
    pub what: String,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// "2026-11-12T21:00:00.125+00:00" (or Z, or no zone = UTC) -> milliseconds since 1970.
pub fn iso_ms(text: &str) -> Option<i64> {
    let t = text.trim();
    let num = |a: usize, b: usize| t.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let mut rest = t.get(19..)?;
    if let Some(frac) = rest.strip_prefix('.') {
        rest = frac.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset_min = match rest {
        "" | "Z" | "z" => 0,
        zone => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let hh = zone.get(1..3)?.parse::<i64>().ok()?;
            let mm = zone.get(zone.len() - 2..)?.parse::<i64>().ok()?;
            sign * (hh * 60 + mm)
        }
    };
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let doy = (153 * ((mo + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3600 + mi * 60 + s - offset_min * 60) * 1000)
}

fn quota(cli: &str, used: f64, resets_at: Option<i64>) -> Quota {
    Quota { cli: cli.into(), left_percent: (100.0 - used).clamp(0.0, 100.0), resets_at, what: "weekly limit".into() }
}

/// Claude Code's usage answer -> the tightest weekly limit (the one with the least left).
pub fn parse_claude(answer: &Value) -> Option<Quota> {
    let none = Vec::new();
    let weekly = answer
        .get("limits")
        .and_then(Value::as_array)
        .unwrap_or(&none)
        .iter()
        .filter(|l| l.get("group").and_then(Value::as_str) == Some("weekly"))
        .filter_map(|l| Some((l.get("percent")?.as_f64()?, l.get("resets_at").and_then(Value::as_str).and_then(iso_ms))))
        .max_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((used, resets)) = weekly {
        return Some(quota("claude", used, resets));
    }
    let week = answer.get("seven_day")?;
    Some(quota("claude", week.get("utilization")?.as_f64()?, week.get("resets_at").and_then(Value::as_str).and_then(iso_ms)))
}

/// One Codex session line -> the weekly limit it reports (the secondary window, else the primary).
pub fn parse_codex(line: &str, now: i64) -> Option<Quota> {
    let v: Value = serde_json::from_str(line).ok()?;
    let payload = v.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return None;
    }
    let limits = payload.get("rate_limits").or_else(|| payload.pointer("/info/rate_limits"))?;
    let window = limits.get("secondary").filter(|w| !w.is_null()).or_else(|| limits.get("primary"))?;
    let used = window.get("used_percent")?.as_f64()?;
    let resets = window
        .get("resets_at")
        .and_then(|r| r.as_i64().map(|s| s * 1000).or_else(|| r.as_str().and_then(iso_ms)))
        .or_else(|| window.get("resets_in_seconds").and_then(Value::as_i64).map(|s| now + s * 1000));
    Some(quota("codex", used, resets))
}

/// The last limits line in a Codex session file.
fn last_codex_quota(path: &Path) -> Option<Quota> {
    let file = std::fs::File::open(path).ok()?;
    let now = now_ms();
    BufReader::new(file).lines().map_while(Result::ok).filter_map(|l| parse_codex(&l, now)).last()
}

fn codex_quota() -> Option<Quota> {
    let root = codex::sessions_root()?;
    let week_ago = SystemTime::now().checked_sub(Duration::from_secs(8 * 86_400))?;
    let mut files = codex::recent_rollouts(&root, week_ago);
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.iter().take(5).find_map(|(_, path)| last_codex_quota(path))
}

fn claude_token() -> Option<String> {
    let path: PathBuf = PathBuf::from(std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?).join(".claude").join(".credentials.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    v.pointer("/claudeAiOauth/accessToken")?.as_str().map(str::to_string).filter(|t| !t.is_empty())
}

fn claude_quota() -> Option<Quota> {
    let token = claude_token()?;
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(15)).build().ok()?;
    let answer = client.get(CLAUDE_USAGE).bearer_auth(token).header("anthropic-beta", "oauth-2025-04-20").send().ok()?;
    if !answer.status().is_success() {
        return None;
    }
    parse_claude(&answer.json().ok()?)
}

static LAST: Mutex<Option<(Instant, Cli, Option<Quota>)>> = Mutex::new(None);

/// The weekly limit left for the member's CLI, asked at most every five minutes; None = unknown.
#[tauri::command]
pub async fn cli_quota(store: State<'_, SettingsStore>) -> Result<Option<Quota>, String> {
    let cli = store.get().cli;
    if let Some((when, was, answer)) = LAST.lock().unwrap().clone() {
        if was == cli && when.elapsed() < EVERY {
            return Ok(answer);
        }
    }
    let answer = tauri::async_runtime::spawn_blocking(move || match cli {
        Cli::Claude => claude_quota(),
        Cli::Codex => codex_quota(),
    })
    .await
    .unwrap_or(None);
    *LAST.lock().unwrap() = Some((Instant::now(), cli, answer.clone()));
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_gives_the_tightest_weekly_limit() {
        let answer: Value = serde_json::from_str(
            r#"{"limits":[
                {"group":"weekly","kind":"weekly_all","percent":28.0,"resets_at":"2026-11-12T21:00:00+00:00"},
                {"group":"weekly","kind":"weekly_model","percent":61.5,"resets_at":"2026-11-13T05:00:00Z","scope":{"model":{"display_name":"Opus"}}},
                {"group":"session","kind":"five_hour","percent":90.0}
            ]}"#,
        )
        .unwrap();
        let q = parse_claude(&answer).unwrap();
        assert_eq!(q.left_percent, 38.5);
        assert_eq!(q.resets_at, iso_ms("2026-11-13T05:00:00Z"));
        let old: Value = serde_json::from_str(r#"{"seven_day":{"utilization":12.0,"resets_at":"2026-11-12T21:00:00.5+00:00"}}"#).unwrap();
        assert_eq!(parse_claude(&old).unwrap().left_percent, 88.0);
        assert!(parse_claude(&serde_json::json!({})).is_none());
    }

    #[test]
    fn codex_reads_its_weekly_window_from_a_session_line() {
        let line = r#"{"timestamp":"2026-11-09T05:00:00Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{"primary":{"used_percent":40.0,"window_minutes":300,"resets_in_seconds":600},"secondary":{"used_percent":75.0,"window_minutes":10080,"resets_in_seconds":3600}}}}"#;
        let q = parse_codex(line, 1_000_000).unwrap();
        assert_eq!(q.left_percent, 25.0);
        assert_eq!(q.resets_at, Some(1_000_000 + 3_600_000));
        let only_primary = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":10.0,"resets_at":1762700000}}}}"#;
        assert_eq!(parse_codex(only_primary, 0).unwrap().resets_at, Some(1_762_700_000_000));
        assert!(parse_codex(r#"{"type":"event_msg","payload":{"type":"agent_message","message":"hi"}}"#, 0).is_none());
    }

    #[test]
    fn used_over_a_hundred_never_goes_below_nothing_left() {
        assert_eq!(quota("claude", 130.0, None).left_percent, 0.0);
        assert_eq!(quota("claude", -5.0, None).left_percent, 100.0);
    }

    /// Live, on demand only: `cargo test live_claude_quota -- --ignored --nocapture`. Prints the
    /// percentage left, never the login.
    #[test]
    #[ignore]
    fn live_claude_quota() {
        println!("claude weekly left: {:?}", claude_quota().map(|q| (q.left_percent, q.resets_at)));
    }

    #[test]
    fn a_time_with_a_zone_becomes_utc_milliseconds() {
        assert_eq!(iso_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(iso_ms("1970-01-01T08:00:00+08:00"), Some(0));
        assert_eq!(iso_ms("2026-11-12T21:00:00.125837+00:00"), iso_ms("2026-11-12T21:00:00Z"));
    }
}
