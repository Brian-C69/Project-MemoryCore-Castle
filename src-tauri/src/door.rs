//! The one door: how the companion reads and writes the memory database from its own terminal.
//!
//! ```text
//!  castle-memory diary add --date 2026-11-10 --title "The garden" --file entry.md
//!  castle-memory diary list [--month 2026-11] [--limit 20]      diary show --date 2026-11-10
//!  castle-memory reminder add --title "Call Rin" [--due 2026-11-12] [--detail "..."]
//!  castle-memory reminder list | reminder done --id 3
//!  castle-memory project add|update --name "Portfolio site" [--status ..] [--summary ..] [--file notes.md]
//!  castle-memory project list | project show --name "Portfolio site"
//!  castle-memory link send --from main --to story --about "..." [--word "..."] [--no-wake]
//!  castle-memory link inbox --to main | link read --id 7 | link here --name main
//!  castle-memory export --to <folder>        the memory back out as markdown
//!  castle-memory dump --to backup/memory.sql | restore --from backup/memory.sql
//! ```
//!
//! `castle-memory` is Castle's own program started with `--memory` (the tools folder holds the
//! small launcher). It finds the database through the memory folder's .env.

use crate::memdb;
use mysql::prelude::Queryable;
use mysql::{Conn, Row, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The argument that turns Castle's program into the door.
pub const FLAG: &str = "--memory";

/// The words after `castle-memory`: plain words in order, and `--name value` pairs
/// (`--no-wake` style flags carry "true").
#[derive(Debug, Default, PartialEq)]
pub struct Ask {
    pub words: Vec<String>,
    pub flags: BTreeMap<String, String>,
}

const BARE_FLAGS: [&str; 2] = ["no-wake", "all"];

pub fn parse(args: &[String]) -> Result<Ask, String> {
    let mut ask = Ask::default();
    let mut i = 0;
    while i < args.len() {
        if let Some(name) = args[i].strip_prefix("--") {
            if BARE_FLAGS.contains(&name) {
                ask.flags.insert(name.to_string(), "true".into());
            } else {
                let value = args.get(i + 1).ok_or(format!("--{name} needs a value."))?;
                ask.flags.insert(name.to_string(), value.clone());
                i += 1;
            }
        } else {
            ask.words.push(args[i].clone());
        }
        i += 1;
    }
    Ok(ask)
}

impl Ask {
    fn need(&self, name: &str) -> Result<&str, String> {
        self.flags.get(name).map(String::as_str).filter(|v| !v.trim().is_empty()).ok_or(format!("--{name} is needed."))
    }
    fn get(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }
    /// The long text of a verb: `--file <path>` (read as UTF-8) or `--body "..."`.
    fn text(&self) -> Result<Option<String>, String> {
        if let Some(file) = self.get("file") {
            let text = std::fs::read_to_string(file).map_err(|_| format!("The file {file} could not be read."))?;
            return Ok(Some(text.trim_start_matches('\u{feff}').to_string()));
        }
        Ok(self.get("body").map(str::to_string))
    }
}

/// "2026-11-10": four digits, dash, two, dash, two.
pub fn is_date(value: &str) -> bool {
    value.len() == 10 && value.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
}

fn date(value: &str) -> Result<&str, String> {
    if is_date(value) { Ok(value) } else { Err(format!("{value} is not a date like 2026-11-10.")) }
}

fn sql(what: &'static str) -> impl Fn(mysql::Error) -> String {
    move |_| format!("{what} did not work.")
}

/// The memory folder: `--memory-folder`, else the nearest folder upward from here whose .env
/// names a memory database.
pub fn find_memory(ask: &Ask, from: &Path) -> Result<PathBuf, String> {
    if let Some(folder) = ask.get("memory-folder") {
        return Ok(PathBuf::from(folder));
    }
    let mut at = Some(from);
    while let Some(folder) = at {
        if std::fs::read_to_string(folder.join(".env")).is_ok_and(|t| memdb::access_from_env(&t).is_some()) {
            return Ok(folder.to_path_buf());
        }
        at = folder.parent();
    }
    Err("No memory database was found from this folder. Run this inside your MemoryCore folder.".into())
}

/// Runs one ask against an open database; the text to print.
pub fn run(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let words: Vec<&str> = ask.words.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["diary", "add"] => diary_add(conn, ask),
        ["diary", "list"] => diary_list(conn, ask),
        ["diary", "show"] => diary_show(conn, ask),
        ["reminder", "add"] => reminder_add(conn, ask),
        ["reminder", "list"] => reminder_list(conn, ask),
        ["reminder", "done"] => reminder_done(conn, ask),
        ["project", "add"] | ["project", "update"] => project_save(conn, ask),
        ["project", "list"] => project_list(conn),
        ["project", "show"] => project_show(conn, ask),
        ["link", "send"] => link_send(conn, ask),
        ["link", "inbox"] => link_inbox(conn, ask),
        ["link", "read"] => link_read(conn, ask),
        ["link", "here"] => link_here(conn, ask),
        ["export"] => export(conn, Path::new(ask.need("to")?)),
        ["dump"] => dump(conn, Path::new(ask.need("to")?)),
        ["restore"] => restore(conn, Path::new(ask.need("from")?)),
        _ => Err(format!("castle-memory does not know `{}`. Verbs: diary, reminder, project, link, export, dump, restore.", words.join(" "))),
    }
}

