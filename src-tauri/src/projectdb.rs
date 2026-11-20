//! A DEV project's own database (MySQL / MariaDB): the engine behind the DEV tab's Database panel.
//! **Local** is the project's database on this computer, edits allowed. **Production** is the live
//! one the member fills in once, over TLS, opened in Look.
//!
//! Look is a look twice over: one reading statement only, in a read-only session with a time limit
//! and a row cap. Change is the member's button: on Production a write runs in a held transaction they
//! keep or undo (undone by itself after two minutes), and a change to a table's shape runs only after
//! their second yes. Every statement is logged without its rows. The companion's own memory database
//! is `db.rs`, never this.
//!
//! The login comes from `projectdb_login.rs` (what the member saved, or the project's `.env`); this
//! file only speaks to the server.

use mysql::prelude::Queryable;
use mysql::{Conn, OptsBuilder, SslOpts, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The most rows a look brings back.
pub const ROW_CAP: usize = 500;
/// A look's time limit on the server.
pub const LOOK_SECONDS: u64 = 15;
/// A change the member has not kept is undone after this long.
pub const HOLD_SECONDS: u64 = 120;

// ---------- pure, tested ----------

/// What one piece of SQL is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Empty,
    /// SELECT, SHOW, DESCRIBE, EXPLAIN, WITH ... SELECT
    Read,
    /// INSERT, UPDATE, DELETE, REPLACE - undoable inside a transaction
    Write,
    /// ALTER, DROP, CREATE, TRUNCATE, RENAME and anything else - MySQL cannot undo it
    Shape,
    /// more than one statement
    Several,
}

