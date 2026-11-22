//! Phase 5: the web-request trigger. A tiny listener on 127.0.0.1 only (never the network, never a
//! firewall change) that runs only while a switched-on job uses it. A request is `POST
//! /hook/<job>/<secret>`; the secret is random, kept in Windows Credential Manager and shown once
//! when the link is made. The body is DATA (`payload.text`), never a command.
use super::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const PORT: u16 = 51861;
/// The design allowed 64 KB; 16 KB keeps a body (twice in the payload) inside a command's 64 KiB limit.
const MAX_BODY: usize = 16 * 1024;
const MAX_HEAD: usize = 8 * 1024;
const WORKERS: usize = 4;
const MINUTE: Duration = Duration::from_secs(60);
const PER_JOB: usize = 10;
const ALL: usize = 60;
const BAD: usize = 20;
pub const MAX_JOBS: u64 = 8;

static PROBLEM: Mutex<Option<String>> = Mutex::new(None);
pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}

/// A web-request trigger has no settings of its own: the link is the setting.
pub fn spec(v: &Value) -> Result<(), String> {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        Ok(())
    } else {
        Err("A web request trigger has no settings.".into())
    }
}

pub fn url(id: u64, secret: &str) -> String {
    format!("http://127.0.0.1:{PORT}/hook/{id}/{secret}")
}

/// 192 random bits in hex. There is no weaker fallback: no randomness, no link.
pub fn new_secret() -> Result<String, String> {
    let mut b = [0u8; 24];
    getrandom::fill(&mut b).map_err(|_| "A secret could not be made.".to_string())?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Compares in the same time whatever the first difference is.
pub fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= (*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0)) as usize;
    }
    diff == 0
}

pub fn is_local(ip: IpAddr) -> bool {
    ip.to_canonical().is_loopback()
}

#[derive(Debug, PartialEq)]
pub enum Reject {
    Bad,
    Forbidden,
    NotFound,
    Method,
    TooLarge,
    Busy,
}
impl Reject {
    fn code(&self) -> (u16, &'static str) {
        match self {
            Reject::Bad => (400, "Bad Request"),
            Reject::Forbidden => (403, "Forbidden"),
            Reject::NotFound => (404, "Not Found"),
            Reject::Method => (405, "Method Not Allowed"),
            Reject::TooLarge => (413, "Payload Too Large"),
            Reject::Busy => (429, "Too Many Requests"),
        }
    }
}

#[derive(Debug)]
pub struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

fn parse_head(head: &str) -> Result<(String, String, Vec<(String, String)>), Reject> {
    let mut lines = head.split("\r\n");
    let first = lines.next().ok_or(Reject::Bad)?;
    let mut parts = first.split(' ');
    let (method, path, version) = (parts.next(), parts.next(), parts.next());
    let (Some(method), Some(path), Some(version), None) = (method, path, version, parts.next()) else {
        return Err(Reject::Bad);
    };
    if !version.starts_with("HTTP/1.") {
        return Err(Reject::Bad);
    }
    let mut headers = Vec::new();
    for line in lines {
        let (k, v) = line.split_once(':').ok_or(Reject::Bad)?;
        if headers.len() >= 40 || k.is_empty() || k.contains(' ') {
            return Err(Reject::Bad);
        }
        headers.push((k.to_ascii_lowercase(), v.trim().to_string()));
    }
    Ok((method.to_string(), path.to_string(), headers))
}

fn read_request(stream: &mut TcpStream) -> Result<Request, Reject> {
    let deadline = Instant::now() + Duration::from_secs(4);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 2048];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEAD || Instant::now() > deadline {
            return Err(Reject::Bad);
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(Reject::Bad),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = std::str::from_utf8(&buf[..end]).map_err(|_| Reject::Bad)?;
    let (method, path, headers) = parse_head(head)?;
    if headers.iter().any(|(k, _)| k == "transfer-encoding") {
        return Err(Reject::Bad);
    }
    let lengths: Vec<&str> = headers.iter().filter(|(k, _)| k == "content-length").map(|(_, v)| v.as_str()).collect();
    if lengths.len() > 1 {
        return Err(Reject::Bad);
    }
    let length: usize = match lengths.first() {
        Some(v) => v.parse().map_err(|_| Reject::Bad)?,
        None => 0,
    };
    if length > MAX_BODY {
        return Err(Reject::TooLarge);
    }
    let start = end + 4;
    while buf.len() < start + length {
        if Instant::now() > deadline {
            return Err(Reject::Bad);
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(Reject::Bad),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    Ok(Request { method, path, headers, body: buf[start..start + length].to_vec() })
}

/// The job and the secret a request names, after the checks that need no secret.
fn route(req: &Request, port: u16) -> Result<(u64, String), Reject> {
    if req.method != "POST" {
        return Err(Reject::Method);
    }
    // a web page in a browser can reach 127.0.0.1 too: it sends an Origin, and a rebound name sends
    // another Host; neither is the member's own program
    let host = req.header("host").unwrap_or("").to_ascii_lowercase();
    if req.header("origin").is_some() || (host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}")) {
        return Err(Reject::Forbidden);
    }
    let parts: Vec<&str> = req.path.split('/').collect();
    let [_, "hook", id, secret] = parts[..] else { return Err(Reject::NotFound) };
    if id.is_empty() || id.len() > 10 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Reject::NotFound);
    }
    if !(20..=128).contains(&secret.len()) || !secret.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(Reject::NotFound);
    }
    Ok((id.parse().map_err(|_| Reject::NotFound)?, secret.to_string()))
}