// ---------- diary ----------

fn diary_add(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let day = date(ask.need("date")?)?;
    let title = ask.need("title")?;
    let body = ask.text()?.ok_or("--file or --body is needed.")?;
    let next: Option<u32> = conn
        .exec_first("SELECT COALESCE(MAX(session_no), 0) + 1 FROM diary_entries WHERE entry_date = ?", (day,))
        .map_err(sql("Reading the diary"))?;
    let session = next.unwrap_or(1);
    conn.exec_drop("INSERT INTO diary_entries (entry_date, session_no, title, body) VALUES (?, ?, ?, ?)", (day, session, title, body))
        .map_err(sql("Saving the diary entry"))?;
    Ok(format!("DIARY saved {day} session {session}: {title}"))
}

fn diary_list(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let limit: u32 = ask.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);
    let month = ask.get("month").map(|m| format!("{m}%")).unwrap_or_else(|| "%".into());
    let rows: Vec<(Option<String>, u32, String)> = conn
        .exec(
            "SELECT DATE_FORMAT(entry_date, '%Y-%m-%d'), session_no, title FROM diary_entries \
             WHERE COALESCE(DATE_FORMAT(entry_date, '%Y-%m-%d'), '') LIKE ? ORDER BY entry_date DESC, session_no DESC LIMIT ?",
            (month, limit),
        )
        .map_err(sql("Reading the diary"))?;
    if rows.is_empty() {
        return Ok("DIARY empty".into());
    }
    Ok(rows.into_iter().map(|(d, n, t)| format!("{} #{n} {t}", d.unwrap_or_else(|| "earlier".into()))).collect::<Vec<_>>().join("\n"))
}

fn diary_show(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let day = date(ask.need("date")?)?;
    let rows: Vec<(u32, String, String, Option<String>)> = conn
        .exec("SELECT session_no, title, body, source_file FROM diary_entries WHERE entry_date = ? ORDER BY session_no", (day,))
        .map_err(sql("Reading the diary"))?;
    let wanted: Option<u32> = ask.get("session").and_then(|v| v.parse().ok());
    // a page that was moved in is its own whole text; one written through the door gets its heading
    let parts: Vec<String> = rows
        .into_iter()
        .filter(|(n, _, _, _)| wanted.is_none_or(|w| w == *n))
        .map(|(n, t, b, source)| if source.is_some() { b.trim_end().to_string() } else { format!("## Session {n} - {t}\n{}", b.trim_end()) })
        .collect();
    if parts.is_empty() {
        return Ok(format!("DIARY nothing on {day}"));
    }
    Ok(format!("# Daily Diary - {day}\n\n{}", parts.join("\n\n")))
}

