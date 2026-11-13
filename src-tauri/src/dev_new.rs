//! The DEV tab's "New project" screen: clone a repo from a link, or make a new GitHub repo with
//! the `gh` CLI, into `<projects folder>\<kind>\<name>` (or straight into the projects folder when
//! no kind is chosen). It also puts an existing project on GitHub (`connect`) and starts git in a
//! folder that has none (`git_init`).
//!
//! Git and gh are always run as programs with their own arguments, never through a shell, never
//! with a prompt or a window, and never over a folder that already exists. The projects folder is
//! the member's own choice (`settings.dev_root`); there is no default.

use crate::env;
use crate::settings::{self, Settings, SettingsStore};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::State;

/// Local git steps (init, add, commit, config, remote).
const QUICK: Duration = Duration::from_secs(60);
/// Steps that talk to GitHub or another host (clone, repo create, push).
const SLOW: Duration = Duration::from_secs(600);
/// Asking gh who is signed in.
const ASK: Duration = Duration::from_secs(30);

const NO_ROOT: &str = "Choose a projects folder first.";
const NO_GH: &str = "Install the GitHub CLI first (cli.github.com).";

// ---------------------------------------------------------------- the rules

/// A name GitHub and the folder can both carry: letters, digits, `.`, `_`, `-`.
pub fn is_repo_name(name: &str) -> bool {
    let len = name.chars().count();
    (1..=100).contains(&len)
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        && !name.starts_with('.')
        && !name.starts_with('-')
}

/// A GitHub user or organisation login.
pub fn is_owner(owner: &str) -> bool {
    let len = owner.chars().count();
    (1..=39).contains(&len)
        && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !owner.starts_with('-')
        && !owner.ends_with('-')
}

/// The repo name at the end of a clone link, without `.git` or a trailing `/`.
pub fn name_from_link(link: &str) -> Option<String> {
    let mut rest = link.trim().trim_end_matches('/');
    if let Some(stripped) = rest.strip_suffix(".git") {
        rest = stripped.trim_end_matches('/');
    }
    let name = rest.rsplit(['/', ':']).next()?;
    is_repo_name(name).then(|| name.to_string())
}

/// A link git may be handed safely: a known scheme, a host, no spaces or flags.
pub fn is_clone_link(link: &str) -> bool {
    if link.is_empty()
        || link.chars().count() > 500
        || link.starts_with('-')
        || link.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return false;
    }
    let host = if let Some(rest) = link.strip_prefix("git@") {
        // git@host:owner/repo - the host ends at the colon.
        match rest.split_once(':') {
            Some((host, path)) if !path.is_empty() => host,
            _ => return false,
        }
    } else if let Some(rest) = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))
        .or_else(|| link.strip_prefix("ssh://"))
    {
        let authority = rest.split('/').next().unwrap_or("");
        // Drop any user@ in front of the host.
        authority.rsplit('@').next().unwrap_or("")
    } else {
        return false;
    };
    let host = host.split(':').next().unwrap_or("");
    !host.is_empty() && !host.starts_with('-')
}

/// The project kinds: the folders inside the projects folder, hidden ones left out, sorted.
pub fn kinds(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// The folder a new project goes in: the projects folder itself when `kind` is empty, else one of
/// its kinds (a plain folder name, never a path).
fn parent_folder(root: &Path, kind: &str) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err(format!("The projects folder is not there: {}", root.display()));
    }
    if kind.is_empty() {
        return Ok(root.to_path_buf());
    }
    let plain = !kind.contains(['/', '\\', ':']) && kind != "." && kind != "..";
    if !plain || !kinds(root).iter().any(|k| k == kind) {
        return Err(format!("There is no {kind} folder in {}.", root.display()));
    }
    Ok(root.join(kind))
}