/// The SQL with comments and quoted text blanked, so keywords and `;` are read only where they count.
pub fn blanked(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '-' && next == Some('-') || c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            out.push(' ');
            continue;
        }
        if c == '/' && next == Some('*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i += 2;
            out.push(' ');
            continue;
        }
        if c == '\'' || c == '"' || c == '`' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && c != '`' {
                    i += 2;
                    continue;
                }
                if chars[i] == c {
                    if chars.get(i + 1) == Some(&c) {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i += 1;
            out.push_str(if c == '`' { " x " } else { " '' " });
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// The statements of a script, split on `;` outside comments and quotes (DELIMITER is not read).
pub fn statements(sql: &str) -> Vec<String> {
    let blank = blanked_keeping_length(sql);
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in blank.char_indices() {
        if c == ';' {
            let piece = sql[start..i].trim();
            if !blanked(piece).trim().is_empty() {
                out.push(piece.to_string());
            }
            start = i + 1;
        }
    }
    let rest = sql[start..].trim();
    if !blanked(rest).trim().is_empty() {
        out.push(rest.to_string());
    }
    out
}

/// Like `blanked`, but every blanked character becomes spaces of the same byte length, so its
/// indices are the original's.
fn blanked_keeping_length(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.char_indices().peekable();
    let fill = |out: &mut String, c: char| out.extend(std::iter::repeat_n(' ', c.len_utf8()));
    while let Some((_, c)) = chars.next() {
        let next = chars.peek().map(|(_, n)| *n);
        if c == '-' && next == Some('-') || c == '#' {
            fill(&mut out, c);
            while let Some(&(_, n)) = chars.peek() {
                if n == '\n' {
                    break;
                }
                fill(&mut out, n);
                chars.next();
            }
        } else if c == '/' && next == Some('*') {
            fill(&mut out, c);
            let mut prev = ' ';
            for (_, n) in chars.by_ref() {
                fill(&mut out, n);
                if prev == '*' && n == '/' {
                    break;
                }
                prev = n;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            fill(&mut out, c);
            let mut escaped = false;
            while let Some((_, n)) = chars.next() {
                fill(&mut out, n);
                if escaped {
                    escaped = false;
                } else if n == '\\' && c != '`' {
                    escaped = true;
                } else if n == c {
                    if chars.peek().map(|(_, m)| *m) == Some(c) {
                        fill(&mut out, c);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// What the SQL is, read past comments and quoted text.
pub fn classify(sql: &str) -> Kind {
    let parts = statements(sql);
    if parts.is_empty() {
        return Kind::Empty;
    }
    if parts.len() > 1 {
        return Kind::Several;
    }
    let words: Vec<String> = blanked(&parts[0])
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_uppercase())
        .collect();
    let first = words.first().map(String::as_str).unwrap_or("");
    let writes = |w: &String| matches!(w.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE");
    match first {
        "SELECT" | "SHOW" | "DESCRIBE" | "DESC" | "EXPLAIN" => {
            if words.iter().any(|w| matches!(w.as_str(), "INTO" | "OUTFILE" | "DUMPFILE")) && first == "SELECT" {
                Kind::Shape
            } else {
                Kind::Read
            }
        }
        "WITH" => {
            if words.iter().any(writes) {
                Kind::Write
            } else {
                Kind::Read
            }
        }
        "INSERT" | "UPDATE" | "DELETE" | "REPLACE" => Kind::Write,
        "" => Kind::Empty,
        _ => Kind::Shape,
    }
}

/// A login read from a project's own files (no password shown anywhere).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvLogin {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    #[serde(skip)]
    pub password: String,
}

/// `${NAME:fallback}` -> fallback; anything else as it is.
fn unplaceholder(v: &str) -> String {
    let v = v.trim().trim_matches('"').trim_matches('\'').trim();
    if let Some(inner) = v.strip_prefix("${").and_then(|r| r.strip_suffix('}')) {
        return inner.split_once(':').map(|(_, d)| d.to_string()).unwrap_or_default();
    }
    v.to_string()
}

/// `jdbc:mysql://host:3307/db?x=y` -> (host, port, db).
pub fn jdbc(url: &str) -> Option<(String, u16, String)> {
    let rest = url.trim().strip_prefix("jdbc:")?.split_once("://")?.1;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()?),
        None => (hostport.to_string(), 3306),
    };
    let db = path.split(['?', ';']).next().unwrap_or("").to_string();
    Some((host, port, db))
}

/// The database login in a Laravel `.env`, a Spring `application.properties`, or Spring's env names.
pub fn env_login(text: &str) -> Option<EnvLogin> {
    let mut map: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_ascii_lowercase().replace(['.', '-'], "_"), unplaceholder(v));
        }
    }
    let get = |keys: &[&str]| keys.iter().find_map(|k| map.get(*k).filter(|v| !v.is_empty()).cloned());
    let mut login = EnvLogin { host: "127.0.0.1".into(), port: 3306, ..EnvLogin::default() };
    let mut found = false;
    if let Some(url) = get(&["spring_datasource_url"]) {
        if let Some((h, p, d)) = jdbc(&url) {
            (login.host, login.port, login.database) = (h, p, d);
            found = true;
        }
    }
    if let Some(h) = get(&["db_host"]) {
        login.host = h;
        found = true;
    }
    if let Some(p) = get(&["db_port"]).and_then(|p| p.parse().ok()) {
        login.port = p;
    }
    if let Some(d) = get(&["db_database"]) {
        login.database = d;
        found = true;
    }
    if let Some(u) = get(&["db_username", "spring_datasource_username"]) {
        login.user = u;
        found = true;
    }
    login.password = get(&["db_password", "spring_datasource_password"]).unwrap_or_default();
    if login.host == "localhost" {
        login.host = "127.0.0.1".into();
    }
    found.then_some(login)
}

/// One value as SQL text, for a dump or an import.
pub fn sql_literal(v: &Value) -> String {
    match v {
        Value::NULL => "NULL".into(),
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => f.to_string(),
        Value::Bytes(b) => quote(&String::from_utf8_lossy(b)),
        other => quote(&text_of(other).unwrap_or_default()),
    }
}

/// A string as a quoted SQL literal.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\u{1a}' => out.push_str("\\Z"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// A name in backticks.
pub fn ident(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// A value as the grid shows it; None for NULL.
pub fn text_of(v: &Value) -> Option<String> {
    Some(match v {
        Value::NULL => return None,
        Value::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => f.to_string(),
        Value::Date(y, mo, d, h, mi, s, us) => {
            if (*h, *mi, *s, *us) == (0, 0, 0, 0) {
                format!("{y:04}-{mo:02}-{d:02}")
            } else {
                format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
            }
        }
        Value::Time(neg, days, h, mi, s, _) => format!("{}{:02}:{mi:02}:{s:02}", if *neg { "-" } else { "" }, *days * 24 + u32::from(*h)),
    })
}

/// A CSV line split into fields (quotes and doubled quotes read).
pub fn csv_fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur)),
            (c, _) => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// One cell for a CSV file.
pub fn csv_cell(v: Option<&str>) -> String {
    match v {
        None => String::new(),
        Some(s) if s.contains([',', '"', '\n', '\r']) => format!("\"{}\"", s.replace('"', "\"\"")),
        Some(s) => s.to_string(),
    }
}

// ---------- the log (no rows, no secret) ----------

#[derive(Debug, Serialize)]
struct LogLine<'a> {
    at: String,
    project: &'a str,
    side: &'a str,
    mode: &'a str,
    sql: &'a str,
    rows: u64,
    ms: u64,
}

/// Unix seconds as an ISO time in UTC: "2026-11-17T15:50:00Z".
pub fn iso_utc(secs: i64) -> String {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil from days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

fn now_iso() -> String {
    iso_utc(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0))
}

/// Where a statement is written down, and for which project.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// Castle's settings folder: `sql-log.jsonl` and `sql-saved.json` live there.
    pub folder: PathBuf,
    /// The project's name, for the log.
    pub project: String,
}

