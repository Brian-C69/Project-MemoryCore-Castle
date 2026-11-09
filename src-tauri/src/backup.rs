//! Keeping the member's memory safe on GitHub, in a PRIVATE repository of their own.
//! git and the GitHub CLI (gh) run as child processes with no window; Castle never stores a
//! GitHub token (gh keeps its own sign-in). A remote that is MemoryCore's public template, or
//! any repository the member does not own, is never pushed to.

use crate::env;
use crate::settings::SettingsStore;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager, State};

/// MemoryCore's public template: a fresh MemoryCore cloned from it still points here.
const TEMPLATE_REPOS: [&str; 2] = ["kiyoraka/poject-ai-memory-architecture", "kiyoraka/project-ai-memory-architecture"];
/// Lines a memory backup must never carry.
const SAFETY_IGNORE: [&str; 6] = [".env", ".env.*", "*.key", "*.pem", "node_modules/", "media-generation/video-generation/"];
pub const BACKUP_EVENT: &str = "backup-done";

/// What a program said, or why it could not be run.
struct Ran {
    ok: bool,
    out: String,
    err: String,
}

fn run(program: &Path, args: &[&str], cwd: &Path, limit: Duration) -> Ran {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    // never let a git or gh prompt wait for typing that cannot come
    command.env("GIT_TERMINAL_PROMPT", "0").env("GH_PROMPT_DISABLED", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let failed = |why: &str| Ran { ok: false, out: String::new(), err: why.to_string() };
    let Ok(mut child) = command.spawn() else { return failed("could not be started") };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                return failed("took too long");
            }
        }
    }
    match child.wait_with_output() {
        Ok(output) => Ran {
            ok: output.status.success(),
            out: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            err: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(_) => failed("could not be read"),
    }
}

fn git(folder: &Path, args: &[&str]) -> Ran {
    match env::find_on_path("git") {
        Some(program) => run(&program, args, folder, Duration::from_secs(120)),
        None => Ran { ok: false, out: String::new(), err: "git is not installed".into() },
    }
}

fn gh(folder: &Path, args: &[&str]) -> Ran {
    match env::find_on_path("gh") {
        Some(program) => run(&program, args, folder, Duration::from_secs(120)),
        None => Ran { ok: false, out: String::new(), err: "the GitHub CLI is not installed".into() },
    }
}

/// "owner/name" from a GitHub remote url (https or ssh), lower-case; None for other hosts.
pub fn repo_slug(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    (!owner.is_empty() && !name.is_empty()).then(|| format!("{owner}/{name}").to_lowercase())
}

pub fn is_template(slug: &str) -> bool {
    TEMPLATE_REPOS.contains(&slug)
}

/// Everything the backup screen shows, in plain facts.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub git_installed: bool,
    pub gh_installed: bool,
    pub gh_signed_in: bool,
    /// the GitHub account gh is signed in as
    pub gh_user: Option<String>,
    pub is_repo: bool,
    pub remote_url: Option<String>,
    /// the remote is MemoryCore's public template, not the member's own backup
    pub remote_is_template: bool,
    /// the remote belongs to the signed-in account
    pub remote_owned: bool,
    /// "PRIVATE", "PUBLIC" or "INTERNAL", when GitHub said
    pub visibility: Option<String>,
    /// files changed since the last backup
    pub changed: usize,
    pub ahead: usize,
    pub last_backup: Option<String>,
    /// true when "Back up now" may run
    pub ready: bool,
    /// one plain sentence on what to do next
    pub next_step: String,
}

