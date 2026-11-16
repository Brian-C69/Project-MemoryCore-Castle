//! The reader for the two services with no balance a key can read: OpenAI (what is left) and
//! BytePlus (Seedance's spend this month). The member signs in once in a small window of Castle's;
//! the WebView2 profile keeps that session the way a browser does, and a hidden window reads the
//! figure from the service's own billing page every 15 minutes.
//!
//! Each service brings a small script that runs only on its own host, finds the one figure in the
//! page and hands back only that figure, by navigating to `castle.invalid`, which `on_navigation`
//! always refuses and reads. A session, cookie or token never leaves the service's page, and these
//! windows have no access to Castle's core (no capability names them).
//!
//! The pages are the services' own and can change. When one does, the reader says "did not answer"
//! and the member's typed figure still works.

use crate::credits::{self, Figure, ReadState};
use crate::settings::SettingsStore;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};

/// A reader with no answer by then closes and says so.
const WAIT: Duration = Duration::from_secs(90);
/// How often the hidden readers look while the member is signed in.
const EVERY: Duration = Duration::from_secs(15 * 60);
/// A read asked by hand waits at least this long after the last one.
const AT_MOST: Duration = Duration::from_secs(60);
/// The host every answer is sent to, and never reached.
const ANSWER_HOST: &str = "castle.invalid";

/// What a navigation in a service's window means.
#[derive(Debug, Clone, PartialEq)]
pub enum Report {
    /// What is left (OpenAI).
    Balance(f64),
    /// This month's spend, the month, and the member's local time it was read (BytePlus).
    Month { usd: f64, month: String, at: i64 },
    SignedOut,
}

pub struct Vendor {
    /// "openai" or "byteplus".
    pub name: &'static str,
    pub page: &'static str,
    pub reader_label: &'static str,
    pub sign_in_label: &'static str,
    pub title: &'static str,
    pub listener: &'static str,
    pub parse: fn(&Url) -> Option<Report>,
}

// ---------- OpenAI ----------

const OPENAI_PAGE: &str = "https://platform.openai.com/settings/organization/billing/credit-grants";

/// Watches the billing page's own credit-grants answer and hands back `total_available` only.
const OPENAI_LISTENER: &str = r#"(function () {
  if (location.hostname !== 'platform.openai.com' || window.__castleListening) return;
  window.__castleListening = true;
  var sent = false;
  function report(usd) {
    if (sent || typeof usd !== 'number' || !isFinite(usd)) return;
    sent = true;
    location.href = 'https://castle.invalid/openai?usd=' + encodeURIComponent(String(usd));
  }
  var original = window.fetch;
  window.fetch = function (input) {
    var url = typeof input === 'string' ? input : (input && input.url) || '';
    var answer = original.apply(this, arguments);
    if (/\/v1\/dashboard\/billing\/credit_grants(\?|$)/.test(url)) {
      answer.then(function (res) {
        if (!res.ok) return;
        res.clone().json().then(function (body) {
          if (body && typeof body.total_available === 'number') report(body.total_available);
        }).catch(function () {});
      }).catch(function () {});
    }
    return answer;
  };
  // After signing in the page may land elsewhere: walk to Credit grants once or twice.
  setTimeout(function () {
    if (sent || /credit-grants|log-?in|auth/.test(location.pathname)) return;
    var walked = Number(sessionStorage.getItem('castle-walked') || '0');
    if (walked >= 2) return;
    sessionStorage.setItem('castle-walked', String(walked + 1));
    location.assign('/settings/organization/billing/credit-grants');
  }, 8000);
})();"#;

pub static OPENAI: Vendor = Vendor {
    name: "openai",
    page: OPENAI_PAGE,
    reader_label: "openai-reader",
    sign_in_label: "openai-signin",
    title: "Sign in to OpenAI - MemoryCore Castle",
    listener: OPENAI_LISTENER,
    parse: parse_openai,
};

/// `https://castle.invalid/openai?usd=18.7` -> the balance; OpenAI's log-in page -> signed out;
/// anything else (the page itself, a sign-in hop) -> nothing.
pub fn parse_openai(url: &Url) -> Option<Report> {
    match url.host_str()? {
        ANSWER_HOST if url.path() == "/openai" => url
            .query_pairs()
            .find(|(k, _)| k == "usd")
            .and_then(|(_, v)| v.parse::<f64>().ok())
            .filter(|usd| usd.is_finite() && *usd >= 0.0)
            .map(Report::Balance),
        "auth.openai.com" if url.path().contains("log-in") || url.path().contains("login") => Some(Report::SignedOut),
        _ => None,
    }
}

// ---------- BytePlus (Seedance) ----------

