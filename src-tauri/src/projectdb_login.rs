//! Where a DEV project's database logins come from: what the member filled in the Database panel
//! (kept in Castle's settings, the password in Windows Credential Manager / the Keychain), or, for
//! Local, the project's own `.env` / `application.properties`. Nothing is ever written into the
//! project folder, and no password ever reaches the page.

use crate::dev_tabs::{name_of, same_folder_key};
use crate::projectdb::{env_login, EnvLogin, Login};
use crate::settings::{self, DbLogin, ProjectDb, SettingsStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The files a project's local login is read from, first found wins.
const ENV_FILES: [&str; 5] = [".env", "backend/.env", "api/.env", "src/main/resources/application.properties", "backend/src/main/resources/application.properties"];

/// A short, stable id for a project folder: the first 16 hex characters of its folder key's sha256.
/// `D:\Shop` and `d:/shop/` are the same project on Windows.
pub fn project_id(folder: &str) -> String {
    let digest = Sha256::digest(same_folder_key(folder).as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// The project's own login, read from its files: the login and the file it came from.
pub fn found_in_files(folder: &Path) -> Option<(EnvLogin, &'static str)> {
    ENV_FILES.iter().find_map(|name| {
        let login = std::fs::read_to_string(folder.join(name)).ok().as_deref().and_then(env_login)?;
        Some((login, if name.ends_with(".properties") { "application.properties" } else { ".env" }))
    })
}

/// One side as the form and the connection line show it: never a password.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideView {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// "saved", ".env", "application.properties" or "none"
    pub from: String,
    pub has_password: bool,
    /// Production: a CA certificate is kept.
    pub ca: bool,
    /// A plain reason it cannot open yet, or ''.
    pub blocked: String,
}

/// Both sides of a project, for the Database panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub project: String,
    pub local: SideView,
    pub production: SideView,
}

/// Local: what the member saved wins; otherwise the project's own files; otherwise nothing yet.
/// `saved_password` is the keyring's; a login read from the files brings its own.
pub fn local_view(saved: Option<&DbLogin>, files: Option<&(EnvLogin, &str)>, saved_password: bool) -> SideView {
    if let Some(s) = saved {
        return SideView {
            host: s.host.clone(),
            port: s.port,
            database: s.database.clone(),
            user: s.user.clone(),
            from: "saved".into(),
            has_password: saved_password,
            ca: false,
            blocked: String::new(),
        };
    }
    if let Some((login, from)) = files {
        return SideView {
            host: login.host.clone(),
            port: login.port,
            database: login.database.clone(),
            user: login.user.clone(),
            from: (*from).into(),
            has_password: !login.password.is_empty() || saved_password,
            ca: false,
            blocked: String::new(),
        };
    }
    SideView {
        host: "127.0.0.1".into(),
        port: 3306,
        user: "root".into(),
        from: "none".into(),
        blocked: "No database login was found in this project - fill in Local and press Save.".into(),
        ..SideView::default()
    }
}

/// Production: only what the member saved, and only once host, user and password are there.
pub fn production_view(saved: Option<&DbLogin>, saved_password: bool) -> SideView {
    let Some(s) = saved else {
        return SideView { port: 25060, from: "none".into(), blocked: "Fill in Production once and press Save - it stays on this computer.".into(), ..SideView::default() };
    };
    let blocked = if s.host.trim().is_empty() || s.user.trim().is_empty() {
        "Production needs a host and a user - open Edit connection.".to_string()
    } else if !saved_password {
        "Production needs its password - open Edit connection.".to_string()
    } else {
        String::new()
    };
    SideView {
        host: s.host.clone(),
        port: s.port,
        database: s.database.clone(),
        user: s.user.clone(),
        from: "saved".into(),
        has_password: saved_password,
        ca: s.ca.as_deref().is_some_and(|c| Path::new(c).is_file()),
        blocked,
    }
}

fn saved_of(settings: &settings::Settings, folder: &str) -> ProjectDb {
    settings.project_databases.get(&same_folder_key(folder)).cloned().unwrap_or_default()
}

/// Both sides of a project, as the panel shows them.
pub fn connection(settings: &settings::Settings, folder: &str) -> Connection {
    let saved = saved_of(settings, folder);
    let id = project_id(folder);
    let files = found_in_files(Path::new(folder));
    Connection {
        project: name_of(folder),
        local: local_view(saved.local.as_ref(), files.as_ref(), crate::key::load_db_password("local", &id).is_some()),
        production: production_view(saved.production.as_ref(), crate::key::load_db_password("production", &id).is_some()),
    }
}

/// The login one side opens with, or the plain reason it cannot open yet.
pub fn login(settings: &settings::Settings, folder: &str, side: &str) -> Result<Login, String> {
    let saved = saved_of(settings, folder);
    let id = project_id(folder);
    if side == "production" {
        let password = crate::key::load_db_password("production", &id);
        let view = production_view(saved.production.as_ref(), password.is_some());
        if !view.blocked.is_empty() {
            return Err(view.blocked);
        }
        let p = saved.production.unwrap_or_default();
        return Ok(Login {
            host: p.host,
            port: p.port,
            user: p.user,
            password: password.unwrap_or_default(),
            database: p.database,
            ca: p.ca.map(PathBuf::from).filter(|c| c.is_file()),
            production: true,
        });
    }
    let saved_password = crate::key::load_db_password("local", &id);
    if let Some(l) = saved.local {
        return Ok(Login { host: l.host, port: l.port, user: l.user, password: saved_password.unwrap_or_default(), database: l.database, ca: None, production: false });
    }
    let (found, _) = found_in_files(Path::new(folder)).ok_or("No database login was found in this project - fill in Local and press Save.")?;
    Ok(Login {
        host: found.host,
        port: found.port,
        user: found.user,
        password: saved_password.unwrap_or(found.password),
        database: found.database,
        ca: None,
        production: false,
    })
}

/// What the form sends for one side.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// None keeps the password already kept; "" removes it.
    #[serde(default)]
    pub password: Option<String>,
    /// A CA certificate file the member chose (Production): Castle keeps its own copy.
    #[serde(default)]
    pub ca_path: Option<String>,
}