pub fn status(folder: &Path) -> BackupStatus {
    let mut s = BackupStatus {
        git_installed: env::find_on_path("git").is_some(),
        gh_installed: env::find_on_path("gh").is_some(),
        ..BackupStatus::default()
    };
    if s.gh_installed {
        let user = gh(folder, &["api", "user", "--jq", ".login"]);
        s.gh_signed_in = user.ok && !user.out.is_empty();
        s.gh_user = s.gh_signed_in.then(|| user.out.clone());
    }
    if s.git_installed && folder.join(".git").exists() {
        s.is_repo = true;
        let remote = git(folder, &["remote", "get-url", "origin"]);
        if remote.ok && !remote.out.is_empty() {
            s.remote_url = Some(remote.out.clone());
            let slug = repo_slug(&remote.out);
            s.remote_is_template = slug.as_deref().is_some_and(is_template);
            s.remote_owned = match (&slug, &s.gh_user) {
                (Some(slug), Some(user)) => slug.split('/').next() == Some(&user.to_lowercase()),
                _ => false,
            };
            if s.gh_signed_in && !s.remote_is_template {
                let view = gh(folder, &["repo", "view", &remote.out, "--json", "visibility", "--jq", ".visibility"]);
                s.visibility = view.ok.then(|| view.out.clone());
            }
        }
        let changes = git(folder, &["status", "--porcelain"]);
        s.changed = changes.out.lines().filter(|l| !l.trim().is_empty()).count();
        let ahead = git(folder, &["rev-list", "--count", "@{u}..HEAD"]);
        s.ahead = ahead.out.parse().unwrap_or(0);
        let last = git(folder, &["log", "-1", "--format=%cI"]);
        s.last_backup = (last.ok && !last.out.is_empty()).then(|| last.out.clone());
    }
    s.ready = s.is_repo && s.remote_url.is_some() && !s.remote_is_template && s.remote_owned && s.gh_signed_in;
    s.next_step = if !s.git_installed {
        "Install Git first (git-scm.com), then check again.".into()
    } else if s.remote_is_template {
        // said first: pushing a private memory to the public template is the one real danger
        "This folder still points at MemoryCore's public template, not your own backup. Create your private backup.".into()
    } else if !s.gh_installed {
        "Install the GitHub CLI (cli.github.com), then check again.".into()
    } else if !s.gh_signed_in {
        "Sign in to GitHub - Castle opens the sign-in in the Log.".into()
    } else if s.remote_url.is_some() && !s.remote_owned {
        "This folder's remote belongs to another GitHub account, so Castle will not push to it. Create your own private backup.".into()
    } else if s.remote_url.is_none() {
        "Create your private backup on GitHub.".into()
    } else if s.visibility.as_deref() == Some("PUBLIC") {
        "Warning: your backup repository is PUBLIC - anyone can read your memory. Make it private on GitHub.".into()
    } else {
        "Ready. Back up whenever you like.".into()
    };
    s
}

/// Adds the safety lines a memory backup needs to .gitignore; the member's own lines stay.
pub fn ensure_safety_ignore(folder: &Path) -> std::io::Result<()> {
    let path = folder.join(".gitignore");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let missing: Vec<&str> = SAFETY_IGNORE.iter().copied().filter(|line| !text.lines().any(|l| l.trim() == *line)).collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut out = text.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str("# Kept out of the memory backup by MemoryCore Castle\n");
    out.push_str(&missing.join("\n"));
    out.push('\n');
    std::fs::write(path, out)
}

fn plain(step: &str, ran: &Ran) -> String {
    let why = ran.err.lines().last().unwrap_or("").trim();
    if why.is_empty() {
        format!("{step} did not work.")
    } else {
        format!("{step} did not work: {why}")
    }
}

/// Commits every change with Castle's message, setting a commit name from the GitHub account
/// when the computer has none. Returns false when there was nothing to commit.
fn commit_all(folder: &Path, message: &str) -> Result<bool, String> {
    if !owns_repo(folder) {
        return Err("This memory folder sits inside another project's repository, so Castle will not back it up there. Move the memory folder out of that project first.".into());
    }
    ensure_safety_ignore(folder).map_err(|_| ".gitignore could not be written.".to_string())?;
    let add = git(folder, &["add", "-A"]);
    if !add.ok {
        return Err(plain("Collecting the changes", &add));
    }
    if git(folder, &["diff", "--cached", "--quiet"]).ok {
        return Ok(false);
    }
    if git(folder, &["config", "user.email"]).out.is_empty() {
        let who = gh(folder, &["api", "user", "--jq", "\"\\(.id)+\\(.login)\""]);
        if let Some((id, login)) = who.out.trim_matches('"').split_once('+') {
            let _ = git(folder, &["config", "user.name", login]);
            let _ = git(folder, &["config", "user.email", &format!("{id}+{login}@users.noreply.github.com")]);
        }
    }
    let commit = git(folder, &["commit", "-m", message]);
    if commit.ok {
        Ok(true)
    } else {
        Err(plain("Saving the changes", &commit))
    }
}