impl Ctx {
    fn log(&self, side: &str, mode: &str, sql: &str, rows: u64, ms: u64) {
        let line = LogLine { at: now_iso(), project: &self.project, side, mode, sql, rows, ms };
        if let Ok(text) = serde_json::to_string(&line) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.folder.join("sql-log.jsonl")) {
                let _ = writeln!(f, "{text}");
            }
        }
    }
}

// ---------- the connection ----------

/// Everything needed to open one side's database. Built by `projectdb_login.rs`; never sent to the page.
#[derive(Clone, Default)]
pub struct Login {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    /// The cluster's CA certificate (a copy Castle keeps), checked against the server.
    pub ca: Option<PathBuf>,
    /// Production: TLS always, writes held, imports only when unlocked.
    pub production: bool,
}

impl Login {
    /// The same login on another database of the same server (the panel's database list).
    pub fn on(&self, database: &str) -> Login {
        let mut next = self.clone();
        if !database.is_empty() {
            next.database = database.to_string();
        }
        next
    }

    fn side(&self) -> &'static str {
        if self.production {
            "production"
        } else {
            "local"
        }
    }
}

fn connect(login: &Login) -> Result<Conn, String> {
    let mut opts = OptsBuilder::new()
        .ip_or_hostname(Some(login.host.clone()))
        .tcp_port(login.port)
        .user(Some(login.user.clone()))
        .pass(Some(login.password.clone()).filter(|p| !p.is_empty()))
        .db_name(Some(login.database.clone()).filter(|d| !d.is_empty()))
        .tcp_connect_timeout(Some(Duration::from_secs(5)))
        .read_timeout(Some(Duration::from_secs(LOOK_SECONDS + 15)))
        .write_timeout(Some(Duration::from_secs(30)));
    if login.production {
        let mut ssl = SslOpts::default();
        if let Some(ca) = &login.ca {
            ssl = ssl.with_root_cert_path(Some(ca.clone()));
        }
        opts = opts.ssl_opts(Some(ssl));
    }
    Conn::new(opts).map_err(|e| {
        let text = e.to_string();
        if text.contains("Access denied") {
            "The database refused the login - check the user and password.".into()
        } else if text.contains("timed out") || text.contains("10060") || text.contains("refused") {
            format!("{}:{} did not answer - is this computer's address allowed on the host, and is the server up?", login.host, login.port)
        } else if text.to_lowercase().contains("certificate") || text.contains("UnknownIssuer") {
            "The server's certificate could not be checked - choose the cluster's CA certificate in Edit connection.".into()
        } else {
            format!("Could not connect: {text}")
        }
    })
}

/// Opens a connection and closes it: the form's Test connection.
pub fn test(login: &Login) -> Result<String, String> {
    let mut conn = connect(login)?;
    let version: Option<String> = conn.query_first("SELECT VERSION()").map_err(|e| e.to_string())?;
    Ok(version.unwrap_or_default())
}

// ---------- reading ----------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub truncated: bool,
    pub affected: u64,
    pub ms: u64,
}

fn collect(result: mysql::QueryResult<'_, '_, '_, mysql::Text>, cap: usize) -> Result<Rows, String> {
    let mut out = Rows::default();
    let mut result = result;
    if let Some(set) = result.iter() {
        out.columns = set.columns().as_ref().iter().map(|c| c.name_str().into_owned()).collect();
        for row in set {
            let row = row.map_err(|e| e.to_string())?;
            if out.rows.len() >= cap {
                out.truncated = true;
                break;
            }
            out.rows.push(row.unwrap().iter().map(text_of).collect());
        }
    }
    out.affected = result.affected_rows();
    Ok(out)
}

