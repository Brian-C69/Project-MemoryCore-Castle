//! The Node that runs Castle's helpers (castle-image.mjs and the rest). A member who already has
//! Node 20 or later keeps using it; anyone else gets Castle's own, a portable official release
//! downloaded once into Castle's folder, the way the database engine comes. No admin prompt, no
//! installer window, no change to the member's own PATH.
//!
//! ```text
//!  %LOCALAPPDATA%\MemoryCore Castle\node\                (macOS: ~/Library/Application Support/MemoryCore Castle/node/)
//!    24.22.2\node.exe ...                                the official zip, unpacked (its top folder dropped)
//!    24.22.2/bin/node ...                                macOS: the official .tar.gz, unpacked
//! ```

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The Node release Castle brings: the long-term-support line, pinned.
pub const VERSION: &str = "24.22.2";
/// The helpers need at least this major version.
pub const MIN_MAJOR: u32 = 20;
/// The event the page hears while Node downloads: { done, total }.
pub const INSTALL_EVENT: &str = "castle://node-install";
/// Tests (and a careful developer) point Castle's own Node folder somewhere else.
pub const HOME_VAR: &str = "CASTLE_NODE_HOME";
/// Set to 1 to pass over the system's Node, so Castle's own can be tried on a machine that has one.
pub const IGNORE_SYSTEM_VAR: &str = "CASTLE_IGNORE_SYSTEM_NODE";
/// Points the download somewhere else (a test of the offline path); the checksum still decides.
pub const URL_VAR: &str = "CASTLE_NODE_URL";

const VERSION_LIMIT: Duration = Duration::from_secs(3);

/// One downloadable file of the pinned release, from its own SHASUMS256.txt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Release {
    pub file: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

const WIN_X64: Release = Release { file: "node-v24.22.2-win-x64.zip", sha256: "ca1bff5d8ead13c9f2ea404a176db46bd430704d5b5113734049fa365e7799e2", bytes: 37_754_834 };
const WIN_ARM64: Release = Release { file: "node-v24.22.2-win-arm64.zip", sha256: "f47968c213105077f64c6510f007632c0ca2574802a0b5067c327c49abe1c468", bytes: 33_789_289 };
const MAC_ARM64: Release = Release { file: "node-v24.22.2-darwin-arm64.tar.gz", sha256: "aa9549349e47dffaa42dcf95d6e06ade8c4bdafc980979883b88767670df385a", bytes: 53_068_959 };
const MAC_X64: Release = Release { file: "node-v24.22.2-darwin-x64.tar.gz", sha256: "d5e336d588c45be0c37dbd52cc29895593009ca4997a77a1fd02815131ed0e52", bytes: 54_383_415 };

/// The release for this computer, or None where Castle brings no Node (Linux keeps its own).
pub fn release() -> Option<Release> {
    release_for(std::env::consts::OS, std::env::consts::ARCH)
}

fn release_for(os: &str, arch: &str) -> Option<Release> {
    match (os, arch) {
        ("windows", "x86_64") => Some(WIN_X64),
        ("windows", "aarch64") => Some(WIN_ARM64),
        ("macos", "aarch64") => Some(MAC_ARM64),
        ("macos", "x86_64") => Some(MAC_X64),
        _ => None,
    }
}

fn url_of(release: &Release) -> String {
    match std::env::var(URL_VAR).ok().filter(|v| !v.is_empty()) {
        Some(url) => url,
        None => format!("https://nodejs.org/dist/v{VERSION}/{}", release.file),
    }
}

// ---------- where things live ----------

/// Castle's own Node folder (every version Castle brought sits inside it).
pub fn home() -> PathBuf {
    if let Some(dir) = std::env::var_os(HOME_VAR).filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "macos")]
    if let Some(user) = std::env::var_os("HOME") {
        return PathBuf::from(user).join("Library/Application Support/MemoryCore Castle/node");
    }
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    local.join("MemoryCore Castle").join("node")
}

/// Where the pinned release's `node` sits once unpacked.
pub fn own_program(home: &Path) -> PathBuf {
    let folder = home.join(VERSION);
    if cfg!(windows) {
        folder.join("node.exe")
    } else {
        folder.join("bin").join("node")
    }
}

// ---------- judging a Node ----------