const BYTEPLUS_PAGE: &str = "https://console.byteplus.com/finance/overview";

/// Asks the console's own month cost from inside the page and hands back the sum, the month and when.
const BYTEPLUS_LISTENER: &str = r#"(function () {
  if (location.hostname !== 'console.byteplus.com' || window.__castleAsking) return;
  window.__castleAsking = true;
  function go(q) { location.href = 'https://castle.invalid/byteplus?' + q; }
  function pad(n) { return String(n).padStart(2, '0'); }
  function ask() {
    if (/login|signin/i.test(location.pathname)) return go('state=signed-out');
    var m = document.cookie.match(/(?:^|; )csrfToken=([^;]+)/);
    if (!m) return go('state=signed-out');
    var now = new Date();
    var month = now.getFullYear() + '-' + pad(now.getMonth() + 1);
    var off = -now.getTimezoneOffset();
    var local = new Date(now.getTime() + off * 60000).toISOString().slice(0, 19);
    var at = local + (off >= 0 ? '+' : '-') + pad(Math.floor(Math.abs(off) / 60)) + ':' + pad(Math.abs(off) % 60);
    fetch('/api/top/bill/ap-singapore-1/2020-01-01/ListCostAnalysisTrend', {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json', 'x-csrf-token': m[1] },
      body: JSON.stringify({ BeginTimeStr: month + '-01', EndTimeStr: month + '-01', TimeGranularity: 0, ClassifyDimension: 3, CostType: 2, BizType: 0, Limit: 10, Offset: 0 })
    }).then(function (r) { return r.json(); }).then(function (body) {
      var err = body && body.ResponseMetadata && body.ResponseMetadata.Error;
      if (err) {
        if (/csrf|login|auth|token|unauthor|session/i.test(err.Code || '')) go('state=signed-out');
        return;
      }
      if (!body || !body.Result) return;
      var list = body.Result.List || [];
      var usd = list.reduce(function (sum, row) { return sum + (typeof row.Amount === 'number' ? row.Amount : 0); }, 0);
      go('usd=' + encodeURIComponent(String(Math.round(usd * 100) / 100)) + '&month=' + month + '&at=' + encodeURIComponent(at));
    }).catch(function () {});
  }
  function later() { setTimeout(ask, 1500); }
  if (document.readyState === 'complete') later(); else window.addEventListener('load', later);
})();"#;

pub static BYTEPLUS: Vendor = Vendor {
    name: "byteplus",
    page: BYTEPLUS_PAGE,
    reader_label: "byteplus-reader",
    sign_in_label: "byteplus-signin",
    title: "Sign in to BytePlus - MemoryCore Castle",
    listener: BYTEPLUS_LISTENER,
    parse: parse_byteplus,
};

fn is_month(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 7 && b[4] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || c.is_ascii_digit())
}

/// `https://castle.invalid/byteplus?usd=2.45&month=2026-11&at=2026-11-16T08:10:00+08:00` -> the
/// month; `?state=signed-out` or a BytePlus sign-in page -> signed out; anything else -> nothing.
pub fn parse_byteplus(url: &Url) -> Option<Report> {
    let host = url.host_str()?;
    if host == ANSWER_HOST {
        if url.path() != "/byteplus" {
            return None;
        }
        let get = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());
        if get("state").as_deref() == Some("signed-out") {
            return Some(Report::SignedOut);
        }
        let usd = get("usd")?.parse::<f64>().ok().filter(|u| u.is_finite() && *u >= 0.0)?;
        let month = get("month").filter(|m| is_month(m))?;
        let at = get("at").filter(|a| a.starts_with(&month)).and_then(|a| crate::quota::iso_ms(&a))?;
        return Some(Report::Month { usd, month, at });
    }
    let path = url.path().to_ascii_lowercase();
    if (host == "byteplus.com" || host.ends_with(".byteplus.com")) && (path.contains("login") || path.contains("signin")) {
        return Some(Report::SignedOut);
    }
    None
}

// ---------- the windows ----------