pub fn databases(login: &Login) -> Result<Vec<String>, String> {
    let mut conn = connect(&Login { database: String::new(), ..login.clone() })?;
    let all: Vec<String> = conn.query("SHOW DATABASES").map_err(|e| e.to_string())?;
    Ok(all.into_iter().filter(|d| !matches!(d.as_str(), "information_schema" | "performance_schema" | "mysql" | "sys")).collect())
}

pub fn tables(login: &Login) -> Result<Vec<String>, String> {
    let mut conn = connect(login)?;
    conn.query("SHOW TABLES").map_err(|e| e.to_string())
}

/// About how many rows each table holds: MySQL's own estimate from `information_schema`, one read,
/// nothing counted row by row. A view has no count and is left out.
pub fn table_rows(login: &Login) -> Result<std::collections::BTreeMap<String, u64>, String> {
    let mut conn = connect(login)?;
    let rows: Vec<(String, Option<u64>)> = conn
        .query("SELECT TABLE_NAME, TABLE_ROWS FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE()")
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().filter_map(|(name, n)| n.map(|n| (name, n))).collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub kind: String,
    pub key: bool,
}

pub fn columns(login: &Login, table: &str) -> Result<Vec<Column>, String> {
    let mut conn = connect(login)?;
    let rows: Vec<mysql::Row> = conn.query(format!("SHOW COLUMNS FROM {}", ident(table))).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let get = |i: usize| r.get_opt::<Option<String>, _>(i).and_then(Result::ok).flatten().unwrap_or_default();
            Column { name: get(0), kind: get(1), key: get(3) == "PRI" }
        })
        .collect())
}

/// One reading statement, in a read-only session with a time limit and the row cap.
pub fn look(login: &Login, sql: &str, ctx: &Ctx) -> Result<Rows, String> {
    if classify(sql) != Kind::Read {
        return Err("Look runs one reading statement only (SELECT, SHOW, DESCRIBE, EXPLAIN).".into());
    }
    let mut conn = connect(login)?;
    conn.query_drop("SET SESSION TRANSACTION READ ONLY").map_err(|e| e.to_string())?;
    // MySQL counts milliseconds, MariaDB seconds: whichever the server knows
    let _ = conn.query_drop(format!("SET SESSION MAX_EXECUTION_TIME = {}", LOOK_SECONDS * 1000));
    let _ = conn.query_drop(format!("SET SESSION max_statement_time = {LOOK_SECONDS}"));
    let start = Instant::now();
    let result = conn.query_iter(sql).map_err(|e| e.to_string())?;
    let mut rows = collect(result, ROW_CAP)?;
    rows.ms = start.elapsed().as_millis() as u64;
    ctx.log(login.side(), "look", sql, rows.rows.len() as u64, rows.ms);
    Ok(rows)
}

// ---------- changing ----------