/// `v24.22.2` (as `node --version` prints it) -> 24.
pub fn parse_major(printed: &str) -> Option<u32> {
    let line = printed.lines().map(str::trim).find(|l| !l.is_empty())?;
    line.strip_prefix('v')?.split('.').next()?.parse().ok()
}

/// Whether a printed version is one the helpers can run on.
pub fn good_enough(printed: &str) -> bool {
    parse_major(printed).is_some_and(|major| major >= MIN_MAJOR)
}

/// What `node --version` prints for this program, run hidden and at most a few seconds.
pub fn version_of(program: &Path) -> Option<String> {
    let mut command = Command::new(program);
    command.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let mut child = command.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if started.elapsed() < VERSION_LIMIT => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                return None;
            }
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let line = out.lines().map(str::trim).find(|l| !l.is_empty())?.to_string();
    parse_major(&line).map(|_| line)
}

/// The system's own Node: on PATH, else where the installers put it.
fn system_candidate() -> Option<PathBuf> {
    crate::env::find_on_path("node").or_else(|| {
        let places: &[&str] = if cfg!(windows) {
            &[r"C:\Program Files\nodejs\node.exe"]
        } else {
            &["/opt/homebrew/bin/node", "/usr/local/bin/node"]
        };
        places.iter().map(PathBuf::from).find(|p| p.is_file())
    })
}

/// A Node found and judged: where it is, what it printed.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub path: PathBuf,
    pub version: String,
}

/// Which Node runs the helpers: a good system Node first, then Castle's own, else none.
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    System(Found),
    Own(Found),
}

impl Choice {
    pub fn path(&self) -> &Path {
        match self {
            Choice::System(f) | Choice::Own(f) => &f.path,
        }
    }
}

/// The order of choice, apart from how each was found: a system Node only when it is new enough.
pub fn choose(system: Option<Found>, own: Option<Found>) -> Option<Choice> {
    match system.filter(|f| good_enough(&f.version)) {
        Some(found) => Some(Choice::System(found)),
        None => own.filter(|f| good_enough(&f.version)).map(Choice::Own),
    }
}

// The system's Node is judged once a run (a process each time would be slow); Castle's own is
// judged again whenever its file appears or goes.
static SYSTEM: Mutex<Option<Option<Found>>> = Mutex::new(None);
static OWN: Mutex<Option<(PathBuf, Option<Found>)>> = Mutex::new(None);
static PROGRESS: Mutex<Fetch> = Mutex::new(Fetch::Idle);
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq)]
enum Fetch {
    Idle,
    Running { done: u64, total: u64 },
    Failed(String),
}

fn system() -> Option<Found> {
    if std::env::var(IGNORE_SYSTEM_VAR).is_ok_and(|v| v == "1") {
        return None;
    }
    let mut cached = SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
    cached
        .get_or_insert_with(|| {
            let path = system_candidate()?;
            let version = version_of(&path)?;
            Some(Found { path, version })
        })
        .clone()
}

fn own() -> Option<Found> {
    let path = own_program(&home());
    let mut cached = OWN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((seen, found)) = cached.as_ref() {
        if *seen == path && found.is_some() == path.is_file() {
            return found.clone();
        }
    }
    let found = if path.is_file() { version_of(&path).map(|version| Found { path: path.clone(), version }) } else { None };
    *cached = Some((path, found.clone()));
    found
}

/// The Node the helpers run with right now, if there is one.
pub fn choice() -> Option<Choice> {
    choose(system(), own())
}

/// The folder of Castle's own Node when it is the one in use (for the session's PATH), else None.
pub fn own_folder_in_use() -> Option<PathBuf> {
    match choice()? {
        Choice::Own(found) => found.path.parent().map(Path::to_path_buf),
        Choice::System(_) => None,
    }
}

// ---------- what the page sees ----------

/// The helpers' Node, for the start-up screen, setup and Settings.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum NodeState {
    /// The member's own Node is new enough; nothing was downloaded.
    System { path: String, version: String },
    /// Castle's own Node is in use.
    Own { path: String, version: String },
    /// Castle's own Node is downloading.
    Fetching { done: u64, total: u64 },
    /// The last download did not work; chat and memory carry on.
    Failed { reason: String },
    /// No Node yet; this many bytes would come.
    Missing { download_bytes: u64 },
    /// No good Node and none Castle can bring here (Linux): install Node 20 or later.
    Unsupported,
}