pub fn vendor(name: &str) -> Option<&'static Vendor> {
    match name {
        "openai" => Some(&OPENAI),
        "byteplus" | "seedance" => Some(&BYTEPLUS),
        _ => None,
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Keeps what a read found and tells the widgets.
fn keep(app: &AppHandle, vendor: &Vendor, report: Option<Report>) {
    let store = app.state::<SettingsStore>();
    let now = now_ms();
    let state = |s: &str| Some(ReadState { state: s.into(), at: now });
    let openai = vendor.name == "openai";
    let _ = credits::save_with(&store, |c| match (report, openai) {
        (Some(Report::Balance(usd)), true) => {
            c.openai = Some(Figure { usd: (usd * 100.0).round() / 100.0, at: now, source: "openai".into(), month: None });
            c.openai_state = state("signed-in");
        }
        (Some(Report::Month { usd, month, at }), false) => {
            c.seedance = Some(Figure { usd: (usd * 100.0).round() / 100.0, at, source: "byteplus".into(), month: Some(month) });
            c.byteplus_state = state("signed-in");
        }
        (Some(Report::SignedOut), true) => c.openai_state = state("signed-out"),
        (Some(Report::SignedOut), false) => c.byteplus_state = state("signed-out"),
        (_, true) => c.openai_state = state("no-answer"),
        (_, false) => c.byteplus_state = state("no-answer"),
    });
    credits::changed(app);
}

/// True once the member signed in, until the service signs them out (or they stop the reading).
fn signed_in(store: &SettingsStore, vendor: &Vendor) -> bool {
    let saved = store.get().credits;
    let state = if vendor.name == "openai" { saved.openai_state } else { saved.byteplus_state };
    state.map(|s| s.state != "signed-out").unwrap_or(false)
}

static READING: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
static LAST_READ: Mutex<Option<HashMap<&'static str, Instant>>> = Mutex::new(None);

fn begin(name: &'static str) -> bool {
    let mut reading = READING.lock().unwrap();
    if !reading.get_or_insert_with(HashSet::new).insert(name) {
        return false;
    }
    LAST_READ.lock().unwrap().get_or_insert_with(HashMap::new).insert(name, Instant::now());
    true
}

fn end(name: &'static str) {
    if let Some(set) = READING.lock().unwrap().as_mut() {
        set.remove(name);
    }
}

fn is_answer(url: &Url) -> bool {
    url.host_str() == Some(ANSWER_HOST)
}

/// One look through the service's hidden reader. False when a look is already running.
pub fn read_now(app: &AppHandle, vendor: &'static Vendor) -> bool {
    if !begin(vendor.name) {
        return false;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<Report>();
        let parse = vendor.parse;
        let built = vendor.page.parse().map_err(|_| ()).and_then(|page| {
            WebviewWindowBuilder::new(&app, vendor.reader_label, WebviewUrl::External(page))
                .title(vendor.title)
                .visible(false)
                .skip_taskbar(true)
                .initialization_script(vendor.listener)
                .on_navigation(move |url| {
                    if let Some(report) = parse(url) {
                        let _ = tx.send(report);
                        return false;
                    }
                    !is_answer(url)
                })
                .build()
                .map_err(|_| ())
        });
        match built {
            Ok(window) => {
                let found = rx.recv_timeout(WAIT).ok();
                let _ = window.destroy();
                keep(&app, vendor, found);
            }
            Err(_) => keep(&app, vendor, None),
        }
        end(vendor.name);
    });
    true
}

/// Opens the service's billing page in a window the member can see, to sign in once. It closes by
/// itself when the figure arrives; closed by the member, nothing changes.
pub fn sign_in(app: &AppHandle, vendor: &'static Vendor) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(vendor.sign_in_label) {
        let _ = window.set_focus();
        return Ok(());
    }
    let page = vendor.page.parse().map_err(|_| "The sign-in page could not be opened.".to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<Report>();
        let parse = vendor.parse;
        let built = WebviewWindowBuilder::new(&handle, vendor.sign_in_label, WebviewUrl::External(page))
            .title(vendor.title)
            .inner_size(520.0, 780.0)
            .center()
            .initialization_script(vendor.listener)
            .on_navigation(move |url| {
                match parse(url) {
                    // a log-in page here is simply the way in
                    Some(Report::SignedOut) | None => {}
                    Some(report) => {
                        let _ = tx.send(report);
                    }
                }
                !is_answer(url)
            })
            .build();
        if let Ok(window) = built {
            // ends with the figure, or when the member closes the window (the sender goes with it)
            if let Ok(report) = rx.recv() {
                let _ = window.destroy();
                keep(&handle, vendor, Some(report));
            }
        }
    });
    Ok(())
}

/// Every 15 minutes, each service the member is signed in to. Never before a first sign-in, never
/// after the service signed them out: the widget's Sign in starts it again.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(20));
        loop {
            for vendor in [&OPENAI, &BYTEPLUS] {
                if signed_in(&app.state::<SettingsStore>(), vendor) {
                    read_now(&app, vendor);
                }
            }
            std::thread::sleep(EVERY);
        }
    });
}

// ---------- commands ----------

