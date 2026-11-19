//! STUDIO's Marketplace: workflows members sell to each other, on the Portfolio's doors (the brief
//! `portfolio-handoff-workflow-marketplace.md`, S-1119-1). The paper-trading desk is `market/`; this
//! is the shop. Every call goes from here with the member's sign-in, never from the page, and the
//! page sees only what the doors answered. The tab shows only when `/api/member/me` says
//! `marketplace.live`, and Publish only when it says `marketplace.selling`.

use crate::key;
use crate::settings::SettingsStore;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tauri::State;

const USER_AGENT: &str = concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION"));
const MAX_SAMPLES: usize = 3;
const MAX_SAMPLE_BYTES: u64 = 5 * 1024 * 1024;

/// A door's refusal in plain words: the Portfolio's own `{"error"}` when it sent one.
pub fn words_for(status: u16, body: &str) -> String {
    let said = serde_json::from_str::<Value>(body).ok().and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string)).filter(|s| !s.trim().is_empty());
    match (status, said) {
        (401, _) => "Your sign-in has run out. Sign in again in CHAT.".into(),
        (_, Some(said)) => said.chars().take(300).collect(),
        (404, None) => "The Marketplace is not open yet.".into(),
        (413, None) => "A sample picture is larger than 5 MB.".into(),
        (415, None) => "Sample pictures must be PNG, JPEG or WebP.".into(),
        _ => "The Marketplace could not answer just now.".into(),
    }
}

fn token() -> Result<String, String> {
    key::load("member").ok_or_else(|| "Sign in to the MemoryCore Foundation first (CHAT > Sign in).".to_string())
}

fn client(timeout: Duration) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder().user_agent(USER_AGENT).timeout(timeout).build().map_err(|_| "Castle could not reach the internet.".to_string())
}

fn answer(response: Result<reqwest::blocking::Response, reqwest::Error>) -> Result<Value, String> {
    let response = response.map_err(|_| "The Portfolio could not be reached. Check the internet connection.".to_string())?;
    let status = response.status().as_u16();
    let body = response.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(words_for(status, &body));
    }
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).map_err(|_| "The Marketplace sent something unreadable.".to_string())
}

/// One JSON call to a Marketplace door. `base` is the Portfolio (a test passes its own).
pub fn call(base: &str, token: &str, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value, String> {
    let mut request = client(Duration::from_secs(30))?.request(method, format!("{base}{path}")).bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    answer(request.send())
}

fn door(method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value, String> {
    call(&crate::signin::portfolio(), &token()?, method, path, body)
}

/// A listing id as the doors take it: letters, digits and dashes only.
fn plain(id: &str) -> Result<&str, String> {
    if !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(id)
    } else {
        Err("That is not a Marketplace listing.".into())
    }
}

/// The search address for the shop's cards.
pub fn listings_path(query: &str, page: u32) -> String {
    let q: String = query.trim().chars().take(80).collect();
    let mut encoded = String::new();
    for b in q.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(b as char),
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    format!("/api/public/market/listings?q={encoded}&power=image&page={page}&size=24")
}

/// The multipart a listing is published with: the workflow file, its samples, the price and words.
fn listing_form(workflow: &Value, samples: &[String], price_myr: f64, description: &str) -> Result<reqwest::blocking::multipart::Form, String> {
    use reqwest::blocking::multipart::{Form, Part};
    if samples.is_empty() || samples.len() > MAX_SAMPLES {
        return Err("Add one to three sample pictures made with this workflow.".into());
    }
    if !(price_myr.is_finite() && price_myr > 0.0) {
        return Err("Give the workflow a price.".into());
    }
    let file = serde_json::to_vec_pretty(workflow).map_err(|_| "The workflow could not be read.".to_string())?;
    let mut form = Form::new()
        .part("workflow", Part::bytes(file).file_name("workflow.json").mime_str("application/json").map_err(|_| "The workflow could not be sent.".to_string())?)
        .text("priceMyr", format!("{price_myr:.2}"))
        .text("description", description.trim().chars().take(300).collect::<String>());
    for sample in samples {
        let path = Path::new(sample);
        let kind = match path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            _ => return Err("Sample pictures must be PNG, JPEG or WebP.".into()),
        };
        let size = std::fs::metadata(path).map_err(|_| "A sample picture was not found.".to_string())?.len();
        if size > MAX_SAMPLE_BYTES {
            return Err("A sample picture is larger than 5 MB.".into());
        }
        let bytes = std::fs::read(path).map_err(|_| "A sample picture could not be read.".to_string())?;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "sample.png".into());
        form = form.part("samples", Part::bytes(bytes).file_name(name).mime_str(kind).map_err(|_| "A sample picture could not be sent.".to_string())?);
    }
    Ok(form)
}