// ---------- reminders ----------

fn reminder_add(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let title = ask.need("title")?;
    let due = ask.get("due").map(date).transpose()?;
    conn.exec_drop(
        "INSERT INTO reminders (title, due_date, detail, position) SELECT ?, ?, ?, COALESCE(MAX(position), 0) + 1 FROM reminders",
        (title, due, ask.get("detail").unwrap_or("")),
    )
    .map_err(sql("Saving the reminder"))?;
    Ok(format!("REMINDER saved #{}: {title}", conn.last_insert_id()))
}

fn reminder_list(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let state = if ask.get("all").is_some() { "%" } else { "open" };
    let rows: Vec<(u64, String, Option<String>, String, String)> = conn
        .exec(
            "SELECT id, title, DATE_FORMAT(due_date, '%Y-%m-%d'), detail, state FROM reminders WHERE state LIKE ? ORDER BY state DESC, position, id",
            (state,),
        )
        .map_err(sql("Reading the reminders"))?;
    if rows.is_empty() {
        return Ok("REMINDERS none open".into());
    }
    Ok(rows
        .into_iter()
        .map(|(id, title, due, detail, state)| {
            let due = due.map(|d| format!(" (by {d})")).unwrap_or_default();
            let detail = if detail.is_empty() { String::new() } else { format!(": {detail}") };
            let done = if state == "done" { " [done]" } else { "" };
            format!("#{id} {title}{due}{detail}{done}")
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn reminder_done(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let id: u64 = ask.need("id")?.trim_start_matches('#').parse().map_err(|_| "--id is a number, like 3.".to_string())?;
    conn.exec_drop("UPDATE reminders SET state = 'done', done_at = NOW() WHERE id = ? AND state = 'open'", (id,)).map_err(sql("Closing the reminder"))?;
    if conn.affected_rows() == 0 {
        return Err(format!("No open reminder #{id}."));
    }
    Ok(format!("REMINDER #{id} done"))
}

// ---------- projects ----------

fn project_save(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let name = ask.need("name")?;
    let body = ask.text()?;
    conn.exec_drop(
        "INSERT INTO projects (name, status, shelf, summary, body, last_updated) VALUES (?, ?, COALESCE(?, 'active'), ?, ?, DATE_FORMAT(NOW(), '%Y-%m-%d')) \
         ON DUPLICATE KEY UPDATE status = COALESCE(VALUES(status), status), shelf = COALESCE(?, shelf), \
         summary = COALESCE(VALUES(summary), summary), body = COALESCE(VALUES(body), body), last_updated = VALUES(last_updated)",
        (name, ask.get("status"), ask.get("shelf"), ask.get("summary"), body, ask.get("shelf")),
    )
    .map_err(sql("Saving the project"))?;
    Ok(format!("PROJECT saved: {name}"))
}

fn project_list(conn: &mut Conn) -> Result<String, String> {
    let rows: Vec<(String, Option<String>, String, Option<String>)> = conn
        .query("SELECT name, status, shelf, last_updated FROM projects ORDER BY shelf, updated_at DESC, id DESC")
        .map_err(sql("Reading the projects"))?;
    if rows.is_empty() {
        return Ok("PROJECTS none".into());
    }
    Ok(rows
        .into_iter()
        .map(|(name, status, shelf, updated)| format!("[{shelf}] {name} · {} · {}", status.unwrap_or_else(|| "-".into()), updated.unwrap_or_else(|| "-".into())))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn project_show(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let name = ask.need("name")?;
    let row: Option<(Option<String>, Option<String>, Option<String>)> =
        conn.exec_first("SELECT status, summary, body FROM projects WHERE name = ?", (name,)).map_err(sql("Reading the project"))?;
    let (status, summary, body) = row.ok_or(format!("No project called {name}."))?;
    let mut out = format!("# {name}\nStatus: {}\n", status.unwrap_or_else(|| "-".into()));
    if let Some(summary) = summary.filter(|s| !s.is_empty()) {
        out.push_str(&format!("Summary: {summary}\n"));
    }
    if let Some(body) = body.filter(|b| !b.trim().is_empty()) {
        out.push_str(&format!("\n{}\n", body.trim_end()));
    }
    Ok(out)
}

// ---------- the Neural Link ----------

fn terminal(value: &str) -> Result<&str, String> {
    let mut chars = value.chars();
    let ok = matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && value.len() <= 40
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok { Ok(value) } else { Err(format!("{value} is not a terminal name (lowercase letters, digits and dashes).")) }
}

fn link_send(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let (from, to) = (terminal(ask.need("from")?)?, terminal(ask.need("to")?)?);
    let wake = ask.get("no-wake").is_none();
    conn.exec_drop(
        "INSERT INTO link_notes (from_terminal, to_terminal, wake, word, about, reply_to) VALUES (?, ?, ?, ?, ?, ?)",
        (from, to, wake, ask.get("word"), ask.need("about")?, ask.get("reply-to")),
    )
    .map_err(sql("Sending the note"))?;
    Ok(format!("LINK note #{} {from} -> {to}{}", conn.last_insert_id(), if wake { "" } else { " (no knock)" }))
}

fn link_inbox(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let to = terminal(ask.need("to")?)?;
    let rows: Vec<(u64, String, String, Option<String>, String, Option<String>)> = conn
        .exec(
            "SELECT id, from_terminal, DATE_FORMAT(sent_at, '%Y-%m-%d %H:%i'), word, about, reply_to FROM link_notes WHERE to_terminal = ? AND read_at IS NULL ORDER BY id",
            (to,),
        )
        .map_err(sql("Reading the notes"))?;
    if rows.is_empty() {
        return Ok("LINK no notes waiting".into());
    }
    Ok(rows
        .into_iter()
        .map(|(id, from, at, word, about, reply)| {
            let mut out = format!("#{id} from {from} at {at}\nabout: {about}");
            if let Some(word) = word {
                out.push_str(&format!("\nword: {word}"));
            }
            if let Some(reply) = reply {
                out.push_str(&format!("\nreply-to: {reply}"));
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n\n"))
}

fn link_read(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let id: u64 = ask.need("id")?.trim_start_matches('#').parse().map_err(|_| "--id is a number, like 7.".to_string())?;
    conn.exec_drop("UPDATE link_notes SET read_at = NOW() WHERE id = ? AND read_at IS NULL", (id,)).map_err(sql("Marking the note"))?;
    Ok(if conn.affected_rows() == 0 { format!("LINK note #{id} was already read") } else { format!("LINK note #{id} read") })
}

fn link_here(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let name = terminal(ask.need("name")?)?;
    conn.exec_drop(
        "INSERT INTO link_terminals (name, detail, last_seen) VALUES (?, ?, NOW()) ON DUPLICATE KEY UPDATE detail = COALESCE(VALUES(detail), detail), last_seen = NOW()",
        (name, ask.get("detail")),
    )
    .map_err(sql("Signing in to the link"))?;
    let rows: Vec<(String, String)> =
        conn.query("SELECT name, DATE_FORMAT(last_seen, '%Y-%m-%d %H:%i') FROM link_terminals ORDER BY last_seen DESC").map_err(sql("Reading the link"))?;
    Ok(rows.into_iter().map(|(n, at)| format!("{n} · seen {at}")).collect::<Vec<_>>().join("\n"))
}

// ---------- export: the memory back out as markdown ----------

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "A folder could not be created.".to_string())?;
    }
    std::fs::write(path, text).map_err(|_| format!("{} could not be written.", path.display()))
}

/// A file name from a title: lowercase letters, digits and dashes.
pub fn file_slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "untitled".into() } else { out }
}

fn export(conn: &mut Conn, to: &Path) -> Result<String, String> {
    // 1. every file that was moved in comes back exactly as it was
    let originals: Vec<(String, String)> = conn.query("SELECT path, body FROM imported_files ORDER BY path").map_err(sql("Reading the kept files"))?;
    for (path, body) in &originals {
        write(&to.join(path), body)?;
    }

    // 2. diary entries written since then: appended to their day's file, MemoryCore's own shape
    let rows: Vec<(Option<String>, u32, String, String)> = conn
        .query("SELECT DATE_FORMAT(entry_date, '%Y-%m-%d'), session_no, title, body FROM diary_entries WHERE source_file IS NULL ORDER BY entry_date, session_no, id")
        .map_err(sql("Reading the diary"))?;
    let mut days: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (day, n, title, body) in rows {
        days.entry(day.unwrap_or_else(|| "earlier".into())).or_default().push(format!("## Session {n} - {title}\n{}\n", body.trim_end()));
    }
    for (day, sessions) in &days {
        let path = to.join("daily-diary").join("current").join(format!("{day}.md"));
        let text = match std::fs::read_to_string(&path) {
            Ok(there) => format!("{}\n\n{}", there.trim_end(), sessions.join("\n")),
            Err(_) => format!("# Daily Diary - {day}\n\n{}", sessions.join("\n")),
        };
        write(&path, &text)?;
    }
    let diary: Option<u64> = conn.query_first("SELECT COUNT(*) FROM diary_entries").map_err(sql("Counting the diary"))?;

    // 3. reminders as they are now
    let rows: Vec<(String, Option<String>, String, String)> = conn
        .query("SELECT title, DATE_FORMAT(due_date, '%Y-%m-%d'), detail, state FROM reminders ORDER BY position, id")
        .map_err(sql("Reading the reminders"))?;
    let reminders = rows.len();
    if reminders > 0 {
        let line = |(title, due, detail, _): &(String, Option<String>, String, String)| {
            let due = due.as_ref().map(|d| format!(" (by {d})")).unwrap_or_default();
            if detail.is_empty() { format!("- **{title}**{due}") } else { format!("- **{title}**{due}: {detail}") }
        };
        let open: Vec<String> = rows.iter().filter(|r| r.3 == "open").map(line).collect();
        let done: Vec<String> = rows.iter().filter(|r| r.3 != "open").map(line).collect();
        write(&to.join("main").join("reminders.md"), &format!("# Reminders\n\n## Open\n{}\n\n## Completed\n{}\n", open.join("\n"), done.join("\n")))?;
    }

    // 4. projects as they are now: the list, and each page at the path it came from
    let rows: Vec<(String, Option<String>, String, Option<String>, Option<String>, Option<String>, Option<String>)> = conn
        .query("SELECT name, status, shelf, summary, body, last_updated, source_file FROM projects ORDER BY shelf, id")
        .map_err(sql("Reading the projects"))?;
    let projects = rows.len();
    if projects > 0 {
        let mut list = String::from("# Project List\n");
        for shelf in ["active", "archived"] {
            let on: Vec<_> = rows.iter().filter(|r| (r.2 == "archived") == (shelf == "archived")).collect();
            if on.is_empty() {
                continue;
            }
            let heading = if shelf == "active" { "Active Projects" } else { "Archived Projects" };
            list.push_str(&format!("\n## {heading}\n| Pos | Project | Last Updated | Status |\n|-----|---------|--------------|--------|\n"));
            for (i, (name, status, _, summary, body, updated, source)) in on.iter().enumerate() {
                let page = source.clone().filter(|s| s.starts_with("projects/")).unwrap_or_else(|| format!("projects/{shelf}/{}.md", file_slug(name)));
                list.push_str(&format!(
                    "| {} | [{name}](./{}) | {} | {} |\n",
                    i + 1,
                    page.trim_start_matches("projects/"),
                    updated.clone().unwrap_or_default(),
                    status.clone().unwrap_or_default()
                ));
                let text = match body.as_ref().filter(|b| !b.trim().is_empty()) {
                    Some(body) => body.clone(),
                    None => format!("# {name}\n\n{}\n", summary.clone().unwrap_or_default()),
                };
                write(&to.join(&page), &text)?;
            }
        }
        write(&to.join("projects").join("project-list.md"), &list)?;
    }
    Ok(format!(
        "EXPORT {} diary page(s), {reminders} reminder(s), {projects} project(s), {} original file(s) -> {}",
        diary.unwrap_or(0),
        originals.len(),
        to.display()
    ))
}

// ---------- dump and restore: one statement per line ----------

pub fn dump(conn: &mut Conn, to: &Path) -> Result<String, String> {
    let tables: Vec<String> = conn.query("SHOW TABLES").map_err(sql("Reading the database"))?;
    let mut out = String::from("-- MemoryCore Castle memory dump: one statement per line. Restore with `castle-memory restore --from <this file>`.\nSET FOREIGN_KEY_CHECKS=0;\n");
    let mut total = 0usize;
    for table in &tables {
        let create: Option<(String, String)> = conn.query_first(format!("SHOW CREATE TABLE `{table}`")).map_err(sql("Reading a table"))?;
        let create = create.ok_or("A table could not be read.")?.1.replace(['\r', '\n'], " ");
        out.push_str(&format!("DROP TABLE IF EXISTS `{table}`;\n{create};\n"));
        let rows: Vec<Row> = conn.query(format!("SELECT * FROM `{table}`")).map_err(sql("Reading a table"))?;
        total += rows.len();
        for row in rows {
            let values: Vec<String> = row.unwrap().iter().map(|v: &Value| v.as_sql(false)).collect();
            out.push_str(&format!("INSERT INTO `{table}` VALUES ({});\n", values.join(",")));
        }
    }
    out.push_str("SET FOREIGN_KEY_CHECKS=1;\n");
    write(to, &out)?;
    Ok(format!("DUMP {} table(s), {total} row(s) -> {}", tables.len(), to.display()))
}

fn restore(conn: &mut Conn, from: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(from).map_err(|_| format!("{} could not be read.", from.display()))?;
    let mut done = 0usize;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("--")) {
        conn.query_drop(line.trim_end_matches(';')).map_err(|_| format!("The dump stopped at statement {}.", done + 1))?;
        done += 1;
    }
    Ok(format!("RESTORE {done} statement(s) from {}", from.display()))
}

// ---------- the program ----------

/// Runs the door for these arguments (everything after `--memory`); the exit code.
pub fn main(args: &[String]) -> i32 {
    let result = (|| {
        let ask = parse(args)?;
        if ask.words.is_empty() {
            return Err("castle-memory needs a verb: diary, reminder, project, link, export, dump, restore.".to_string());
        }
        let memory = find_memory(&ask, &std::env::current_dir().map_err(|_| "This folder could not be read.".to_string())?)?;
        let mut conn = memdb::open_for(&memory)?;
        run(&mut conn, &ask)
    })();
    match result {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(why) => {
            eprintln!("castle-memory: {why}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &[&str]) -> Vec<String> {
        line.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn words_and_flags_are_told_apart() {
        let ask = parse(&args(&["link", "send", "--from", "main", "--to", "story", "--about", "the garden", "--no-wake"])).unwrap();
        assert_eq!(ask.words, vec!["link", "send"]);
        assert_eq!(ask.get("about"), Some("the garden"));
        assert_eq!(ask.get("no-wake"), Some("true"));
        assert_eq!(ask.need("word").unwrap_err(), "--word is needed.");
        assert_eq!(parse(&args(&["diary", "add", "--title"])).unwrap_err(), "--title needs a value.");
    }

    #[test]
    fn dates_terminals_and_file_names_are_checked() {
        assert!(is_date("2026-11-10"));
        assert!(!is_date("2026-11-1") && !is_date("10-11-2026") && !is_date("2026/11/10"));
        assert!(terminal("main").is_ok() && terminal("room-2").is_ok());
        assert!(terminal("Main").is_err() && terminal("a b").is_err() && terminal("").is_err());
        assert_eq!(file_slug("Portfolio Site (v2)!"), "portfolio-site-v2");
        assert_eq!(file_slug("アリス"), "untitled");
    }

    #[test]
    fn the_memory_folder_is_found_upward_from_a_subfolder() {
        let root = std::env::temp_dir().join(format!("castle-door-{}", std::process::id()));
        let deep = root.join("adventures").join("world");
        std::fs::create_dir_all(&deep).unwrap();
        assert!(find_memory(&Ask::default(), &deep).is_err() || find_memory(&Ask::default(), &deep).unwrap() != root);
        std::fs::write(root.join(".env"), "MEMORY_DB_HOST=127.0.0.1\nMEMORY_DB_PORT=3307\nMEMORY_DB_NAME=lyra_memorycoredb\nMEMORY_DB_USER=castle_lyra\nMEMORY_DB_PASSWORD=x\n").unwrap();
        assert_eq!(find_memory(&Ask::default(), &deep).unwrap(), root);
        let named = parse(&args(&["diary", "list", "--memory-folder", "D:\\Mem"])).unwrap();
        assert_eq!(find_memory(&named, &deep).unwrap(), PathBuf::from("D:\\Mem"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Against a real server, on demand (CASTLE_TEST_DB="<port>:<admin>:<password>"): every verb
    /// once, then export, dump, wipe, restore, and the same rows are back.
    #[test]
    #[ignore]
    fn live_every_verb_then_dump_and_restore() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("set CASTLE_TEST_DB");
        let mut parts = spec.splitn(3, ':');
        let port: u16 = parts.next().unwrap().parse().unwrap();
        let (admin, admin_pw) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = memdb::provision(port, &admin, &admin_pw, &format!("Door Test {}", std::process::id())).unwrap();
        let mut conn = memdb::open(&access).unwrap();
        let folder = std::env::temp_dir().join(format!("castle-door-live-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let entry = folder.join("entry.md");
        std::fs::write(&entry, "We planned the garden beds; it's \"ours\".\nLine two 💜\n").unwrap();
        let mut go = |line: &[&str]| run(&mut conn, &parse(&args(line)).unwrap());

        assert_eq!(go(&["diary", "add", "--date", "2026-11-10", "--title", "The garden", "--file", entry.to_str().unwrap()]).unwrap(), "DIARY saved 2026-11-10 session 1: The garden");
        assert!(go(&["diary", "add", "--date", "2026-11-10", "--title", "Evening", "--body", "Tea."]).unwrap().contains("session 2"));
        assert!(go(&["diary", "add", "--date", "10 Nov", "--title", "x", "--body", "y"]).is_err());
        assert_eq!(go(&["diary", "list"]).unwrap(), "2026-11-10 #2 Evening\n2026-11-10 #1 The garden");
        assert_eq!(go(&["diary", "list", "--month", "2025-01"]).unwrap(), "DIARY empty");
        let shown = go(&["diary", "show", "--date", "2026-11-10"]).unwrap();
        assert!(shown.starts_with("# Daily Diary - 2026-11-10\n\n## Session 1 - The garden\nWe planned") && shown.contains("Line two 💜") && shown.ends_with("## Session 2 - Evening\nTea."));

        assert!(go(&["reminder", "add", "--title", "Call Rin", "--due", "2026-11-12", "--detail", "about the trip"]).unwrap().starts_with("REMINDER saved #1"));
        go(&["reminder", "add", "--title", "Water plants"]).unwrap();
        assert_eq!(go(&["reminder", "list"]).unwrap(), "#1 Call Rin (by 2026-11-12): about the trip\n#2 Water plants");
        assert_eq!(go(&["reminder", "done", "--id", "#1"]).unwrap(), "REMINDER #1 done");
        assert!(go(&["reminder", "done", "--id", "1"]).is_err());
        assert_eq!(go(&["reminder", "list"]).unwrap(), "#2 Water plants");

        go(&["project", "add", "--name", "Portfolio site", "--status", "building", "--summary", "My site"]).unwrap();
        go(&["project", "update", "--name", "Portfolio site", "--status", "live"]).unwrap();
        let shown = go(&["project", "show", "--name", "Portfolio site"]).unwrap();
        assert!(shown.contains("Status: live") && shown.contains("Summary: My site"));
        assert!(go(&["project", "list"]).unwrap().starts_with("[active] Portfolio site · live · "));

        assert_eq!(go(&["link", "send", "--from", "story", "--to", "main", "--about", "the tale paused", "--word", "save it"]).unwrap(), "LINK note #1 story -> main");
        let inbox = go(&["link", "inbox", "--to", "main"]).unwrap();
        assert!(inbox.starts_with("#1 from story at ") && inbox.ends_with("about: the tale paused\nword: save it"));
        assert_eq!(go(&["link", "read", "--id", "1"]).unwrap(), "LINK note #1 read");
        assert_eq!(go(&["link", "inbox", "--to", "main"]).unwrap(), "LINK no notes waiting");
        assert!(go(&["link", "here", "--name", "main"]).unwrap().starts_with("main · seen "));
        assert!(go(&["link", "send", "--from", "Bad Name", "--to", "main", "--about", "x"]).is_err());
        assert!(go(&["sing"]).is_err());

        let out = folder.join("out");
        assert!(go(&["export", "--to", out.to_str().unwrap()]).unwrap().starts_with("EXPORT 2 diary page(s), 2 reminder(s), 1 project(s), 0 original file(s)"));
        let day = std::fs::read_to_string(out.join("daily-diary/current/2026-11-10.md")).unwrap();
        assert!(day.starts_with("# Daily Diary - 2026-11-10\n\n## Session 1 - The garden\n") && day.contains("## Session 2 - Evening\nTea.\n"));
        assert!(std::fs::read_to_string(out.join("main/reminders.md")).unwrap().contains("## Open\n- **Water plants**\n\n## Completed\n- **Call Rin** (by 2026-11-12): about the trip"));
        assert!(std::fs::read_to_string(out.join("projects/project-list.md")).unwrap().contains("| 1 | [Portfolio site](./active/portfolio-site.md) |"));
        // the export reads back through Castle's own file readers
        assert_eq!(crate::memory::newest_diary(&out).unwrap().title, "Evening");
        assert_eq!(crate::memory::reminders(&out)[0].title, "Water plants");
        assert_eq!(crate::memory::projects(&out)[0].name, "Portfolio site");

        let file = folder.join("backup").join("memory.sql");
        assert!(go(&["dump", "--to", file.to_str().unwrap()]).unwrap().starts_with("DUMP 7 table(s), "));
        let before = go(&["diary", "show", "--date", "2026-11-10"]).unwrap();
        drop(go);
        conn.query_drop("DROP TABLE diary_entries, reminders, projects, link_notes, link_terminals, imported_files, schema_migrations").unwrap();
        let mut go = |line: &[&str]| run(&mut conn, &parse(&args(line)).unwrap());
        assert!(go(&["diary", "list"]).is_err());
        assert!(go(&["restore", "--from", file.to_str().unwrap()]).unwrap().starts_with("RESTORE "));
        assert_eq!(go(&["diary", "show", "--date", "2026-11-10"]).unwrap(), before);
        assert_eq!(go(&["reminder", "list"]).unwrap(), "#2 Water plants");
        drop(go);
        assert!(memdb::migrate(&mut conn).unwrap().is_empty());

        std::fs::remove_dir_all(&folder).unwrap();
        let mut root = mysql::Conn::new(mysql::Opts::from(
            mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(admin)).pass(Some(admin_pw)).prefer_socket(false),
        ))
        .unwrap();
        root.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER '{}'@'{host}'", access.user)).unwrap();
        }
    }
}
