//! The DEV tab's Database panel speaks only through these. Every command names a DEV tab by its
//! slot, never a folder, so nothing outside the open projects is ever read; the login is looked up
//! here (`projectdb_login`) and never travels to the page.

use crate::projectdb::{self, Changed, Column, Ctx, Held, Login, Rows, Saved};
use crate::projectdb_login::{self, Connection, Form};
use crate::settings::SettingsStore;
use serde::Deserialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Which side of which DEV tab, and the database chosen in the panel ('' = the login's own).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub slot: usize,
    /// "local" or "production"
    pub side: String,
    #[serde(default)]
    pub database: String,
}

/// The folder of an open DEV tab.
fn folder_of(store: &SettingsStore, slot: usize) -> Result<String, String> {
    store
        .get()
        .dev_tabs
        .get(slot)
        .cloned()
        .flatten()
        .ok_or_else(|| "That DEV tab has no project open.".to_string())
}

fn side_of(side: &str) -> Result<&'static str, String> {
    match side {
        "local" => Ok("local"),
        "production" => Ok("production"),
        _ => Err("Local or Production only.".into()),
    }
}

/// The login for a target and the log's context.
fn open(store: &SettingsStore, target: &Target) -> Result<(Login, Ctx), String> {
    let folder = folder_of(store, target.slot)?;
    let settings = store.get();
    let login = projectdb_login::login(&settings, &folder, side_of(&target.side)?)?.on(&target.database);
    Ok((login, Ctx { folder: store.folder(), project: crate::dev_tabs::name_of(&folder) }))
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| "The database work could not finish.".to_string())?
}

/// Undoes held changes nobody kept, every 30 seconds, while Castle runs.
pub fn start_sweeper(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        projectdb::sweep(&app.state::<Held>());
    });
}

// ---------- the connection form ----------

/// Both sides of a tab's project as the panel shows them - never a password.
#[tauri::command]
pub async fn projectdb_connection(slot: usize, store: State<'_, SettingsStore>) -> Result<Connection, String> {
    let folder = folder_of(&store, slot)?;
    let settings = store.get();
    blocking(move || Ok(projectdb_login::connection(&settings, &folder))).await
}

/// Saves one side's login from the form.
#[tauri::command]
pub fn projectdb_save(slot: usize, side: String, form: Form, store: State<SettingsStore>) -> Result<(), String> {
    let folder = folder_of(&store, slot)?;
    projectdb_login::save(&store, &folder, side_of(&side)?, &form)
}

/// Forgets one side's login, password and certificate.
#[tauri::command]
pub fn projectdb_forget(slot: usize, side: String, store: State<SettingsStore>) -> Result<(), String> {
    let folder = folder_of(&store, slot)?;
    projectdb_login::forget(&store, &folder, side_of(&side)?)
}

/// Opens and closes a connection with the saved login: the server's version, or why it failed.
#[tauri::command]
pub async fn projectdb_test(target: Target, store: State<'_, SettingsStore>) -> Result<String, String> {
    let (login, _) = open(&store, &target)?;
    blocking(move || projectdb::test(&login)).await
}

/// The CA certificate file the member picks for Production, or None when they cancelled.
#[tauri::command]
pub async fn projectdb_pick_ca(app: AppHandle) -> Result<Option<String>, String> {
    blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("The cluster's CA certificate")
            .add_filter("Certificate", &["crt", "pem", "cer"])
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.display().to_string()))
    })
    .await
}

// ---------- reading ----------

#[tauri::command]
pub async fn projectdb_databases(target: Target, store: State<'_, SettingsStore>) -> Result<Vec<String>, String> {
    let (login, _) = open(&store, &target)?;
    blocking(move || projectdb::databases(&login)).await
}

#[tauri::command]
pub async fn projectdb_tables(target: Target, store: State<'_, SettingsStore>) -> Result<Vec<String>, String> {
    let (login, _) = open(&store, &target)?;
    blocking(move || projectdb::tables(&login)).await
}

#[tauri::command]
pub async fn projectdb_table_rows(target: Target, store: State<'_, SettingsStore>) -> Result<std::collections::BTreeMap<String, u64>, String> {
    let (login, _) = open(&store, &target)?;
    blocking(move || projectdb::table_rows(&login)).await
}