/// The new project's folder, refused if the kind is unknown, the name is bad or the folder is
/// already there.
pub fn new_folder(root: &Path, kind: &str, name: &str) -> Result<PathBuf, String> {
    let parent = parent_folder(root, kind)?;
    if !is_repo_name(name) {
        return Err("The name can only use letters, digits, dots, dashes and underscores.".to_string());
    }
    let folder = parent.join(name);
    if folder.exists() {
        return Err(format!("That folder already exists: {}", folder.display()));
    }
    Ok(folder)
}

/// The projects folder from the settings, or the plain line asking the member to choose one.
pub fn root_of(settings: &Settings) -> Result<PathBuf, String> {
    settings
        .dev_root
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| NO_ROOT.to_string())
}

// ---------------------------------------------------------------- running git and gh

fn git_program() -> Result<PathBuf, String> {
    env::find_on_path("git").ok_or_else(|| "git is not installed on this computer.".to_string())
}

/// GitHub's tool: on PATH, else its install folder (a Castle started before the GitHub CLI was
/// installed still carries the old PATH).
fn gh_program() -> Option<PathBuf> {
    env::find_on_path("gh").or_else(|| {
        let mut places = vec![PathBuf::from(r"C:\Program Files\GitHub CLI\gh.exe")];
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            places.push(PathBuf::from(local).join("Programs").join("GitHub CLI").join("gh.exe"));
        }
        places.into_iter().find(|p| p.is_file())
    })
}

/// Reads a pipe to its end on its own thread so a full pipe never stalls the program.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> mpsc::Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = send.send(bytes);
    });
    receive
}

/// Runs a program with its arguments: no shell, no window, nothing to type into, at most
/// `timeout`. Gives back its trimmed output, or the tool's own words when it fails.
fn run(program: &Path, args: &[&str], cwd: Option<&Path>, timeout: Duration) -> Result<String, String> {
    let name = program.file_stem().and_then(|s| s.to_str()).unwrap_or("The program").to_string();
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1");
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("{name} is not installed on this computer.")
        } else {
            format!("{name} could not run.")
        }
    })?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{name} took too long."));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.kill();
                return Err(format!("{name} could not run."));
            }
        }
    };
    // A helper left running could hold a pipe open; never wait on it forever.
    let grace = Duration::from_secs(5);
    let stdout = String::from_utf8_lossy(&out.recv_timeout(grace).unwrap_or_default()).trim().to_string();
    let stderr = String::from_utf8_lossy(&err.recv_timeout(grace).unwrap_or_default()).trim().to_string();
    if status.success() {
        Ok(stdout)
    } else if !stderr.is_empty() {
        Err(stderr)
    } else if !stdout.is_empty() {
        Err(stdout)
    } else {
        Err(format!("{name} could not run."))
    }
}

fn git(folder: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    run(&git_program()?, args, Some(folder), timeout)
}

/// The folder as text a program can take, or a plain refusal.
fn folder_text(folder: &Path) -> Result<&str, String> {
    folder.to_str().ok_or_else(|| "That folder's path has characters git cannot take.".to_string())
}

// ---------------------------------------------------------------- GitHub

/// What the gh CLI knows: whether it is here, who is signed in, where repos may go.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHub {
    pub installed: bool,
    pub signed_in: bool,
    pub login: Option<String>,
    /// The signed-in account first, then its organisations.
    pub owners: Vec<String>,
}

/// Asks gh whether it is installed, who is signed in and which organisations they belong to.
pub fn github() -> GitHub {
    let mut state = GitHub { installed: false, signed_in: false, login: None, owners: Vec::new() };
    let Some(gh) = gh_program() else {
        return state;
    };
    if run(&gh, &["--version"], None, ASK).is_err() {
        return state;
    }
    state.installed = true;
    let login = match run(&gh, &["api", "user", "--jq", ".login"], None, ASK) {
        Ok(login) if is_owner(login.trim()) => login.trim().to_string(),
        _ => return state,
    };
    state.signed_in = true;
    state.owners.push(login.clone());
    state.login = Some(login);
    if let Ok(orgs) = run(&gh, &["api", "user/orgs", "--jq", ".[].login"], None, ASK) {
        for org in orgs.lines().map(str::trim).filter(|o| is_owner(o)) {
            if !state.owners.iter().any(|o| o == org) {
                state.owners.push(org.to_string());
            }
        }
    }
    state
}

