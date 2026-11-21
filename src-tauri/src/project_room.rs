//! The PROJECTS room's controls: where a project is (active, paused, archived or recently deleted)
//! and moving it. The companion moves projects the same way through `castle-memory project pause |
//! resume | archive | restore | delete` (door.rs); both end in `door::move_project`.
//!
//! Sub-projects and tasks (CASTLE-6): the open project's parent and sub-projects, its tasks
//! (ticked, added, removed and put in order by the member here, by the companion through
//! `castle-memory task`), and the one-time split of grouped names ("Castle · X").

use crate::door::{self, Shelf};
use crate::project_tree;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectState {
    pub name: String,
    pub shelf: String,
    pub note: Option<String>,
    /// For a deleted project: whole days left before it is removed for good.
    pub days_left: Option<i64>,
    /// its parent, for a sub-project
    pub parent: Option<Kin>,
    /// its sub-projects, on any shelf
    pub kids: Vec<Kin>,
    /// how many sub-projects stand on its shelf with it (the ones a move asks about)
    pub kids_beside: usize,
    /// whether it can take sub-projects (it is not one itself)
    pub can_have_kids: bool,
    pub tasks: Vec<project_tree::Task>,
    /// tasks done and all, a parent's counting its sub-projects'
    pub done: u32,
    pub total: u32,
}

/// A project next to the open one: its page, name, shelf, status and task progress.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Kin {
    pub file: String,
    pub name: String,
    pub shelf: String,
    pub status: Option<String>,
    pub done: u32,
    pub total: u32,
}

fn memory(store: &crate::settings::SettingsStore) -> Result<PathBuf, String> {
    store.get().memory_folder.map(PathBuf::from).ok_or_else(|| "No memory folder.".to_string())
}

fn project_id(file: &str) -> Result<u64, String> {
    match crate::store::page_of(file) {
        Some(("project", id)) => Ok(id),
        _ => Err("Only projects in the memory database can be moved.".into()),
    }
}

/// Where the project behind a reader page (`db/project/<id>.md`) is.
#[tauri::command]
pub fn project_state(file: String, store: State<crate::settings::SettingsStore>) -> Result<ProjectState, String> {
    let id = project_id(&file)?;
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    use mysql::prelude::Queryable;
    let row: Option<(String, String, Option<String>, Option<i64>)> = conn
        .exec_first(
            "SELECT name, shelf, shelf_note, CASE WHEN shelf = 'deleted' THEN GREATEST(0, ? - DATEDIFF(NOW(), shelved_at)) END FROM projects WHERE id = ?",
            (door::KEEP_DELETED_DAYS, id),
        )
        .map_err(|_| "The project could not be read.".to_string())?;
    let (name, shelf, note, days_left) = row.ok_or("That project is no longer there.")?;
    let shelf = Shelf::named(&shelf).unwrap_or(Shelf::Active).word().to_string();
    let nodes = project_tree::nodes(&mut conn);
    let progress = project_tree::rolled_progress(&nodes, &project_tree::own_progress(&mut conn));
    let kin = |n: &project_tree::Node| {
        let (done, total) = progress.get(&n.id).copied().unwrap_or((0, 0));
        Kin { file: format!("db/project/{}.md", n.id), name: n.name.clone(), shelf: Shelf::named(&n.shelf).unwrap_or(Shelf::Active).word().into(), status: n.status.clone(), done, total }
    };
    let me = nodes.iter().find(|n| n.id == id);
    let parent = me.and_then(|m| m.parent).and_then(|p| nodes.iter().find(|n| n.id == p)).map(kin);
    let kids: Vec<Kin> = nodes.iter().filter(|n| n.parent == Some(id)).map(kin).collect();
    let kids_beside = project_tree::kids_beside(&mut conn, id)?.len();
    let (done, total) = progress.get(&id).copied().unwrap_or((0, 0));
    Ok(ProjectState {
        name,
        shelf,
        note: note.filter(|n| !n.is_empty()),
        days_left,
        can_have_kids: parent.is_none(),
        parent,
        kids,
        kids_beside,
        tasks: project_tree::tasks(&mut conn, id)?,
        done,
        total,
    })
}

/// Moves the project behind a reader page to `to` (active | paused | archived | deleted); with
/// `with_kids`, the sub-projects on its shelf go with it.
#[tauri::command]
pub fn project_shelve(file: String, to: String, note: Option<String>, with_kids: Option<bool>, store: State<crate::settings::SettingsStore>) -> Result<String, String> {
    let id = project_id(&file)?;
    let to = Shelf::named(&to).ok_or("That is not a shelf.")?;
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::move_with_kids(&mut conn, id, to, note.as_deref(), with_kids.unwrap_or(false))
}

/// Adds a task to the project behind a reader page; its id.
#[tauri::command]
pub fn project_task_add(file: String, title: String, due: Option<String>, store: State<crate::settings::SettingsStore>) -> Result<u64, String> {
    let id = project_id(&file)?;
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::task_add(&mut conn, id, &title, due.as_deref(), false)
}

/// Ticks a task, or takes its tick off.
#[tauri::command]
pub fn project_task_done(id: u64, done: bool, store: State<crate::settings::SettingsStore>) -> Result<(), String> {
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::task_set_done(&mut conn, id, done).map(|_| ())
}

#[tauri::command]
pub fn project_task_remove(id: u64, store: State<crate::settings::SettingsStore>) -> Result<(), String> {
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::task_remove(&mut conn, id).map(|_| ())
}

/// Puts the tasks of the project behind a reader page in this order (task ids, first to last).
#[tauri::command]
pub fn project_task_order(file: String, ids: Vec<u64>, store: State<crate::settings::SettingsStore>) -> Result<(), String> {
    let id = project_id(&file)?;
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::task_order(&mut conn, id, &ids)
}

/// The grouped names ("Castle · X") that could become sub-projects.
#[tauri::command]
pub fn project_split_proposals(store: State<crate::settings::SettingsStore>) -> Result<Vec<project_tree::Split>, String> {
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    Ok(project_tree::proposals(&project_tree::nodes(&mut conn)))
}

/// Turns the ticked projects (row ids) into sub-projects of the parent their name gives.
#[tauri::command]
pub fn project_split_apply(ids: Vec<u64>, store: State<crate::settings::SettingsStore>) -> Result<String, String> {
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    project_tree::apply_split(&mut conn, &ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_database_project_pages_can_be_moved() {
        assert_eq!(project_id("db/project/12.md"), Ok(12));
        assert!(project_id("db/diary/12.md").is_err());
        assert!(project_id("projects/active/site.md").is_err());
    }
}