/// True when the repository git finds for `folder` is the memory folder's own (its top folder is
/// the memory folder), so `git add -A` can never sweep up a project the memory happens to sit in.
fn owns_repo(folder: &Path) -> bool {
    let top = git(folder, &["rev-parse", "--show-toplevel"]);
    if !top.ok {
        return false;
    }
    let same = |a: &Path, b: &Path| match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    same(Path::new(top.out.trim()), folder)
}

/// Makes the member's own PRIVATE backup: a repository here (when there is none), the template
/// remote kept as "upstream", the first backup, and `gh repo create --private --push`.
pub fn create_private(folder: &Path, name: &str) -> Result<BackupStatus, String> {
    let name: String = name.trim().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    if name.trim_matches('-').is_empty() {
        return Err("Choose a name for the backup repository.".into());
    }
    let before = status(folder);
    if !before.gh_signed_in {
        return Err(before.next_step);
    }
    if !before.is_repo {
        let init = git(folder, &["init", "-b", "main"]);
        if !init.ok {
            return Err(plain("Starting the backup", &init));
        }
    }
    if before.remote_url.is_some() {
        // the template (or someone else's repo) stays reachable as upstream; origin becomes theirs
        let _ = git(folder, &["remote", "remove", "upstream"]);
        let renamed = git(folder, &["remote", "rename", "origin", "upstream"]);
        if !renamed.ok {
            return Err(plain("Setting the old remote aside", &renamed));
        }
    }
    commit_all(folder, "Memory backup - first backup by MemoryCore Castle")?;
    let created = gh(folder, &["repo", "create", &name, "--private", "--source", ".", "--remote", "origin", "--push"]);
    if !created.ok {
        return Err(plain("Creating the private repository", &created));
    }
    Ok(status(folder))
}

/// Saves every change and sends it to the member's own private backup.
pub fn back_up_now(folder: &Path, message: &str) -> Result<BackupStatus, String> {
    let before = status(folder);
    if !before.ready {
        return Err(before.next_step);
    }
    commit_all(folder, message)?;
    let push = git(folder, &["push", "-u", "origin", "HEAD"]);
    if !push.ok {
        return Err(plain("Sending the backup to GitHub", &push));
    }
    Ok(status(folder))
}

fn memory_folder(store: &SettingsStore) -> Result<PathBuf, String> {
    store.get().memory_folder.map(PathBuf::from).ok_or_else(|| "No MemoryCore folder is chosen yet.".to_string())
}

#[tauri::command]
pub async fn backup_status(store: State<'_, SettingsStore>) -> Result<BackupStatus, String> {
    let folder = memory_folder(&store)?;
    tauri::async_runtime::spawn_blocking(move || status(&folder)).await.map_err(|_| "The check stopped.".to_string())
}

#[tauri::command]
pub async fn backup_create(name: String, store: State<'_, SettingsStore>) -> Result<BackupStatus, String> {
    let folder = memory_folder(&store)?;
    tauri::async_runtime::spawn_blocking(move || create_private(&folder, &name)).await.map_err(|_| "The backup stopped.".to_string())?
}

#[tauri::command]
pub async fn backup_now(store: State<'_, SettingsStore>) -> Result<BackupStatus, String> {
    let folder = memory_folder(&store)?;
    tauri::async_runtime::spawn_blocking(move || back_up_now(&folder, "Memory backup by MemoryCore Castle"))
        .await
        .map_err(|_| "The backup stopped.".to_string())?
}

/// Opens GitHub's own sign-in in a console window of its own (browser + one-time code); the
/// member finishes it there, then presses Check again.
#[tauri::command]
pub fn backup_sign_in() -> Result<(), String> {
    let gh = env::find_on_path("gh").ok_or("Install the GitHub CLI first (cli.github.com).")?;
    Command::new("cmd.exe")
        .args(["/c", "start", "GitHub sign-in", "cmd", "/k"])
        .arg(gh)
        .args(["auth", "login", "--web", "--hostname", "github.com", "--git-protocol", "https"])
        .spawn()
        .map(|_| ())
        .map_err(|_| "The GitHub sign-in could not be opened.".to_string())
}