#[tauri::command]
pub async fn projectdb_columns(target: Target, table: String, store: State<'_, SettingsStore>) -> Result<Vec<Column>, String> {
    let (login, _) = open(&store, &target)?;
    blocking(move || projectdb::columns(&login, &table)).await
}

#[tauri::command]
pub async fn projectdb_look(target: Target, sql_text: String, store: State<'_, SettingsStore>) -> Result<Rows, String> {
    let (login, ctx) = open(&store, &target)?;
    blocking(move || projectdb::look(&login, &sql_text, &ctx)).await
}

// ---------- changing ----------

/// A change: on Production a write is held for Keep or Undo; a shape change needs `confirmed`.
#[tauri::command]
pub async fn projectdb_change(app: AppHandle, target: Target, sql_text: String, confirmed: bool, store: State<'_, SettingsStore>) -> Result<Changed, String> {
    let (login, ctx) = open(&store, &target)?;
    blocking(move || projectdb::change(&app.state::<Held>(), &login, &sql_text, confirmed, &ctx)).await
}

/// Keep (commit) or Undo (rollback) of a held change.
#[tauri::command]
pub async fn projectdb_finish(app: AppHandle, id: u64, keep: bool) -> Result<(), String> {
    blocking(move || projectdb::finish(&app.state::<Held>(), id, keep)).await
}

// ---------- files ----------

fn save_dialog(app: &AppHandle, title: &str, name: &str, ext: &str) -> Option<std::path::PathBuf> {
    app.dialog().file().set_title(title).set_file_name(name).add_filter(ext, &[ext]).blocking_save_file().and_then(|p| p.into_path().ok())
}

/// The grid to a CSV file the member chooses; the path written, or None when they cancelled.
#[tauri::command]
pub async fn projectdb_export(app: AppHandle, rows: Rows, name: String) -> Result<Option<String>, String> {
    blocking(move || {
        let Some(path) = save_dialog(&app, "Export the rows", &format!("{name}.csv"), "csv") else { return Ok(None) };
        projectdb::export_csv(&rows, &path)?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

/// A dump of one table or the whole database to a .sql file the member chooses -> (path, rows).
#[tauri::command]
pub async fn projectdb_dump(app: AppHandle, target: Target, table: Option<String>, store: State<'_, SettingsStore>) -> Result<Option<(String, u64)>, String> {
    let (login, ctx) = open(&store, &target)?;
    blocking(move || {
        let base = if login.database.is_empty() { "database".to_string() } else { login.database.clone() };
        let name = format!("{base}-{}", table.as_deref().unwrap_or("all"));
        let Some(path) = save_dialog(&app, "Dump to a file", &format!("{name}.sql"), "sql") else { return Ok(None) };
        let rows = projectdb::dump(&login, table.as_deref(), &path, &ctx)?;
        Ok(Some((path.display().to_string(), rows)))
    })
    .await
}

/// A .sql or .csv file the member picks, imported -> (file name, statements or rows), None when cancelled.
#[tauri::command]
pub async fn projectdb_import(app: AppHandle, target: Target, table: Option<String>, unlocked: bool, store: State<'_, SettingsStore>) -> Result<Option<(String, u64)>, String> {
    let (login, ctx) = open(&store, &target)?;
    blocking(move || {
        let Some(path) = app.dialog().file().set_title("Import a file").add_filter("SQL or CSV", &["sql", "csv"]).blocking_pick_file().and_then(|p| p.into_path().ok()) else {
            return Ok(None);
        };
        let n = projectdb::import(&login, &path, table.as_deref(), unlocked, &ctx)?;
        Ok(Some((path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), n)))
    })
    .await
}

// ---------- saved queries ----------

#[tauri::command]
pub fn projectdb_saved(slot: usize, store: State<SettingsStore>) -> Result<Vec<Saved>, String> {
    let folder = folder_of(&store, slot)?;
    Ok(projectdb::saved(&store.folder(), &crate::dev_tabs::same_folder_key(&folder)))
}

#[tauri::command]
pub fn projectdb_saved_set(slot: usize, list: Vec<Saved>, store: State<SettingsStore>) -> Result<(), String> {
    let folder = folder_of(&store, slot)?;
    projectdb::set_saved(&store.folder(), &crate::dev_tabs::same_folder_key(&folder), list)
}