/// The form's fields checked and trimmed.
pub fn checked(form: &Form) -> Result<DbLogin, String> {
    let host = form.host.trim();
    let user = form.user.trim();
    if host.is_empty() || host.contains(char::is_whitespace) {
        return Err("Fill in the host (a name or an address, no spaces).".into());
    }
    if user.is_empty() {
        return Err("Fill in the user.".into());
    }
    if form.port == 0 {
        return Err("The port is a number such as 3306.".into());
    }
    Ok(DbLogin { host: host.into(), port: form.port, database: form.database.trim().into(), user: user.into(), ca: None })
}

/// Saves one side's login: the fields to settings, the password to the keyring, a chosen CA as
/// Castle's own copy.
pub fn save(store: &SettingsStore, folder: &str, side: &str, form: &Form) -> Result<(), String> {
    let production = side == "production";
    let mut login = checked(form)?;
    let id = project_id(folder);
    let key = same_folder_key(folder);
    let mut next = store.get();
    let mut entry = next.project_databases.get(&key).cloned().unwrap_or_default();
    let before = if production { entry.production.clone() } else { entry.local.clone() };
    if production {
        login.ca = before.and_then(|b| b.ca);
        if let Some(path) = form.ca_path.as_deref().filter(|p| !p.trim().is_empty()) {
            login.ca = Some(keep_ca(store, &id, Path::new(path))?);
        }
    }
    match form.password.as_deref() {
        Some("") => crate::key::forget_db_password(side, &id)?,
        Some(password) => crate::key::save_db_password(side, &id, password)?,
        None => {}
    }
    if production {
        entry.production = Some(login);
    } else {
        entry.local = Some(login);
    }
    next.project_databases.insert(key, entry);
    write(store, next)
}

/// Forgets one side: its fields, its password and its CA copy. Local then reads the `.env` again.
pub fn forget(store: &SettingsStore, folder: &str, side: &str) -> Result<(), String> {
    let id = project_id(folder);
    let key = same_folder_key(folder);
    let mut next = store.get();
    let mut entry = next.project_databases.get(&key).cloned().unwrap_or_default();
    let gone = if side == "production" { entry.production.take() } else { entry.local.take() };
    if let Some(ca) = gone.and_then(|g| g.ca) {
        let _ = std::fs::remove_file(ca);
    }
    crate::key::forget_db_password(side, &id)?;
    if entry == ProjectDb::default() {
        next.project_databases.remove(&key);
    } else {
        next.project_databases.insert(key, entry);
    }
    write(store, next)
}

