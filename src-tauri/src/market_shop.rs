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
        form = form.part("samples", picture_part(sample, "sample")?);
    }
    Ok(form)
}

/// A picture to send (a sample, the keeper's picture, the backdrop): PNG, JPEG or WebP, 5 MB at most.
fn picture_part(file: &str, what: &str) -> Result<reqwest::blocking::multipart::Part, String> {
    let path = Path::new(file);
    let kind = match path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => return Err(format!("The {what} picture must be PNG, JPEG or WebP.")),
    };
    let size = std::fs::metadata(path).map_err(|_| format!("The {what} picture was not found."))?.len();
    if size > MAX_SAMPLE_BYTES {
        return Err(format!("The {what} picture is larger than 5 MB."));
    }
    let bytes = std::fs::read(path).map_err(|_| format!("The {what} picture could not be read."))?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| format!("{what}.png"));
    reqwest::blocking::multipart::Part::bytes(bytes).file_name(name).mime_str(kind).map_err(|_| format!("The {what} picture could not be sent."))
}

/// What a member's shop is saved with (the Portfolio's E.134 member door).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopFields {
    pub name: String,
    pub keeper_name: String,
    pub greeting: String,
    pub keeper_card: String,
    pub open: bool,
}

/// Checks a shop before it is sent; the Portfolio checks again.
pub fn shop_problem(f: &ShopFields) -> Option<&'static str> {
    let count = |s: &str| s.trim().chars().count();
    if count(&f.name) == 0 || count(&f.name) > 60 {
        return Some("Give the shop a name (at most 60 letters).");
    }
    if count(&f.keeper_name) == 0 || count(&f.keeper_name) > 40 {
        return Some("Give the keeper a name (at most 40 letters).");
    }
    if count(&f.greeting) > 160 {
        return Some("Keep the greeting under 160 letters.");
    }
    if count(&f.keeper_card) > 1200 {
        return Some("Keep the keeper card under 1200 letters.");
    }
    None
}

fn shop_form(f: &ShopFields, keeper_picture: Option<&str>, backdrop: Option<&str>) -> Result<reqwest::blocking::multipart::Form, String> {
    if let Some(problem) = shop_problem(f) {
        return Err(problem.into());
    }
    let mut form = reqwest::blocking::multipart::Form::new()
        .text("name", f.name.trim().to_string())
        .text("keeperName", f.keeper_name.trim().to_string())
        .text("greeting", f.greeting.trim().to_string())
        .text("keeperCard", f.keeper_card.trim().to_string())
        .text("open", f.open.to_string());
    if let Some(file) = keeper_picture.filter(|f| !f.is_empty()) {
        form = form.part("keeperPicture", picture_part(file, "keeper")?);
    }
    if let Some(file) = backdrop.filter(|f| !f.is_empty()) {
        form = form.part("backdrop", picture_part(file, "backdrop")?);
    }
    Ok(form)
}

/// A door whose 404 only means "not there yet" (the shops before E.134, a member with no shop).
pub fn call_or_none(base: &str, token: &str, path: &str) -> Result<Option<Value>, String> {
    let response = client(Duration::from_secs(30))?.get(format!("{base}{path}")).bearer_auth(token).send().map_err(|_| "The Portfolio could not be reached. Check the internet connection.".to_string())?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    answer(Ok(response)).map(Some)
}

/// The shops' address, searched like the items'.
pub fn shops_path(query: &str, page: u32) -> String {
    listings_path(query, page).replace("/api/public/market/listings?", "/api/public/market/shops?").replace("&power=image", "")
}

/// A shop's slug as the doors take it.
fn slug(s: &str) -> Result<&str, String> {
    if !s.is_empty() && s.len() <= 80 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Ok(s)
    } else {
        Err("That is not a shop in the town.".into())
    }
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

/// The town's shops; `{ "items": [], "missing": true }` until the Portfolio's shops door answers.
#[tauri::command]
pub async fn market_shop_shops(query: String, page: u32, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || {
        let found = call_or_none(&crate::signin::portfolio(), &token()?, &shops_path(&query, page))?;
        Ok(found.unwrap_or_else(|| serde_json::json!({ "items": [], "missing": true })))
    })
    .await
}

#[tauri::command]
pub async fn market_shop_shop(slug_name: String, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || door(reqwest::Method::GET, &format!("/api/public/market/shops/{}", slug(&slug_name)?), None)).await
}

/// The member's own shop, or null when they have none yet.
#[tauri::command]
pub async fn market_shop_my_shop(store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(|| Ok(call_or_none(&crate::signin::portfolio(), &token()?, "/api/member/market/shop")?.unwrap_or(Value::Null))).await
}

