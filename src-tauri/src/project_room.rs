//! The PROJECTS room's controls: where a project is (active, paused, archived or recently deleted)
//! and moving it. The companion moves projects the same way through `castle-memory project pause |
//! resume | archive | restore | delete` (door.rs); both end in `door::move_project`.

use crate::door::{self, Shelf};
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
    Ok(ProjectState { name, shelf, note: note.filter(|n| !n.is_empty()), days_left })
}

/// Moves the project behind a reader page to `to` (active | paused | archived | deleted).
#[tauri::command]
pub fn project_shelve(file: String, to: String, note: Option<String>, store: State<crate::settings::SettingsStore>) -> Result<String, String> {
    let id = project_id(&file)?;
    let to = Shelf::named(&to).ok_or("That is not a shelf.")?;
    let mut conn = crate::memdb::open_for(&memory(&store)?)?;
    door::move_project(&mut conn, id, to, note.as_deref())
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