/// Changes waiting for the member's Keep or Undo, by id.
#[derive(Default)]
pub struct Held(pub Mutex<HashMap<u64, (Conn, Instant)>>);

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Undoes every change older than HOLD_SECONDS.
pub fn sweep(held: &Held) {
    let mut map = held.0.lock().unwrap();
    let old: Vec<u64> = map.iter().filter(|(_, (_, at))| at.elapsed() > Duration::from_secs(HOLD_SECONDS)).map(|(id, _)| *id).collect();
    for id in old {
        if let Some((mut conn, _)) = map.remove(&id) {
            let _ = conn.query_drop("ROLLBACK");
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Changed {
    /// Waiting for Keep or Undo; 0 when it is already final (a Local edit, a shape change).
    pub id: u64,
    pub affected: u64,
    pub ms: u64,
}

/// Why a piece of SQL cannot run as a change, or None when it can. A shape change needs the
/// member's second yes (`confirmed`), on either side.
pub fn change_refusal(kind: Kind, confirmed: bool) -> Option<&'static str> {
    match kind {
        Kind::Write => None,
        Kind::Shape if confirmed => None,
        Kind::Shape => Some("A change to a table's shape cannot be undone - confirm it first."),
        Kind::Several => Some("One statement at a time."),
        Kind::Read => Some("That is a reading statement - run it as a look."),
        Kind::Empty => Some("There is nothing to run."),
    }
}

/// A change. On Production a write runs in a held transaction (Keep / Undo); a shape change and any
/// Local edit are final at once.
pub fn change(held: &Held, login: &Login, sql: &str, confirmed: bool, ctx: &Ctx) -> Result<Changed, String> {
    sweep(held);
    let kind = classify(sql);
    if let Some(why) = change_refusal(kind, confirmed) {
        return Err(why.into());
    }
    let mut conn = connect(login)?;
    let start = Instant::now();
    if login.production && kind == Kind::Write {
        conn.query_drop("START TRANSACTION").map_err(|e| e.to_string())?;
        conn.query_drop(sql).map_err(|e| e.to_string())?;
        let affected = conn.affected_rows();
        let ms = start.elapsed().as_millis() as u64;
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        held.0.lock().unwrap().insert(id, (conn, Instant::now()));
        ctx.log(login.side(), "change (held)", sql, affected, ms);
        return Ok(Changed { id, affected, ms });
    }
    conn.query_drop(sql).map_err(|e| e.to_string())?;
    let changed = Changed { id: 0, affected: conn.affected_rows(), ms: start.elapsed().as_millis() as u64 };
    ctx.log(login.side(), if kind == Kind::Shape { "shape" } else { "change" }, sql, changed.affected, changed.ms);
    Ok(changed)
}

/// The member's Keep (commit) or Undo (rollback) of a held change.
pub fn finish(held: &Held, id: u64, keep: bool) -> Result<(), String> {
    sweep(held);
    let (mut conn, _) = held.0.lock().unwrap().remove(&id).ok_or("That change was already undone - two minutes passed.")?;
    conn.query_drop(if keep { "COMMIT" } else { "ROLLBACK" }).map_err(|e| e.to_string())
}

// ---------- files ----------

pub fn export_csv(rows: &Rows, path: &Path) -> Result<(), String> {
    let mut text = rows.columns.iter().map(|c| csv_cell(Some(c))).collect::<Vec<_>>().join(",");
    text.push_str("\r\n");
    for row in &rows.rows {
        text.push_str(&row.iter().map(|v| csv_cell(v.as_deref())).collect::<Vec<_>>().join(","));
        text.push_str("\r\n");
    }
    std::fs::write(path, text).map_err(|_| "The file could not be written.".to_string())
}

/// A dump of one table or the whole database: each table's CREATE and its rows in INSERTs of 200.
pub fn dump(login: &Login, table: Option<&str>, path: &Path, ctx: &Ctx) -> Result<u64, String> {
    let mut conn = connect(login)?;
    let names: Vec<String> = match table {
        Some(t) => vec![t.to_string()],
        None => conn.query("SHOW TABLES").map_err(|e| e.to_string())?,
    };
    let mut out = format!("-- MemoryCore Castle dump of {} ({})\nSET FOREIGN_KEY_CHECKS=0;\n\n", login.database, login.side());
    let mut total = 0u64;
    for name in &names {
        let create: Option<(String, String)> = conn.query_first(format!("SHOW CREATE TABLE {}", ident(name))).map_err(|e| e.to_string())?;
        if let Some((_, sql)) = create {
            out.push_str(&format!("DROP TABLE IF EXISTS {};\n{sql};\n\n", ident(name)));
        }
        let rows: Vec<mysql::Row> = conn.query(format!("SELECT * FROM {}", ident(name))).map_err(|e| e.to_string())?;
        for chunk in rows.chunks(200) {
            let values: Vec<String> = chunk.iter().map(|r| format!("({})", r.clone().unwrap().iter().map(sql_literal).collect::<Vec<_>>().join(","))).collect();
            out.push_str(&format!("INSERT INTO {} VALUES\n{};\n", ident(name), values.join(",\n")));
            total += chunk.len() as u64;
        }
        out.push('\n');
    }
    out.push_str("SET FOREIGN_KEY_CHECKS=1;\n");
    std::fs::write(path, out).map_err(|_| "The dump could not be written.".to_string())?;
    ctx.log(login.side(), "dump", table.unwrap_or("*"), total, 0);
    Ok(total)
}

/// An import: a `.sql` script statement by statement, or a CSV (first line the column names) into
/// `table`. Production only while Change is unlocked.
pub fn import(login: &Login, path: &Path, table: Option<&str>, unlocked: bool, ctx: &Ctx) -> Result<u64, String> {
    if login.production && !unlocked {
        return Err("Unlock Change to import into Production.".into());
    }
    let text = std::fs::read_to_string(path).map_err(|_| "The file could not be read.".to_string())?;
    let mut conn = connect(login)?;
    let is_csv = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    let mut count = 0u64;
    if is_csv {
        let table = table.filter(|t| !t.is_empty()).ok_or("Pick the table the CSV goes into.")?;
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let head = csv_fields(lines.next().ok_or("The CSV is empty.")?);
        let cols = head.iter().map(|c| ident(c.trim())).collect::<Vec<_>>().join(",");
        for line in lines {
            let vals = csv_fields(line).iter().map(|v| if v.is_empty() { "NULL".to_string() } else { quote(v) }).collect::<Vec<_>>().join(",");
            conn.query_drop(format!("INSERT INTO {} ({cols}) VALUES ({vals})", ident(table))).map_err(|e| format!("Row {}: {e}", count + 1))?;
            count += 1;
        }
    } else {
        for statement in statements(&text) {
            conn.query_drop(&statement).map_err(|e| format!("Statement {}: {e}", count + 1))?;
            count += 1;
        }
    }
    ctx.log(login.side(), "import", &path.display().to_string(), count, 0);
    Ok(count)
}

// ---------- saved queries (per project, on this computer) ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Saved {
    pub name: String,
    pub sql: String,
}

fn saved_file(folder: &Path) -> PathBuf {
    folder.join("sql-saved.json")
}

/// The saved queries of one project, by its folder key.
pub fn saved(settings_folder: &Path, key: &str) -> Vec<Saved> {
    let all: HashMap<String, Vec<Saved>> = std::fs::read_to_string(saved_file(settings_folder)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    all.get(key).cloned().unwrap_or_default()
}

pub fn set_saved(settings_folder: &Path, key: &str, list: Vec<Saved>) -> Result<(), String> {
    let mut all: HashMap<String, Vec<Saved>> = std::fs::read_to_string(saved_file(settings_folder)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let list: Vec<Saved> = list.into_iter().filter(|s| !s.name.trim().is_empty() && !s.sql.trim().is_empty()).take(100).collect();
    all.insert(key.to_string(), list);
    std::fs::write(saved_file(settings_folder), serde_json::to_string_pretty(&all).unwrap_or_default()).map_err(|_| "The saved queries could not be kept.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_a_statement_is_past_comments_and_quotes() {
        assert_eq!(classify("SELECT * FROM trips"), Kind::Read);
        assert_eq!(classify("  -- today\n select 1;  "), Kind::Read);
        assert_eq!(classify("show tables"), Kind::Read);
        assert_eq!(classify("WITH t AS (SELECT 1) SELECT * FROM t"), Kind::Read);
        assert_eq!(classify("WITH t AS (SELECT 1) DELETE FROM x"), Kind::Write);
        assert_eq!(classify("UPDATE trips SET status = 'done' WHERE id = 1"), Kind::Write);
        assert_eq!(classify("DROP TABLE trips"), Kind::Shape);
        assert_eq!(classify("SELECT * INTO OUTFILE '/tmp/x' FROM t"), Kind::Shape);
        assert_eq!(classify("SET GLOBAL x = 1"), Kind::Shape);
        assert_eq!(classify("SELECT 1; DELETE FROM trips"), Kind::Several);
        // a ; or a keyword inside quotes or a comment does not count
        assert_eq!(classify("SELECT 'a; DELETE FROM t' AS x"), Kind::Read);
        assert_eq!(classify("SELECT 1 /* ; drop table t */"), Kind::Read);
        assert_eq!(classify("SELECT `delete` FROM t"), Kind::Read);
        assert_eq!(classify("   ;  -- nothing"), Kind::Empty);
    }

    #[test]
    fn a_shape_change_waits_for_the_second_yes() {
        assert_eq!(change_refusal(Kind::Write, false), None);
        assert!(change_refusal(Kind::Shape, false).is_some());
        assert_eq!(change_refusal(Kind::Shape, true), None);
        assert!(change_refusal(Kind::Read, true).is_some());
        assert!(change_refusal(Kind::Several, true).is_some());
        assert!(change_refusal(Kind::Empty, true).is_some());
    }

    #[test]
    fn splits_a_script_into_statements() {
        let s = statements("CREATE TABLE a (x TEXT);\nINSERT INTO a VALUES ('1;2');\n-- end;\n");
        assert_eq!(s, vec!["CREATE TABLE a (x TEXT)".to_string(), "INSERT INTO a VALUES ('1;2')".to_string()]);
        assert_eq!(statements("SELECT 'it''s; fine'"), vec!["SELECT 'it''s; fine'".to_string()]);
    }

    #[test]
    fn finds_the_local_login_in_laravel_and_spring() {
        let laravel = env_login("APP_NAME=x\nDB_HOST=localhost\nDB_PORT=3307\nDB_DATABASE=driver_app\nDB_USERNAME=root\nDB_PASSWORD=\n").unwrap();
        assert_eq!((laravel.host.as_str(), laravel.port, laravel.database.as_str(), laravel.user.as_str()), ("127.0.0.1", 3307, "driver_app", "root"));
        let spring = env_login("spring.datasource.url=jdbc:mysql://127.0.0.1:3306/bus_central?useSSL=false\nspring.datasource.username=${DB_USER:bus}\nspring.datasource.password=${DB_PASS:}\n").unwrap();
        assert_eq!((spring.port, spring.database.as_str(), spring.user.as_str(), spring.password.as_str()), (3306, "bus_central", "bus", ""));
        assert_eq!(jdbc("jdbc:mysql://db.example:25060/app"), Some(("db.example".into(), 25060, "app".into())));
        assert!(env_login("APP_NAME=x\n").is_none());
    }

    #[test]
    fn writes_values_and_csv_safely() {
        assert_eq!(quote("it's\n\\"), "'it\\'s\\n\\\\'");
        assert_eq!(sql_literal(&Value::NULL), "NULL");
        assert_eq!(sql_literal(&Value::Int(-4)), "-4");
        assert_eq!(sql_literal(&Value::Bytes(b"a'b".to_vec())), "'a\\'b'");
        assert_eq!(text_of(&Value::Date(2026, 11, 17, 0, 0, 0, 0)).unwrap(), "2026-11-17");
        assert_eq!(ident("we`ird"), "`we``ird`");
        assert_eq!(csv_fields("1,\"a, b\",\"say \"\"hi\"\"\","), vec!["1", "a, b", "say \"hi\"", ""]);
        assert_eq!(csv_cell(Some("a,b")), "\"a,b\"");
        assert_eq!(csv_cell(None), "");
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_792_135_888), "2026-10-16T07:31:28Z");
    }

    #[test]
    fn saved_queries_are_kept_per_project() {
        let folder = std::env::temp_dir().join(format!("castle-projectdb-saved-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let one = vec![Saved { name: "paid".into(), sql: "SELECT * FROM orders WHERE paid".into() }, Saved { name: " ".into(), sql: "x".into() }];
        set_saved(&folder, "d:/shop", one).unwrap();
        assert_eq!(saved(&folder, "d:/shop").len(), 1, "a nameless query is not kept");
        assert!(saved(&folder, "d:/other").is_empty());
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_login_moves_to_another_database_of_the_same_server() {
        let login = Login { host: "h".into(), port: 3306, database: "app".into(), ..Login::default() };
        assert_eq!(login.on("other").database, "other");
        assert_eq!(login.on("").database, "app");
        assert_eq!(login.side(), "local");
    }

    #[test]
    #[ignore = "reads a real local database (CASTLE_DB_HOST, CASTLE_DB_USER, CASTLE_DB_NAME)"]
    fn real_local() {
        let (Ok(host), Ok(user), Ok(database)) = (std::env::var("CASTLE_DB_HOST"), std::env::var("CASTLE_DB_USER"), std::env::var("CASTLE_DB_NAME")) else { return };
        let login = Login { host, port: 3306, user, database, ..Login::default() };
        println!("{:?}", test(&login));
        let ctx = Ctx { folder: std::env::temp_dir(), project: "check".into() };
        let tables = tables(&login).unwrap();
        println!("{} tables", tables.len());
        if let Some(first) = tables.first() {
            println!("{:?}", look(&login, &format!("SELECT * FROM {} LIMIT 3", ident(first)), &ctx).map(|r| r.rows.len()));
            println!("a delete as a look: {:?}", look(&login, &format!("DELETE FROM {}", ident(first)), &ctx).err());
        }
    }
}