#[derive(Default)]
pub struct Limits {
    all: Vec<Instant>,
    bad: Vec<Instant>,
    jobs: HashMap<u64, Vec<Instant>>,
}
impl Limits {
    fn trim(list: &mut Vec<Instant>, now: Instant) {
        list.retain(|t| now.duration_since(*t) < MINUTE);
    }
    /// Counts the request; true when it may go on.
    fn admit(&mut self, now: Instant) -> bool {
        Self::trim(&mut self.all, now);
        Self::trim(&mut self.bad, now);
        if self.all.len() >= ALL || self.bad.len() >= BAD {
            return false;
        }
        self.all.push(now);
        true
    }
    fn wrong(&mut self, now: Instant) {
        self.bad.push(now);
    }
    fn job(&mut self, id: u64, now: Instant) -> bool {
        let list = self.jobs.entry(id).or_default();
        Self::trim(list, now);
        if list.len() >= PER_JOB {
            return false;
        }
        list.push(now);
        true
    }
}

pub struct Deps<'a> {
    pub secret: &'a dyn Fn(u64) -> Option<String>,
    pub launch: &'a dyn Fn(u64, Value) -> Result<u64, String>,
    pub on: &'a dyn Fn() -> bool,
}

fn reply(stream: &mut TcpStream, code: u16, text: &str, body: &Value) {
    let body = body.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 {code} {text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.flush();
    // close politely: a client still sending would otherwise be reset before it reads the answer
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let mut sink = [0u8; 4096];
    for _ in 0..16 {
        if !matches!(stream.read(&mut sink), Ok(n) if n > 0) {
            break;
        }
    }
}
fn refuse(stream: &mut TcpStream, why: Reject) {
    let (code, text) = why.code();
    reply(stream, code, text, &json!({"error": text.to_lowercase()}));
}

pub fn serve(mut stream: TcpStream, port: u16, deps: &Deps, limits: &Mutex<Limits>) {
    // only this computer, whatever the listener is bound to
    if !stream.peer_addr().is_ok_and(|a| is_local(a.ip())) {
        return;
    }
    if !limits.lock().unwrap().admit(Instant::now()) {
        return refuse(&mut stream, Reject::Busy);
    }
    let request = match read_request(&mut stream) {
        Ok(r) => r,
        Err(why) => return refuse(&mut stream, why),
    };
    let (id, given) = match route(&request, port) {
        Ok(found) => found,
        Err(why) => return refuse(&mut stream, why),
    };
    // a wrong secret and an unknown job look the same, and are counted
    if !(deps.secret)(id).is_some_and(|real| same(&real, &given)) {
        limits.lock().unwrap().wrong(Instant::now());
        return refuse(&mut stream, Reject::NotFound);
    }
    if !(deps.on)() {
        return reply(&mut stream, 503, "Service Unavailable", &json!({"error":"Automations are off or paused."}));
    }
    if !limits.lock().unwrap().job(id, Instant::now()) {
        return refuse(&mut stream, Reject::Busy);
    }
    let text = String::from_utf8_lossy(&request.body).into_owned();
    let payload = json!({"cause":"webhook","text":text,"bytes":request.body.len(),"time":schedule::stamp(chrono::Local::now())});
    match (deps.launch)(id, payload) {
        Ok(run) => reply(&mut stream, 202, "Accepted", &json!({"ok":true,"run":run})),
        Err(error) => reply(&mut stream, 409, "Conflict", &json!({"error":error})),
    }
}

/// Whether any switched-on web-request job exists (the listener runs only then).
fn wanted(app: &AppHandle) -> bool {
    let store = app.state::<SettingsStore>();
    if !allowed(&store.get()) {
        return false;
    }
    connection(&store)
        .and_then(|mut c| store::list(&mut c))
        .is_ok_and(|jobs| jobs.iter().any(|j| j["state"] == "active" && j["enabled"] == 1 && j["triggerKind"] == "webhook"))
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let limits = Arc::new(Mutex::new(Limits::default()));
        let active = Arc::new(AtomicUsize::new(0));
        let mut listener: Option<TcpListener> = None;
        let mut checked = Instant::now() - Duration::from_secs(10);
        loop {
            if checked.elapsed() >= Duration::from_secs(3) {
                checked = Instant::now();
                let want = wanted(&app);
                if want && listener.is_none() {
                    match TcpListener::bind(("127.0.0.1", PORT)) {
                        Ok(l) => {
                            let _ = l.set_nonblocking(true);
                            listener = Some(l);
                            *PROBLEM.lock().unwrap() = None;
                        }
                        Err(_) => {
                            *PROBLEM.lock().unwrap() = Some(format!("The web request listener could not start: port {PORT} is in use."));
                        }
                    }
                } else if !want && listener.is_some() {
                    listener = None;
                    *PROBLEM.lock().unwrap() = None;
                }
            }
            let Some(l) = &listener else {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            };
            match l.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    if active.load(Ordering::Acquire) >= WORKERS {
                        let mut stream = stream;
                        refuse(&mut stream, Reject::Busy);
                        continue;
                    }
                    active.fetch_add(1, Ordering::AcqRel);
                    let (app, limits, active) = (app.clone(), limits.clone(), active.clone());
                    std::thread::spawn(move || {
                        let secret = |id: u64| crate::key::hook_secret_load(id);
                        let on = || allowed(&app.state::<SettingsStore>().get());
                        let launch = |id: u64, payload: Value| {
                            let store = app.state::<SettingsStore>();
                            let job = store::get(&mut connection(&store)?, id)?;
                            if job["triggerKind"] != "webhook" || job["enabled"] != 1 || job["state"] != "active" {
                                return Err("This job is not switched on.".to_string());
                            }
                            runner::launch_payload(&app, id, "webhook", payload, Some(&job))
                        };
                        serve(stream, PORT, &Deps { secret: &secret, launch: &launch, on: &on }, &limits);
                        active.fetch_sub(1, Ordering::AcqRel);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    });
}

/// Makes (or replaces) the job's link and returns it. This is the only time the secret is shown.
#[tauri::command]
pub async fn automation_hook_reset(app: AppHandle, id: u64) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.state::<SettingsStore>();
        let job = store::get(&mut connection(&store)?, id)?;
        if job["triggerKind"] != "webhook" || job["state"] != "active" {
            return Err("Save the job with the web request trigger first.".into());
        }
        let secret = new_secret()?;
        crate::key::hook_secret_save(id, &secret)?;
        Ok(url(id, &secret))
    })
    .await
    .map_err(|_| "The link could not be made.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    const SECRET: &str = "0123456789abcdef0123456789abcdef0123456789abcdef";

    /// One real request over a real loopback socket into `serve`, with fake secrets and launching.
    fn exchange(raw: &str, on: bool) -> (String, Vec<(u64, Value)>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let launched = Mutex::new(Vec::new());
        let limits = Mutex::new(Limits::default());
        let raw = raw.replace("{PORT}", &port.to_string()).replace("{SECRET}", SECRET);
        let response = std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                let (stream, _) = listener.accept().unwrap();
                let secret = |id: u64| (id == 7).then(|| SECRET.to_string());
                let launch = |id: u64, payload: Value| {
                    launched.lock().unwrap().push((id, payload));
                    Ok(99)
                };
                let on = || on;
                serve(stream, port, &Deps { secret: &secret, launch: &launch, on: &on }, &limits);
            });
            let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
            client.write_all(raw.as_bytes()).unwrap();
            let mut out = String::new();
            let _ = client.read_to_string(&mut out);
            server.join().unwrap();
            out
        });
        (response, launched.into_inner().unwrap())
    }
    fn post(path: &str, extra: &str, body: &str) -> String {
        format!("POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{{PORT}}\r\n{extra}Content-Length: {}\r\n\r\n{body}", body.len())
    }

    #[test]
    fn the_right_link_starts_the_job_and_the_body_is_only_data() {
        let hostile = "; calc & {{payload.text}} <script>";
        let (out, launched) = exchange(&post("/hook/7/{SECRET}", "", hostile), true);
        assert!(out.starts_with("HTTP/1.1 202"), "{out}");
        assert!(out.contains("\"run\":99"));
        assert_eq!(launched.len(), 1);
        assert_eq!(launched[0].0, 7);
        assert_eq!(launched[0].1["cause"], "webhook");
        assert_eq!(launched[0].1["text"], hostile);
    }

    #[test]
    fn a_wrong_secret_an_unknown_job_and_odd_paths_all_look_the_same_and_start_nothing() {
        for path in [
            "/hook/7/0123456789abcdef0123456789abcdef0123456789abcdeX",
            "/hook/8/{SECRET}",
            "/hook/7/{SECRET}?x=1",
            "/hook/7/../x/{SECRET}",
            "/hook/7/short",
            "/hook//{SECRET}",
            "/other/7/{SECRET}",
            "/hook/7/{SECRET}/more",
            "/hook/99999999999/{SECRET}",
        ] {
            let (out, launched) = exchange(&post(path, "", "x"), true);
            assert!(out.starts_with("HTTP/1.1 404"), "{path}: {out}");
            assert!(launched.is_empty(), "{path}");
        }
    }

    #[test]
    fn only_post_from_a_program_with_the_right_host_is_accepted() {
        let (out, l) = exchange("GET /hook/7/{SECRET} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n\r\n", true);
        assert!(out.starts_with("HTTP/1.1 405") && l.is_empty());
        let (out, l) = exchange(&post("/hook/7/{SECRET}", "Origin: https://evil.example\r\n", "x"), true);
        assert!(out.starts_with("HTTP/1.1 403") && l.is_empty());
        let (out, l) = exchange("POST /hook/7/{SECRET} HTTP/1.1\r\nHost: evil.example\r\nContent-Length: 1\r\n\r\nx", true);
        assert!(out.starts_with("HTTP/1.1 403") && l.is_empty());
        let (out, l) = exchange("POST /hook/7/{SECRET} HTTP/1.1\r\nContent-Length: 1\r\n\r\nx", true);
        assert!(out.starts_with("HTTP/1.1 403") && l.is_empty());
        let (out, l) = exchange(&post("/hook/7/{SECRET}", "Transfer-Encoding: chunked\r\n", ""), true);
        assert!(out.starts_with("HTTP/1.1 400") && l.is_empty());
        let (out, l) = exchange("nonsense\r\n\r\n", true);
        assert!(out.starts_with("HTTP/1.1 400") && l.is_empty());
    }

    #[test]
    fn a_big_body_is_refused_before_it_is_read_and_an_empty_one_is_fine() {
        let (out, l) = exchange(&format!("POST /hook/7/{{SECRET}} HTTP/1.1\r\nHost: 127.0.0.1:{{PORT}}\r\nContent-Length: {}\r\n\r\n", MAX_BODY + 1), true);
        assert!(out.starts_with("HTTP/1.1 413") && l.is_empty());
        let (out, l) = exchange("POST /hook/7/{SECRET} HTTP/1.1\r\nHost: localhost:{PORT}\r\n\r\n", true);
        assert!(out.starts_with("HTTP/1.1 202"), "{out}");
        assert_eq!(l[0].1["text"], "");
    }

    #[test]
    fn paused_or_off_automations_answer_only_the_right_link_and_start_nothing() {
        let (out, l) = exchange(&post("/hook/7/{SECRET}", "", "x"), false);
        assert!(out.starts_with("HTTP/1.1 503") && l.is_empty());
        let (out, l) = exchange(&post("/hook/7/wrongwrongwrongwrongwrong", "", "x"), false);
        assert!(out.starts_with("HTTP/1.1 404") && l.is_empty());
    }

    #[test]
    fn only_this_computer_counts_as_local() {
        for (ip, local) in [("127.0.0.1", true), ("::1", true), ("::ffff:127.0.0.1", true), ("192.168.1.5", false), ("10.0.0.2", false), ("8.8.8.8", false), ("fe80::1", false)] {
            assert_eq!(is_local(ip.parse().unwrap()), local, "{ip}");
        }
    }

    #[test]
    fn limits_stop_floods_and_guessing() {
        let t = Instant::now();
        let mut l = Limits::default();
        for _ in 0..PER_JOB {
            assert!(l.job(1, t));
        }
        assert!(!l.job(1, t));
        assert!(l.job(2, t));
        assert!(l.job(1, t + MINUTE + Duration::from_secs(1)));
        let mut l = Limits::default();
        for _ in 0..BAD {
            assert!(l.admit(t));
            l.wrong(t);
        }
        assert!(!l.admit(t));
        assert!(l.admit(t + MINUTE + Duration::from_secs(1)));
        let mut l = Limits::default();
        for _ in 0..ALL {
            assert!(l.admit(t));
        }
        assert!(!l.admit(t));
    }

    #[test]
    fn secrets_are_long_random_and_compared_whole() {
        let (a, b) = (new_secret().unwrap(), new_secret().unwrap());
        assert_eq!(a.len(), 48);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert!(same(&a, &a) && !same(&a, &b) && !same(&a, &a[..47]) && !same("", "\0"));
        assert!(url(7, &a).starts_with("http://127.0.0.1:51861/hook/7/"));
    }

    #[test]
    fn the_trigger_has_no_settings() {
        assert!(spec(&json!({})).is_ok());
        for bad in [json!({"secret":"x"}), json!(null), json!([]), json!({"port":1})] {
            assert!(spec(&bad).is_err());
        }
    }
}