/// The newest change time of anything in daily-diary/ (two levels: files and day folders).
fn diary_changed(folder: &Path) -> Option<SystemTime> {
    let diary = folder.join("daily-diary");
    let mut newest: Option<SystemTime> = None;
    let mut look = |p: &Path| {
        if let Ok(time) = p.metadata().and_then(|m| m.modified()) {
            newest = Some(newest.map_or(time, |n| n.max(time)));
        }
    };
    for entry in std::fs::read_dir(&diary).ok()?.flatten() {
        let path = entry.path();
        look(&path);
        if path.is_dir() {
            for inner in std::fs::read_dir(&path).into_iter().flatten().flatten() {
                look(&inner.path());
            }
        }
    }
    newest
}

/// Backs up after every Save diary while the member's switch is on: the diary folder is checked
/// once a minute; a change that has settled for a minute starts one backup.
pub fn start_diary_watcher(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut seen: Option<SystemTime> = None;
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let settings = app.state::<SettingsStore>().get();
            let Some(folder) = settings.memory_folder.map(PathBuf::from) else { continue };
            let changed = diary_changed(&folder);
            let settled = changed.is_some_and(|t| SystemTime::now().duration_since(t).unwrap_or_default() > Duration::from_secs(60));
            if settings.backup_after_diary && settled && seen.is_some() && changed != seen {
                let result = back_up_now(&folder, "Memory backup after Save diary");
                let _ = app.emit(BACKUP_EVENT, result.map(|_| "Memory backed up after Save diary.".to_string()));
            }
            if settled || seen.is_none() {
                seen = changed;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_remotes_are_read_and_the_template_is_recognised() {
        assert_eq!(repo_slug("https://github.com/Kiyoraka/Poject-AI-Memory-Architecture.git").as_deref(), Some("kiyoraka/poject-ai-memory-architecture"));
        assert_eq!(repo_slug("git@github.com:Rin/my-memory.git").as_deref(), Some("rin/my-memory"));
        assert_eq!(repo_slug("https://gitlab.com/rin/my-memory"), None);
        assert!(is_template("kiyoraka/poject-ai-memory-architecture"));
        assert!(!is_template("rin/my-memory"));
    }

    #[test]
    fn the_safety_lines_are_added_once_and_the_members_lines_stay() {
        let root = std::env::temp_dir().join(format!("castle-backup-ignore-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), "notes/private/\n.env\n").unwrap();
        ensure_safety_ignore(&root).unwrap();
        ensure_safety_ignore(&root).unwrap();
        let text = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(text.starts_with("notes/private/\n.env\n"));
        assert_eq!(text.matches(".env\n").count(), 1);
        assert_eq!(text.matches("*.key").count(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_memory_inside_another_project_is_never_committed_there() {
        if env::find_on_path("git").is_none() {
            return;
        }
        let project = std::env::temp_dir().join(format!("castle-backup-outer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&project);
        let memory = project.join("fixtures").join("memory");
        std::fs::create_dir_all(&memory).unwrap();
        assert!(git(&project, &["init", "-b", "main"]).ok);
        std::fs::write(memory.join("master-memory.md"), "# memory\n").unwrap();
        std::fs::write(project.join("app.rs"), "fn main() {}\n").unwrap();
        assert!(!owns_repo(&memory));
        assert!(commit_all(&memory, "x").is_err());
        assert!(git(&project, &["log", "--oneline"]).out.is_empty(), "the outer project got no commit");
        // its own repository is fine
        assert!(git(&memory, &["init", "-b", "main"]).ok);
        assert!(owns_repo(&memory));
        let _ = std::fs::remove_dir_all(project);
    }

    #[test]
    fn a_local_repo_without_a_remote_is_never_pushed() {
        if env::find_on_path("git").is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("castle-backup-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(git(&root, &["init", "-b", "main"]).ok);
        std::fs::write(root.join("master-memory.md"), "# memory\n").unwrap();
        let s = status(&root);
        assert!(s.is_repo && s.remote_url.is_none() && !s.ready);
        assert_eq!(s.changed, 1);
        assert!(back_up_now(&root, "x").is_err(), "no remote: nothing is pushed");
        // a template remote is never a backup
        assert!(git(&root, &["remote", "add", "origin", "https://github.com/Kiyoraka/Poject-AI-Memory-Architecture.git"]).ok);
        let s = status(&root);
        assert!(s.remote_is_template && !s.ready);
        assert!(s.next_step.contains("template"));
        let _ = std::fs::remove_dir_all(root);
    }
}