// ---------------------------------------------------------------- new projects

/// Clones the link into `root\kind\name` and gives back the new folder. An empty name takes the
/// repo's own name from the link.
pub fn clone(link: &str, root: &Path, kind: &str, name: &str) -> Result<PathBuf, String> {
    let link = link.trim();
    if !is_clone_link(link) {
        return Err("That does not look like a link git can clone.".to_string());
    }
    let name = match name {
        "" => name_from_link(link).ok_or("Give the new project a name.")?,
        given => given.to_string(),
    };
    let folder = new_folder(root, kind, &name)?;
    let target = folder_text(&folder)?;
    let parent = parent_folder(root, kind)?;
    if let Err(words) = git(&parent, &["clone", "--", link, target], SLOW) {
        // The folder was not there before, so anything left there is the failed clone's.
        if folder.exists() {
            let _ = fs::remove_dir_all(&folder);
        }
        return Err(words);
    }
    Ok(folder)
}

/// Makes `root\kind\name`, starts git in it and creates `owner/name` on GitHub with it as the
/// source and `origin`. Gives back the new folder.
pub fn create(owner: &str, name: &str, private: bool, root: &Path, kind: &str) -> Result<PathBuf, String> {
    if !is_owner(owner) {
        return Err("That is not a GitHub account or organisation name.".to_string());
    }
    let gh = gh_program().ok_or(NO_GH)?;
    let folder = new_folder(root, kind, name)?;
    let target = folder_text(&folder)?.to_string();
    fs::create_dir(&folder).map_err(|_| format!("The folder could not be made: {}", folder.display()))?;
    let made = (|| {
        git(&folder, &["init"], QUICK)?;
        let repo = format!("{owner}/{name}");
        let visibility = if private { "--private" } else { "--public" };
        run(&gh, &["repo", "create", &repo, visibility, "--source", &target, "--remote", "origin"], Some(&folder), SLOW)
    })();
    if let Err(words) = made {
        // Only the folder made just now is removed.
        let _ = fs::remove_dir_all(&folder);
        return Err(words);
    }
    Ok(folder)
}

// ---------------------------------------------------------------- existing projects

fn has_git(folder: &Path) -> bool {
    folder.join(".git").exists()
}

/// Starts git in a project folder. A folder that already has git is left as it is.
pub fn git_init(folder: &Path) -> Result<(), String> {
    if !folder.is_dir() {
        return Err("That project folder is not there.".to_string());
    }
    if has_git(folder) {
        return Ok(());
    }
    git(folder, &["init"], QUICK).map(|_| ())
}

/// The refusals `connect` makes before it changes anything: a bad owner or name, a missing folder,
/// an `origin` already there. Gives back whether the folder already has git.
fn connect_check(folder: &Path, owner: &str, name: &str) -> Result<bool, String> {
    if !is_owner(owner) {
        return Err("That is not a GitHub account or organisation name.".to_string());
    }
    if !is_repo_name(name) {
        return Err("The name can only use letters, digits, dots, dashes and underscores.".to_string());
    }
    if !folder.is_dir() {
        return Err("That project folder is not there.".to_string());
    }
    let repo = has_git(folder);
    if repo {
        let remotes = git(folder, &["remote"], QUICK)?;
        if remotes.lines().any(|r| r.trim() == "origin") {
            return Err("This project already has a remote called origin, so it is already connected somewhere.".to_string());
        }
    }
    Ok(repo)
}

/// git's own user.name and user.email, needed before a first commit.
fn identity_check(folder: &Path) -> Result<(), String> {
    let set = |key: &str| git(folder, &["config", key], QUICK).is_ok_and(|v| !v.trim().is_empty());
    if !set("user.name") || !set("user.email") {
        return Err("git does not know your name yet. Set it once in a terminal: git config --global user.name \"Your Name\" and git config --global user.email \"you@example.com\", then try again.".to_string());
    }
    Ok(())
}

