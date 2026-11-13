//! The door's plan verbs (`castle-memory plan copy | status | next | mark | show`), kept apart
//! from door.rs so the door stays readable: an approved plan from `~/.claude/plans` becomes rows in
//! the member's database, walked one box at a time from the project folder.

use crate::door::Ask;
use crate::plans;
use mysql::Conn;
use std::path::{Path, PathBuf};

// ---------- plans for a project folder ----------

/// The project folder: `--folder`, else the folder the door was started in; made whole.
pub(crate) fn project_folder(ask: &Ask) -> Result<String, String> {
    let folder = match ask.get("folder").map(str::trim).filter(|f| !f.is_empty()) {
        Some(folder) => PathBuf::from(folder),
        None => std::env::current_dir().map_err(|_| "This folder could not be read.".to_string())?,
    };
    let folder = std::path::absolute(&folder).unwrap_or(folder);
    Ok(folder.display().to_string())
}

/// The newest `*.md` in a folder.
pub fn newest_md(folder: &Path) -> Option<PathBuf> {
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md")))
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
}

/// The plan the CLI approved last: the newest `*.md` in `%USERPROFILE%\.claude\plans`.
pub(crate) fn newest_plan() -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).ok_or("Your home folder could not be found. Name the plan with --plan <file>.")?;
    newest_md(&PathBuf::from(home).join(".claude").join("plans")).ok_or_else(|| "No plan was found in ~/.claude/plans. Name one with --plan <file>.".to_string())
}

/// The box in turn, and what is left.
pub(crate) fn next_line(conn: &mut Conn, folder: &str) -> Result<String, String> {
    let left = plans::left(conn, folder)?;
    // carried and flagged boxes are the member's to decide: said, never walked
    let waiting: Vec<String> = [(left.carried, "carried"), (left.flagged, "flagged")].iter().filter(|(n, _)| *n > 0).map(|(n, what)| format!("{n} {what}")).collect();
    let waiting = if waiting.is_empty() { String::new() } else { format!(" ({} waiting for you)", waiting.join(", ")) };
    Ok(match plans::next_item(conn, folder)? {
        Some(next) => format!("NEXT #{} item {} [{}] {}\nLEFT {} open{waiting}", next.plan_id, next.item.ordinal, next.item.kind, next.item.text, left.open),
        None => format!("NEXT nothing - every open box is walked{waiting}"),
    })
}

pub(crate) fn plan_copy(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let file = match ask.get("plan") {
        Some(file) => PathBuf::from(file),
        None => newest_plan()?,
    };
    let markdown = std::fs::read_to_string(&file).map_err(|_| format!("The plan {} could not be read.", file.display()))?;
    let folder = project_folder(ask)?;
    let (id, boxes, duplicate) = plans::copy_plan(conn, &folder, &markdown, Some(&file.display().to_string()))?;
    if duplicate {
        return Ok(format!("PLAN #{id} is already here ({boxes} items): the same plan was copied before"));
    }
    let parsed = plans::parse_plan(&markdown);
    let phase = parsed.phase().map(|p| format!("{p} · ")).unwrap_or_default();
    let title = if parsed.title.is_empty() { "untitled".to_string() } else { parsed.title };
    Ok(format!("PLAN copied #{id} · {phase}{title} · {boxes} items\n{}", next_line(conn, &folder)?))
}

pub(crate) fn plan_status(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let folder = project_folder(ask)?;
    let open = plans::open_plans(conn, &folder)?;
    if open.is_empty() {
        return Ok("PLAN none open here - copy an approved plan with `castle-memory plan copy`".into());
    }
    // oldest first: the order they are walked in
    let mut lines: Vec<String> = open
        .iter()
        .rev()
        .map(|plan| {
            let boxes: Vec<_> = plan.items.iter().filter(|i| i.kind != "phase").collect();
            let done = boxes.iter().filter(|i| i.state == "done").count();
            let phase = plan.phase.as_ref().map(|p| format!("{p} · ")).unwrap_or_default();
            format!("PLAN #{} · {phase}{} · {done}/{} done", plan.id, plan.title, boxes.len())
        })
        .collect();
    lines.push(next_line(conn, &folder)?);
    Ok(lines.join("\n"))
}

/// A `--name 3` flag as a number (a leading # is fine).
pub(crate) fn number<T: std::str::FromStr>(ask: &Ask, name: &str) -> Result<Option<T>, String> {
    ask.get(name).map(|v| v.trim().trim_start_matches('#').parse().map_err(|_| format!("--{name} is a number, like 3."))).transpose()
}

pub(crate) fn plan_mark(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let folder = project_folder(ask)?;
    let which = match (number::<u32>(ask, "item")?, ask.get("next")) {
        (Some(_), Some(_)) => return Err("Name one box: --next or --item N.".into()),
        (Some(n), None) => plans::Which::Ordinal(n),
        (None, _) => plans::Which::Next,
    };
    let state = ask.get("state").unwrap_or("done");
    let marked = plans::mark(conn, &folder, number(ask, "id")?, which, state, ask.get("note"))?;
    let mut out = format!("MARKED #{} item {} {}: {}", marked.plan_id, marked.ordinal, marked.state, plans::first_line(&marked.text));
    if marked.plan_done {
        out.push_str(&format!("\nPLAN #{} done - every box is ticked", marked.plan_id));
    }
    out.push('\n');
    out.push_str(&next_line(conn, &folder)?);
    Ok(out)
}

pub(crate) fn plan_show(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    let id = match number(ask, "id")? {
        Some(id) => id,
        None => plans::current_plan(conn, &project_folder(ask)?)?.ok_or("PLAN none open here - name one with --id N.")?,
    };
    Ok(plans::plan_body(conn, id)?.trim_end().to_string())
}