pub fn status() -> NodeState {
    let fetch = PROGRESS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Fetch::Running { done, total } = fetch {
        return NodeState::Fetching { done, total };
    }
    match choice() {
        Some(Choice::System(f)) => NodeState::System { path: f.path.display().to_string(), version: f.version },
        Some(Choice::Own(f)) => NodeState::Own { path: f.path.display().to_string(), version: f.version },
        None => match (fetch, release()) {
            (Fetch::Failed(reason), _) => NodeState::Failed { reason },
            (_, Some(release)) => NodeState::Missing { download_bytes: release.bytes },
            (_, None) => NodeState::Unsupported,
        },
    }
}

fn set_progress(fetch: Fetch) {
    *PROGRESS.lock().unwrap_or_else(|e| e.into_inner()) = fetch;
}

// ---------- bringing Castle's own ----------

/// Makes sure a Node is there: nothing to do when one already is; otherwise downloads the pinned
/// release, keeps it only when its SHA-256 is the release's own, unpacks it into Castle's folder
/// and judges it. One download at a time; a second caller waits and then finds it done.
pub fn ensure(progress: impl Fn(u64, u64)) -> NodeState {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    if choice().is_some() {
        return status();
    }
    let Some(release) = release() else { return status() };
    set_progress(Fetch::Running { done: 0, total: release.bytes });
    match bring(&release, &url_of(&release), &home(), &|done, total| {
        set_progress(Fetch::Running { done, total });
        progress(done, total);
    }) {
        Ok(()) => set_progress(Fetch::Idle),
        Err(reason) => set_progress(Fetch::Failed(reason)),
    }
    status()
}

fn bring(release: &Release, url: &str, home: &Path, progress: &dyn Fn(u64, u64)) -> Result<(), String> {
    let bytes = crate::fetch::download(url, release.bytes, "The helpers' Node", progress)?;
    if !crate::fetch::matches(&bytes, release.sha256) {
        return Err("The download did not match Node's own checksum, so Castle threw it away. Try again later.".into());
    }
    std::fs::create_dir_all(home).map_err(|_| "Castle's Node folder could not be made.".to_string())?;
    let target = home.join(VERSION);
    if release.file.ends_with(".zip") {
        crate::fetch::unzip_dropping_top(&bytes, &target, |_| true, "Node")?;
    } else {
        crate::fetch::untar_gz_dropping_top(&bytes, &target, "Node")?;
    }
    let node = own_program(home);
    if version_of(&node).is_none_or(|v| !good_enough(&v)) {
        let _ = std::fs::remove_dir_all(&target);
        return Err("Node was downloaded but would not start on this computer.".into());
    }
    forget_older(home);
    Ok(())
}