/// Puts an existing project on GitHub as `owner/name`: git is started when it is not there, a
/// first commit of everything is made when there is none, then gh creates the repo with this
/// folder as its source and `origin`, and pushes. Gives back the repo's address.
pub fn connect(folder: &Path, owner: &str, name: &str, private: bool) -> Result<String, String> {
    let repo = connect_check(folder, owner, name)?;
    let gh = gh_program().ok_or(NO_GH)?;
    let needs_commit = !repo || git(folder, &["rev-parse", "--verify", "-q", "HEAD"], QUICK).is_err();
    if needs_commit {
        identity_check(folder)?;
        let empty = fs::read_dir(folder).map(|mut e| e.next().is_none()).unwrap_or(true);
        if empty {
            return Err("This folder is empty, so there is nothing to put on GitHub yet.".to_string());
        }
    }
    if !repo {
        git(folder, &["init"], QUICK)?;
    }
    if needs_commit {
        git(folder, &["add", "-A"], QUICK)?;
        if git(folder, &["status", "--porcelain"], QUICK)?.is_empty() {
            return Err("There is nothing in this project for a first commit yet.".to_string());
        }
        git(folder, &["commit", "-m", "First commit"], QUICK)?;
    }
    let source = folder_text(folder)?;
    let repo_name = format!("{owner}/{name}");
    let visibility = if private { "--private" } else { "--public" };
    run(&gh, &["repo", "create", &repo_name, visibility, "--source", source, "--remote", "origin", "--push"], Some(folder), SLOW)?;
    git(folder, &["remote", "get-url", "origin"], QUICK)
}

// ---------------------------------------------------------------- commands

fn tab_folder(settings: &Settings, slot: usize) -> Result<PathBuf, String> {
    settings
        .dev_tabs
        .get(slot)
        .cloned()
        .flatten()
        .map(PathBuf::from)
        .ok_or_else(|| "That DEV tab has no project open.".to_string())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static, failed: &str) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| failed.to_string())?
}

/// GitHub's own tool on this PC: installed, signed in, and the owners a new repo can go under.
#[tauri::command]
pub async fn dev_github() -> Result<GitHub, String> {
    blocking(|| Ok(github()), "GitHub could not be asked.").await
}

/// The projects folder new projects go in, when the member has chosen one.
#[tauri::command]
pub fn dev_root(store: State<SettingsStore>) -> Option<String> {
    store.get().dev_root
}