#[tauri::command]
pub async fn market_shop_my_shop_save(fields: ShopFields, keeper_picture: Option<String>, backdrop: Option<String>, store: State<'_, SettingsStore>) -> Result<Value, String> {
    crate::rooms::need(&store, "studio")?;
    blocking(move || {
        let form = shop_form(&fields, keeper_picture.as_deref(), backdrop.as_deref())?;
        send_listing(reqwest::Method::PUT, "/api/member/market/shop", form)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn serve_once(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buf = [0; 8192];
                let n = stream.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&bytes);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").and_then(|s| s.trim().parse::<usize>().ok())).unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            write!(stream, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8_lossy(&bytes).to_string()
        });
        (base, handle)
    }

    #[test]
    fn a_refusal_reads_as_plain_words() {
        assert_eq!(words_for(401, ""), "Your sign-in has run out. Sign in again in CHAT.");
        assert_eq!(words_for(403, r#"{"error":"Selling opens soon."}"#), "Selling opens soon.");
        assert_eq!(words_for(404, ""), "The Marketplace is not open yet.");
        assert_eq!(words_for(415, "<html>"), "Sample pictures must be PNG, JPEG or WebP.");
        assert_eq!(words_for(500, "oops"), "The Marketplace could not answer just now.");
    }

    #[test]
    fn a_door_is_called_with_the_members_sign_in_and_its_answer_comes_back() {
        let (base, handle) = serve_once(200, r#"{"items":[{"id":"7","name":"Two cats"}],"page":0,"total":1}"#);
        let answer = call(&base, "member-token", reqwest::Method::GET, &listings_path("two cats & tea", 0), None).unwrap();
        let request = handle.join().unwrap();
        assert_eq!(answer["items"][0]["name"], "Two cats");
        assert!(request.starts_with("GET /api/public/market/listings?q=two+cats+%26+tea&power=image&page=0&size=24 "));
        assert!(request.to_lowercase().contains("authorization: bearer member-token"));
        assert!(request.to_lowercase().contains("user-agent: memorycorecastle/"));
    }

    #[test]
    fn a_refused_call_never_carries_the_sign_in_back() {
        let (base, handle) = serve_once(403, r#"{"error":"Connect Stripe first."}"#);
        let said = call(&base, "member-token", reqwest::Method::POST, "/api/member/market/listings", Some(json!({}))).unwrap_err();
        handle.join().unwrap();
        assert_eq!(said, "Connect Stripe first.");
        assert!(!said.contains("member-token"));
    }

    #[test]
    fn only_plain_listing_ids_reach_a_door() {
        assert!(plain("42").is_ok() && plain("abc-123").is_ok());
        assert!(plain("../admin").is_err() && plain("").is_err() && plain("a?b=c").is_err());
    }

    #[test]
    fn a_missing_shops_door_means_not_there_yet() {
        let (base, handle) = serve_once(404, "");
        assert_eq!(call_or_none(&base, "member-token", &shops_path("mira", 0)).unwrap(), None);
        let request = handle.join().unwrap();
        assert!(request.starts_with("GET /api/public/market/shops?q=mira&page=0&size=24 "), "{request}");
        assert!(request.to_lowercase().contains("authorization: bearer member-token"));
        let (base, handle) = serve_once(200, r#"{"items":[{"slug":"miras-atelier"}]}"#);
        assert_eq!(call_or_none(&base, "member-token", "/api/member/market/shop").unwrap().unwrap()["items"][0]["slug"], "miras-atelier");
        handle.join().unwrap();
        let (base, handle) = serve_once(403, r#"{"error":"Shops open with selling."}"#);
        assert_eq!(call_or_none(&base, "member-token", "/api/member/market/shop").unwrap_err(), "Shops open with selling.");
        handle.join().unwrap();
    }

    #[test]
    fn a_shop_is_checked_before_it_is_sent() {
        let good = ShopFields { name: "Mira's Atelier".into(), keeper_name: "Mira".into(), greeting: "Welcome!".into(), keeper_card: "I paint postcards.".into(), open: true };
        assert_eq!(shop_problem(&good), None);
        assert!(shop_problem(&ShopFields { name: " ".into(), ..good.clone() }).unwrap().contains("shop a name"));
        assert!(shop_problem(&ShopFields { keeper_name: String::new(), ..good.clone() }).unwrap().contains("keeper a name"));
        assert!(shop_problem(&ShopFields { greeting: "x".repeat(161), ..good.clone() }).unwrap().contains("greeting"));
        assert!(shop_problem(&ShopFields { keeper_card: "x".repeat(1201), ..good.clone() }).unwrap().contains("keeper card"));
        assert!(shop_form(&good, Some("C:/nowhere/keeper.gif"), None).unwrap_err().contains("PNG, JPEG or WebP"));
        assert!(shop_form(&good, None, None).is_ok());
        assert!(slug("miras-atelier").is_ok() && slug("../admin").is_err() && slug("Mira").is_err());
    }

    #[test]
    fn a_listing_needs_samples_and_a_price() {
        let root = std::env::temp_dir().join(format!("castle-shop-form-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let png = root.join("sample.png");
        std::fs::write(&png, b"png").unwrap();
        let gif = root.join("sample.gif");
        std::fs::write(&gif, b"gif").unwrap();
        let w = json!({ "schema": 1 });
        let one = vec![png.display().to_string()];
        assert!(listing_form(&w, &[], 5.0, "").unwrap_err().contains("one to three"));
        assert!(listing_form(&w, &vec![one[0].clone(); 4], 5.0, "").unwrap_err().contains("one to three"));
        assert!(listing_form(&w, &one, 0.0, "").unwrap_err().contains("price"));
        assert!(listing_form(&w, &[gif.display().to_string()], 5.0, "").unwrap_err().contains("PNG, JPEG or WebP"));
        assert!(listing_form(&w, &one, 9.9, "Two cats in the garden").is_ok());
    }
}