/// A version Castle brought before is removed once the pinned one runs.
fn forget_older(home: &Path) {
    let Ok(entries) = std::fs::read_dir(home) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_version = name.split('.').count() == 3 && name.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
        if is_version && name != VERSION && entry.path().is_dir() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

// ---------- commands ----------

/// Which Node the helpers use right now (judging a Node runs a process: off the page's thread).
#[tauri::command]
pub async fn node_status() -> Result<NodeState, String> {
    tauri::async_runtime::spawn_blocking(status).await.map_err(|_| "The helpers' Node could not be checked.".to_string())
}

/// Makes sure the helpers have a Node, downloading Castle's own when needed; the page hears the
/// download on `INSTALL_EVENT` and gets the state at the end (Failed carries the reason).
#[tauri::command]
pub async fn node_prepare(app: tauri::AppHandle) -> Result<NodeState, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        ensure(move |done, total| {
            let _ = app.emit(INSTALL_EVENT, serde_json::json!({ "done": done, "total": total }));
        })
    })
    .await
    .map_err(|_| "The helpers' Node could not be prepared.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, version: &str) -> Found {
        Found { path: PathBuf::from(path), version: version.into() }
    }

    #[test]
    fn a_printed_version_is_read_and_judged() {
        assert_eq!(parse_major("v24.22.2\n"), Some(24));
        assert_eq!(parse_major("\n  v20.0.0  "), Some(20));
        assert_eq!(parse_major("24.22.2"), None, "node always prints the v");
        assert_eq!(parse_major("bash: node: command not found"), None);
        assert!(good_enough("v20.11.1"));
        assert!(good_enough("v26.13.1"));
        assert!(!good_enough("v18.20.4"));
        assert!(!good_enough(""));
    }

    #[test]
    fn a_good_system_node_comes_first_then_castles_own_then_none() {
        let sys = found("C:/Program Files/nodejs/node.exe", "v22.1.0");
        let own = found("C:/castle/node/24.22.2/node.exe", "v24.22.2");
        assert_eq!(choose(Some(sys.clone()), Some(own.clone())), Some(Choice::System(sys)));
        let old = found("C:/old/node.exe", "v18.20.4");
        assert_eq!(choose(Some(old.clone()), Some(own.clone())), Some(Choice::Own(own.clone())), "a system Node too old is passed over");
        assert_eq!(choose(None, Some(own.clone())), Some(Choice::Own(own)));
        assert_eq!(choose(Some(old), None), None);
        assert_eq!(choose(None, None), None);
    }

    #[test]
    fn every_system_castle_ships_for_has_its_release() {
        assert_eq!(release_for("windows", "x86_64"), Some(WIN_X64));
        assert_eq!(release_for("windows", "aarch64"), Some(WIN_ARM64));
        assert_eq!(release_for("macos", "aarch64"), Some(MAC_ARM64));
        assert_eq!(release_for("macos", "x86_64"), Some(MAC_X64));
        assert_eq!(release_for("linux", "x86_64"), None, "Linux keeps its own Node");
        for r in [WIN_X64, WIN_ARM64, MAC_ARM64, MAC_X64] {
            assert_eq!(r.sha256.len(), 64);
            assert!(r.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
            assert!(r.file.contains(&format!("v{VERSION}-")), "{} is the pinned version", r.file);
            assert!(r.bytes > 20_000_000);
        }
    }

    #[test]
    fn castles_own_node_sits_in_its_version_folder() {
        let node = own_program(Path::new("/castle/node"));
        assert!(node.starts_with(Path::new("/castle/node").join(VERSION)));
        assert_eq!(node.file_name().unwrap(), if cfg!(windows) { "node.exe" } else { "node" });
    }

    /// Serves `body` once over plain HTTP on a free local port; returns its address.
    fn serve_once(body: &'static [u8]) -> String {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/node.zip", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 1024];
                let _ = std::io::Read::read(&mut stream, &mut request);
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(body);
            }
        });
        address
    }

    #[test]
    fn a_download_that_is_not_the_release_is_thrown_away_before_anything_is_unpacked() {
        let dir = std::env::temp_dir().join(format!("castle-node-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let url = serve_once(b"not node at all");
        let error = bring(&WIN_X64, &url, &dir, &|_, _| {}).unwrap_err();
        assert!(error.contains("checksum"), "{error}");
        assert!(!dir.join(VERSION).exists(), "nothing was unpacked");
    }

    #[test]
    fn an_unreachable_download_says_so_in_plain_words() {
        let dir = std::env::temp_dir().join(format!("castle-node-off-{}", std::process::id()));
        // a port nothing listens on: bind one, learn it, let it go
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let error = bring(&WIN_X64, &format!("http://127.0.0.1:{port}/node.zip"), &dir, &|_, _| {}).unwrap_err();
        assert!(error.contains("could not be downloaded"), "{error}");
        assert!(!dir.join(VERSION).exists());
    }

    #[test]
    fn older_versions_castle_brought_are_forgotten_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!("castle-node-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for name in ["22.11.0", VERSION, "notes", "1.2"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
        }
        forget_older(&dir);
        assert!(!dir.join("22.11.0").exists());
        assert!(dir.join(VERSION).exists());
        assert!(dir.join("notes").exists() && dir.join("1.2").exists(), "only version folders are removed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_runnable_node_is_asked_its_version() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("castle-node-ver-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("node");
        std::fs::write(&fake, "#!/bin/sh\necho v24.22.2\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(version_of(&fake).as_deref(), Some("v24.22.2"));
        assert_eq!(version_of(&dir.join("missing")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_missing_node_has_no_version() {
        assert_eq!(version_of(Path::new(r"C:\no\such\node.exe")), None);
    }
}