fn send_listing(method: reqwest::Method, path: &str, form: reqwest::blocking::multipart::Form) -> Result<Value, String> {
    let base = crate::signin::portfolio();
    answer(client(Duration::from_secs(120))?.request(method, format!("{base}{path}")).bearer_auth(token()?).multipart(form).send())
}

fn open(url: &str) -> Result<(), String> {
    crate::commands::open_url(url.to_string())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| "The Marketplace call stopped before it finished.".to_string())?
}

#[tauri::command]
pub async fn market_shop_listings(query: String, page: u32, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || door(reqwest::Method::GET, &listings_path(&query, page), None)).await
}

#[tauri::command]
pub async fn market_shop_listing(id: String, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || door(reqwest::Method::GET, &format!("/api/public/market/listings/{}", plain(&id)?), None)).await
}

/// Buy: a Stripe Checkout page in the member's browser. Install follows once it is paid.
#[tauri::command]
pub async fn market_shop_checkout(id: String, store: State<'_, SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || {
        let answer = door(reqwest::Method::POST, &format!("/api/member/market/checkout/{}", plain(&id)?), None)?;
        open(answer.get("url").and_then(Value::as_str).ok_or("The Marketplace sent no checkout page.")?)
    })
    .await
}

#[tauri::command]
pub async fn market_shop_purchases(store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(|| door(reqwest::Method::GET, "/api/member/market/purchases", None)).await
}

/// Puts a bought workflow on the member's rail (the latest approved version). Returns its id.
#[tauri::command]
pub async fn market_shop_install(purchase: String, listing: String, store: State<'_, SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "studio")?;
    let folder = store.folder();
    blocking(move || {
        let workflow = door(reqwest::Method::GET, &format!("/api/member/market/purchases/{}/workflow", plain(&purchase)?), None)?;
        crate::workflows::install_bought(&folder, plain(&listing)?, workflow)
    })
    .await
}

/// A thumbs up (true), down (false), or the vote taken back (null). Buyers only, one each.
#[tauri::command]
pub async fn market_shop_vote(listing: String, up: Option<bool>, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || {
        let path = format!("/api/member/market/listings/{}/vote", plain(&listing)?);
        match up {
            Some(up) => door(reqwest::Method::PUT, &path, Some(serde_json::json!({ "up": up }))),
            None => door(reqwest::Method::DELETE, &path, None),
        }
    })
    .await
}

/// Connect Stripe: Stripe's own onboarding page in the browser.
#[tauri::command]
pub async fn market_shop_connect(store: State<'_, SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "studio")?;
    blocking(|| {
        let answer = door(reqwest::Method::POST, "/api/member/market/connect", None)?;
        open(answer.get("url").and_then(Value::as_str).ok_or("The Marketplace sent no Stripe page.")?)
    })
    .await
}

#[tauri::command]
pub async fn market_shop_mine(store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(|| door(reqwest::Method::GET, "/api/member/market/listings/mine", None)).await
}

/// Publish one of the member's own workflows (a new listing), or a new version of a listing.
#[tauri::command]
pub async fn market_shop_publish(workflow: String, listing: Option<String>, samples: Vec<String>, price_myr: f64, description: String, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    let folder = store.folder();
    blocking(move || {
        let (entry, file) = crate::workflows::find(&folder, &workflow)?;
        if entry.source != "yours" {
            return Err("Only your own workflows can be sold.".into());
        }
        let form = listing_form(&file, &samples, price_myr, &description)?;
        match listing {
            Some(id) => send_listing(reqwest::Method::PUT, &format!("/api/member/market/listings/{}", plain(&id)?), form),
            None => send_listing(reqwest::Method::POST, "/api/member/market/listings", form),
        }
    })
    .await
}

#[tauri::command]
pub async fn market_shop_withdraw(listing: String, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || door(reqwest::Method::DELETE, &format!("/api/member/market/listings/{}", plain(&listing)?), None)).await
}
