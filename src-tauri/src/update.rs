//! The update door. Only an official build (made by scripts/release.ps1 with CASTLE_OFFICIAL set
//! at build time) checks for updates: the signed latest.json on the project's GitHub release.
//! A member's own build never updates itself, so it can never overwrite their changes. An update
//! replaces the app only - the memory folder, settings, themes and keys are never touched.
//! The page saves the diary first (it asks the companion), then calls update_install.

use serde::Serialize;
use tauri::AppHandle;
#[cfg(feature = "official")]
use tauri_plugin_updater::UpdaterExt;

/// True for an official release build.
pub fn official() -> bool {
    option_env!("CASTLE_OFFICIAL").is_some_and(|v| v == "1")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub official: bool,
    pub version: String,
    /// the newer version, when there is one
    pub available: Option<String>,
    pub notes: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> UpdateStatus {
    let version = env!("CARGO_PKG_VERSION").to_string();
    if !official() {
        return UpdateStatus {
            official: false,
            version,
            available: None,
            notes: None,
            message: "Updates: your own build - pull new code with git (see MAKE-IT-YOURS.md).".into(),
        };
    }
    check(&app, version).await
}

#[cfg(feature = "official")]
async fn check(app: &AppHandle, version: String) -> UpdateStatus {
    let status = |available: Option<String>, notes: Option<String>, message: String| UpdateStatus { official: true, version: version.clone(), available, notes, message };
    let Ok(updater) = app.updater() else { return status(None, None, "Updates could not be checked.".into()) };
    match updater.check().await {
        Ok(Some(update)) => status(Some(update.version.clone()), update.body.clone(), format!("Update ready: {}", update.version)),
        Ok(None) => status(None, None, "Castle is up to date.".into()),
        Err(_) => status(None, None, "Updates could not be checked right now.".into()),
    }
}

#[cfg(not(feature = "official"))]
async fn check(_app: &AppHandle, version: String) -> UpdateStatus {
    UpdateStatus { official: true, version, available: None, notes: None, message: "This build was made without the updater.".into() }
}

/// Downloads, checks the signature, installs and restarts. The page has saved the diary first.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    if !official() {
        return Err("Your own build does not update itself.".into());
    }
    install(&app).await
}

#[cfg(feature = "official")]
async fn install(app: &AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|_| "Updates could not be started.".to_string())?;
    let update = updater.check().await.map_err(|_| "Updates could not be checked.".to_string())?.ok_or("Castle is already up to date.")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| "The update could not be installed. Castle stays as it is.".to_string())?;
    app.restart();
}

#[cfg(not(feature = "official"))]
async fn install(_app: &AppHandle) -> Result<(), String> {
    Err("This build was made without the updater.".into())
}

/// The newest change time of the diary (seconds since 1970), so the page can wait for Save diary.
#[tauri::command]
pub fn diary_stamp(store: tauri::State<crate::settings::SettingsStore>) -> u64 {
    let Some(folder) = store.get().memory_folder else { return 0 };
    crate::memory::diary_files(std::path::Path::new(&folder))
        .iter()
        .filter_map(|p| p.metadata().and_then(|m| m.modified()).ok())
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_test_build_is_never_official() {
        assert!(!super::official());
    }
}