/// The widget's Sign in: the service's page in a small window.
#[tauri::command]
pub fn credits_sign_in(app: AppHandle, vendor: String) -> Result<(), String> {
    let vendor = self::vendor(&vendor).ok_or("Only OpenAI and BytePlus sign in.")?;
    sign_in(&app, vendor)
}

/// A read asked by hand (clicking the figure): at most one a minute per service. False when it waits.
#[tauri::command]
pub fn credits_read_now(app: AppHandle, vendor: String) -> Result<bool, String> {
    let vendor = self::vendor(&vendor).ok_or("Only OpenAI and BytePlus are read from their page.")?;
    let recent = LAST_READ.lock().unwrap().as_ref().and_then(|m| m.get(vendor.name).map(|t| t.elapsed() < AT_MOST)).unwrap_or(false);
    Ok(!recent && read_now(&app, vendor))
}

/// Stops reading a service's page and forgets its figure. The session itself ends on the service's
/// own page (its sign-out, in the Sign in window).
#[tauri::command]
pub fn credits_stop_reading(app: AppHandle, store: tauri::State<SettingsStore>, vendor: String) -> Result<(), String> {
    let vendor = self::vendor(&vendor).ok_or("Only OpenAI and BytePlus are read from their page.")?;
    let stopped = Some(ReadState { state: "signed-out".into(), at: now_ms() });
    credits::save_with(&store, |c| {
        if vendor.name == "openai" {
            c.openai = None;
            c.openai_state = stopped;
        } else {
            c.seedance = None;
            c.byteplus_state = stopped;
        }
    })?;
    credits::changed(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        text.parse().unwrap()
    }

    #[test]
    fn openai_hands_back_the_balance_only() {
        assert_eq!(parse_openai(&url("https://castle.invalid/openai?usd=18.70366073")), Some(Report::Balance(18.70366073)));
        assert_eq!(parse_openai(&url("https://castle.invalid/openai?usd=0")), Some(Report::Balance(0.0)));
        assert_eq!(parse_openai(&url("https://castle.invalid/openai?usd=-1")), None);
        assert_eq!(parse_openai(&url("https://castle.invalid/openai?usd=NaN")), None);
        assert_eq!(parse_openai(&url("https://castle.invalid/byteplus?usd=2.45")), None);
        assert_eq!(parse_openai(&url("https://auth.openai.com/log-in")), Some(Report::SignedOut));
        assert_eq!(parse_openai(&url("https://auth.openai.com/authorize?client_id=x")), None);
        assert_eq!(parse_openai(&url(OPENAI_PAGE)), None);
    }

    #[test]
    fn byteplus_hands_back_the_month_only() {
        let month = parse_byteplus(&url("https://castle.invalid/byteplus?usd=2.45&month=2026-11&at=2026-11-16T08%3A10%3A00%2B08%3A00"));
        assert_eq!(month, Some(Report::Month { usd: 2.45, month: "2026-11".into(), at: crate::quota::iso_ms("2026-11-16T08:10:00+08:00").unwrap() }));
        assert_eq!(parse_byteplus(&url("https://castle.invalid/byteplus?state=signed-out")), Some(Report::SignedOut));
        assert_eq!(parse_byteplus(&url("https://castle.invalid/byteplus?usd=2&month=2026-1&at=2026-11-16T08:10:00Z")), None);
        assert_eq!(parse_byteplus(&url("https://castle.invalid/byteplus?usd=2&month=2026-11&at=2026-10-31T08:10:00Z")), None);
        assert_eq!(parse_byteplus(&url("https://console.byteplus.com/login")), Some(Report::SignedOut));
        assert_eq!(parse_byteplus(&url(BYTEPLUS_PAGE)), None);
    }

    #[test]
    fn the_scripts_stay_on_their_own_host_and_send_no_secret() {
        assert!(OPENAI_LISTENER.contains("location.hostname !== 'platform.openai.com'"));
        assert!(BYTEPLUS_LISTENER.contains("location.hostname !== 'console.byteplus.com'"));
        for script in [OPENAI_LISTENER, BYTEPLUS_LISTENER] {
            assert!(script.contains("https://castle.invalid/"));
            assert!(!script.to_lowercase().contains("authorization"));
            // the csrf token is sent to the console itself, never in the answer
            assert!(!script.contains("castle.invalid/byteplus?' + m"));
        }
    }

    #[test]
    fn one_look_at_a_time_per_service() {
        assert!(begin("test-a"));
        assert!(!begin("test-a"));
        assert!(begin("test-b"));
        end("test-a");
        assert!(begin("test-a"));
        end("test-a");
        end("test-b");
        assert!(is_answer(&url("https://castle.invalid/x")));
        assert!(!is_answer(&url("https://platform.openai.com/")));
    }
}