/// Chooses the projects folder (it must be an existing folder).
#[tauri::command]
pub fn dev_set_root(folder: String, store: State<SettingsStore>) -> Result<(), String> {
    let folder = folder.trim().to_string();
    if folder.is_empty() || !Path::new(&folder).is_dir() {
        return Err("That folder is not there.".to_string());
    }
    let mut next = store.get();
    next.dev_root = Some(folder);
    settings::save(&store.path, &next).map_err(|_| "The projects folder could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(())
}

/// The kinds of project in the projects folder (the folders inside it).
#[tauri::command]
pub async fn dev_kinds(store: State<'_, SettingsStore>) -> Result<Vec<String>, String> {
    let root = root_of(&store.get())?;
    blocking(move || Ok(kinds(&root)), "The projects folder could not be read.").await
}

/// New project from a link, cloned into the projects folder; gives back the new folder.
#[tauri::command]
pub async fn dev_clone(link: String, kind: String, name: String, store: State<'_, SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "dev")?;
    let root = root_of(&store.get())?;
    blocking(move || clone(&link, &root, kind.trim(), name.trim()).map(|f| f.display().to_string()), "The clone could not run.").await
}

/// New project on GitHub: the folder, git, and owner/name created by gh; gives back the new folder.
#[tauri::command]
pub async fn dev_create(owner: String, name: String, private: bool, kind: String, store: State<'_, SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "dev")?;
    let root = root_of(&store.get())?;
    blocking(
        move || create(owner.trim(), name.trim(), private, &root, kind.trim()).map(|f| f.display().to_string()),
        "The repo could not be created.",
    )
    .await
}

/// Puts the project open in a DEV tab on GitHub; gives back the repo's address.
#[tauri::command]
pub async fn dev_connect(slot: usize, owner: String, name: String, private: bool, store: State<'_, SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "dev")?;
    let folder = tab_folder(&store.get(), slot)?;
    blocking(move || connect(&folder, owner.trim(), name.trim(), private), "The project could not be connected.").await
}

/// Starts git in the project open in a DEV tab.
#[tauri::command]
pub async fn dev_git_init(slot: usize, store: State<'_, SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "dev")?;
    let folder = tab_folder(&store.get(), slot)?;
    blocking(move || git_init(&folder), "git could not run.").await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh scratch projects folder with a few kinds, unique to this process and test.
    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("castle-dev-new-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Web")).unwrap();
        fs::create_dir_all(root.join("Application")).unwrap();
        fs::create_dir_all(root.join("Games")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("notes.txt"), "not a kind").unwrap();
        root
    }

    fn have_git() -> bool {
        env::find_on_path("git").is_some()
    }

    #[test]
    fn the_projects_folder_comes_only_from_the_setting() {
        assert_eq!(root_of(&Settings::default()).unwrap_err(), NO_ROOT);
        let blank = Settings { dev_root: Some("  ".into()), ..Settings::default() };
        assert_eq!(root_of(&blank).unwrap_err(), NO_ROOT);
        let set = Settings { dev_root: Some(r"E:\Work".into()), ..Settings::default() };
        assert_eq!(root_of(&set), Ok(PathBuf::from(r"E:\Work")));
    }

    #[test]
    fn repo_names() {
        for good in ["alice-keep", "Alice_Keep", "a", "v1.2", "x.y-z_9", &"a".repeat(100)] {
            assert!(is_repo_name(good), "{good} should be a name");
        }
        for bad in ["", ".", "..", ".hidden", "-rf", "../x", "a/b", r"a\b", "my repo", "name!", "café", &"a".repeat(101)] {
            assert!(!is_repo_name(bad), "{bad} should not be a name");
        }
    }

    #[test]
    fn owners() {
        for good in ["afif", "Kiyo-Software", "a", "a1-b2", &"a".repeat(39)] {
            assert!(is_owner(good), "{good} should be an owner");
        }
        for bad in ["", "-afif", "afif-", "a b", "a_b", "a.b", "../x", &"a".repeat(40)] {
            assert!(!is_owner(bad), "{bad} should not be an owner");
        }
    }

    #[test]
    fn names_from_links() {
        let cases = [
            ("https://github.com/a/b", "b"),
            ("https://github.com/a/b.git", "b"),
            ("https://github.com/a/b/", "b"),
            ("https://host/x/y/b.git/", "b"),
            ("git@github.com:a/b.git", "b"),
            ("git@github.com:b.git", "b"),
            ("ssh://git@github.com/a/alice-keep.git", "alice-keep"),
        ];
        for (link, name) in cases {
            assert_eq!(name_from_link(link).as_deref(), Some(name), "{link}");
        }
        for bad in ["", "https://github.com/a/my repo", "https://github.com/a/..", "https://github.com/a/-rf"] {
            assert_eq!(name_from_link(bad), None, "{bad}");
        }
    }

    #[test]
    fn clone_links() {
        for good in [
            "https://github.com/a/b",
            "https://github.com/a/b.git",
            "http://gitlab.local/a/b",
            "git@github.com:a/b.git",
            "ssh://git@github.com/a/b.git",
            "https://user@host:8443/a/b.git/",
        ] {
            assert!(is_clone_link(good), "{good} should be a link");
        }
        for bad in [
            "",
            "-rf",
            "--upload-pack=evil",
            "github.com/a/b",
            "file:///C:/x",
            "https://",
            "https:///a/b",
            "git@",
            "git@github.com",
            "git@:a/b",
            "https://github.com/a/b c",
            "https://github.com/a/b\n",
            "https://github.com/a/\u{7}b",
            &format!("https://github.com/a/{}", "b".repeat(500)),
        ] {
            assert!(!is_clone_link(bad), "{bad:?} should not be a link");
        }
    }

    #[test]
    fn kinds_are_folders_only_sorted() {
        let root = scratch("kinds");
        assert_eq!(kinds(&root), vec!["Application", "Games", "Web"]);
        assert!(kinds(&root.join("missing")).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_folder_checks_everything() {
        let root = scratch("folder");
        assert_eq!(new_folder(&root, "Web", "site").unwrap(), root.join("Web").join("site"));
        assert_eq!(new_folder(&root, "", "site").unwrap(), root.join("site"), "no kind = straight in the projects folder");
        assert!(new_folder(&root, "Mobile", "site").is_err());
        assert!(new_folder(&root, ".hidden", "site").is_err());
        assert!(new_folder(&root, "notes.txt", "site").is_err());
        assert!(new_folder(&root, "..", "site").is_err());
        assert!(new_folder(&root, "Web/..", "site").is_err());
        assert!(new_folder(&root, r"Web\..\Games", "site").is_err());
        assert!(new_folder(&root, "Web", "../x").is_err());
        assert!(new_folder(&root, "Web", "my site").is_err());
        assert!(new_folder(&root, "Web", "-rf").is_err());
        assert!(new_folder(&root.join("missing"), "", "site").is_err(), "a projects folder that is gone");

        fs::create_dir(root.join("Web").join("taken")).unwrap();
        let error = new_folder(&root, "Web", "taken").unwrap_err();
        assert!(error.starts_with("That folder already exists: "), "{error}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_refused_clone_or_create_makes_no_folder() {
        let root = scratch("refused");
        assert!(clone("not a link", &root, "Web", "site").is_err());
        assert!(clone("https://example.invalid/a/site.git", &root, "Mobile", "site").is_err());
        assert!(create("-bad-", "site", true, &root, "Web").is_err());
        assert!(!root.join("Web").join("site").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn git_init_makes_a_repository_and_leaves_one_alone() {
        if !have_git() {
            return;
        }
        let root = scratch("init");
        let project = root.join("Web");
        git_init(&project).unwrap();
        assert!(project.join(".git").is_dir());
        git_init(&project).unwrap();
        assert!(git_init(&root.join("missing")).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn connect_refuses_before_changing_anything() {
        let root = scratch("connect");
        let project = root.join("Web");
        fs::write(project.join("index.html"), "<p>hi</p>").unwrap();
        assert!(connect(&project, "-bad-", "site", true).is_err());
        assert!(connect(&project, "afif", "my site", true).is_err());
        assert!(connect(&root.join("missing"), "afif", "site", true).is_err());
        assert!(!project.join(".git").exists(), "a refusal starts no git");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn connect_refuses_a_project_that_already_has_an_origin() {
        if !have_git() {
            return;
        }
        let root = scratch("origin");
        let project = root.join("Web");
        git(&project, &["init"], QUICK).unwrap();
        git(&project, &["remote", "add", "origin", "https://example.invalid/x.git"], QUICK).unwrap();
        let error = connect(&project, "afif", "site", true).unwrap_err();
        assert!(error.contains("origin"), "{error}");
        assert_eq!(git(&project, &["remote", "get-url", "origin"], QUICK).unwrap(), "https://example.invalid/x.git");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_github_state_is_camel_case() {
        let state = GitHub { installed: true, signed_in: true, login: Some("afif".into()), owners: vec!["afif".into(), "Kiyo-Software".into()] };
        let text = serde_json::to_string(&state).unwrap();
        assert_eq!(text, r#"{"installed":true,"signedIn":true,"login":"afif","owners":["afif","Kiyo-Software"]}"#);
    }
}