/// Copies a CA certificate into `<settings folder>/certs/<id>.crt`, so moving the original never
/// breaks the connection. Only a small PEM file is taken.
fn keep_ca(store: &SettingsStore, id: &str, source: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(source).map_err(|_| "The CA certificate could not be read.".to_string())?;
    if text.len() > 64 * 1024 || !text.contains("-----BEGIN CERTIFICATE-----") {
        return Err("That is not a CA certificate (a .crt / .pem file starting with BEGIN CERTIFICATE).".into());
    }
    let folder = store.folder().join("certs");
    std::fs::create_dir_all(&folder).map_err(|_| "The certificate could not be kept.".to_string())?;
    let target = folder.join(format!("{id}.crt"));
    std::fs::write(&target, text).map_err(|_| "The certificate could not be kept.".to_string())?;
    Ok(target.display().to_string())
}

fn write(store: &SettingsStore, next: settings::Settings) -> Result<(), String> {
    settings::save(&store.path, &next).map_err(|_| "The database login could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(host: &str, db: &str, password: &str) -> (EnvLogin, &'static str) {
        (EnvLogin { host: host.into(), port: 3306, database: db.into(), user: "root".into(), password: password.into() }, ".env")
    }

    #[test]
    fn the_id_is_stable_for_one_folder_however_it_is_written() {
        if cfg!(windows) {
            assert_eq!(project_id("D:\\Shop"), project_id("d:/shop/"));
        }
        assert_ne!(project_id("D:/Shop"), project_id("D:/Other"));
        assert_eq!(project_id("D:/Shop").len(), 16);
    }

    #[test]
    fn a_saved_local_login_wins_over_the_env() {
        let saved = DbLogin { host: "10.0.0.5".into(), port: 3307, database: "shop".into(), user: "dev".into(), ca: None };
        let files = env("127.0.0.1", "shop_env", "");
        let view = local_view(Some(&saved), Some(&files), true);
        assert_eq!((view.host.as_str(), view.from.as_str(), view.has_password), ("10.0.0.5", "saved", true));
        let read = local_view(None, Some(&files), false);
        assert_eq!((read.database.as_str(), read.from.as_str(), read.has_password), ("shop_env", ".env", false));
        assert_eq!(local_view(None, Some(&env("h", "d", "secret")), false).has_password, true);
    }

    #[test]
    fn nothing_found_asks_to_fill_it_in() {
        let local = local_view(None, None, false);
        assert_eq!(local.from, "none");
        assert!(!local.blocked.is_empty());
        let production = production_view(None, false);
        assert_eq!((production.from.as_str(), production.port), ("none", 25060));
        assert!(!production.blocked.is_empty());
    }

    #[test]
    fn production_opens_only_with_host_user_and_password() {
        let saved = DbLogin { host: "db-mysql-sgp1.example.com".into(), port: 25060, database: "app".into(), user: "doadmin".into(), ca: None };
        assert!(production_view(Some(&saved), false).blocked.contains("password"));
        assert!(production_view(Some(&saved), true).blocked.is_empty());
        let no_host = DbLogin { host: " ".into(), ..saved.clone() };
        assert!(production_view(Some(&no_host), true).blocked.contains("host"));
    }

    #[test]
    fn the_form_is_checked_before_anything_is_kept() {
        let form = |host: &str, user: &str, port: u16| Form { host: host.into(), port, database: " app ".into(), user: user.into(), password: None, ca_path: None };
        assert_eq!(checked(&form(" db.example ", "app", 25060)).unwrap().database, "app");
        assert!(checked(&form("", "app", 3306)).is_err());
        assert!(checked(&form("db example", "app", 3306)).is_err());
        assert!(checked(&form("db", " ", 3306)).is_err());
        assert!(checked(&form("db", "app", 0)).is_err());
    }

    #[test]
    fn the_project_files_are_read_in_order() {
        let folder = std::env::temp_dir().join(format!("castle-projectdb-env-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("src/main/resources")).unwrap();
        std::fs::write(folder.join("src/main/resources/application.properties"), "spring.datasource.url=jdbc:mysql://127.0.0.1:3306/bus\nspring.datasource.username=bus\n").unwrap();
        assert_eq!(found_in_files(&folder).map(|(l, from)| (l.database, from)), Some(("bus".into(), "application.properties")));
        std::fs::write(folder.join(".env"), "DB_HOST=localhost\nDB_DATABASE=shop\nDB_USERNAME=root\n").unwrap();
        assert_eq!(found_in_files(&folder).map(|(l, from)| (l.database, from)), Some(("shop".into(), ".env")));
        let _ = std::fs::remove_dir_all(folder);
        assert!(found_in_files(Path::new("Z:/nowhere/at/all")).is_none());
    }
}
