//! SWEEP: a system cleaner of Castle's own, behind HOME's Sweep widget. It only ever deletes inside a
//! fixed list of known cache folders, never follows a link or a junction, skips what is in use,
//! cleans only when the member presses the button, and writes one log line per group.
//!
//! Never touched by design: documents, downloads, desktop, pictures, the memory folder, project
//! folders, cookies, saved passwords and Castle's own folders. In Temp, the Claude Code sessions'
//! `claude` scratchpads and anything changed in the last 24 hours stay.
//!
//! The engine is the Alice Keep's (its Phase 97), ported: the same walk and rules, with the macOS
//! folders added and Deep clean run through Castle itself, elevated, with this same safe walk.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The groups, in the sheet's order: (id, label).
pub const GROUPS: [(&str, &str); 5] = [
    ("temp", "Temp files"),
    ("browser", "Browser caches"),
    ("thumbs", "Thumbnail cache"),
    ("crash", "Crash dumps and logs"),
    ("recycle", "Recycle Bin"),
];

/// A file changed more recently than this stays in Temp.
pub const TEMP_FRESH: Duration = Duration::from_secs(24 * 60 * 60);

/// Tests and a careful first run point every group inside this folder: `<root>/temp`,
/// `<root>/browser`, `<root>/thumbs`, `<root>/crash`, `<root>/deep`. The Recycle Bin is then not offered.
pub const ROOT_VAR: &str = "CASTLE_SWEEP_ROOT";

/// The log beside Castle's settings.
pub const LOG_FILE: &str = "sweep-log.jsonl";

/// One folder to clean and the rule it follows.
#[derive(Debug, Clone)]
pub struct Root {
    pub path: PathBuf,
    /// Temp's rule: fresh files and the `claude` scratchpads stay.
    pub temp: bool,
    /// Only files whose name starts with this (the thumbnail cache), or every file.
    pub prefix: Option<&'static str>,
    /// Why this root is skipped now (a browser is open), or None.
    pub skip: Option<String>,
}

impl Root {
    fn plain(path: PathBuf) -> Self {
        Root { path, temp: false, prefix: None, skip: None }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub bytes: u64,
    pub files: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GroupSize {
    pub id: String,
    pub label: String,
    pub bytes: u64,
    pub files: u64,
    /// "Chrome is open - its cache is skipped", or ''.
    pub note: String,
}

// ---------- the rules (pure, tested) ----------

/// A reparse point (a symlink or a junction) is never entered or deleted through.
fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// Temp's keep rule for a path under the Temp root: the Claude Code sessions' scratchpads.
pub fn temp_keeps(rel: &Path) -> bool {
    let Some(first) = rel.components().next() else { return true };
    first.as_os_str().to_string_lossy().eq_ignore_ascii_case("claude")
}

fn fresh(meta: &std::fs::Metadata, now: SystemTime) -> bool {
    meta.modified().ok().and_then(|m| now.duration_since(m).ok()).is_none_or(|age| age < TEMP_FRESH)
}

/// Walks one root: counts (and, unless `dry`, deletes) what the rule allows. Never leaves the root,
/// never follows a link, never removes the root itself, counts a file that cannot be deleted as
/// skipped. `progress` hears the bytes freed so far.
pub fn sweep_root(root: &Root, dry: bool, now: SystemTime, progress: &mut dyn FnMut(u64)) -> Tally {
    let mut tally = Tally::default();
    if root.skip.is_some() {
        return tally;
    }
    let Ok(meta) = std::fs::symlink_metadata(&root.path) else { return tally };
    if is_link(&meta) || !meta.is_dir() {
        return tally;
    }
    walk(root, &root.path, dry, now, &mut tally, progress);
    tally
}

fn walk(root: &Root, dir: &Path, dry: bool, now: SystemTime, tally: &mut Tally, progress: &mut dyn FnMut(u64)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.starts_with(&root.path) {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if is_link(&meta) {
            continue;
        }
        let rel = path.strip_prefix(&root.path).unwrap_or(&path);
        if root.temp && temp_keeps(rel) {
            continue;
        }
        if meta.is_dir() {
            walk(root, &path, dry, now, tally, progress);
            if !dry {
                // an emptied folder goes too (one still holding a skipped file stays)
                let _ = std::fs::remove_dir(&path);
            }
            continue;
        }
        if let Some(prefix) = root.prefix {
            if !entry.file_name().to_string_lossy().to_lowercase().starts_with(prefix) {
                continue;
            }
        }
        if root.temp && fresh(&meta, now) {
            continue;
        }
        if dry {
            tally.bytes += meta.len();
            tally.files += 1;
        } else if std::fs::remove_file(&path).is_ok() {
            tally.bytes += meta.len();
            tally.files += 1;
            progress(tally.bytes);
        } else {
            tally.skipped += 1;
        }
    }
}

// ---------- the folders on this computer ----------

fn dir_of(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|p| p.is_dir())
}

fn env_dir(name: &str) -> Option<PathBuf> {
    dir_of(std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from))
}

/// The test root, when one is set.
fn test_root() -> Option<PathBuf> {
    std::env::var_os(ROOT_VAR).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The browsers that are running now, by the names given (lowercase).
fn running(names: &[&str]) -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let Ok(mut system) = System::new() else { return Vec::new() };
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    system.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).filter(|n| names.contains(&n.as_str())).collect()
}

/// Whether a group is offered on this computer (the sheet shows only these).
pub fn offered(group: &str) -> bool {
    offered_on(group, std::env::consts::OS, test_root().is_some())
}

fn offered_on(group: &str, os: &str, test: bool) -> bool {
    match group {
        "temp" | "browser" | "crash" => os == "windows" || os == "macos" || test,
        "thumbs" => os == "windows" || test,
        // never the real Recycle Bin while every other group points at a test folder
        "recycle" => os == "windows" && !test,
        _ => false,
    }
}

/// The profiles under a browser's data folder, each joined with the cache sub-folders that exist.
fn profile_caches(base: &Path, subs: &[PathBuf], skip: &Option<String>) -> Vec<Root> {
    let mut out = Vec::new();
    for profile in std::fs::read_dir(base).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        for sub in subs {
            let path = profile.join(sub);
            if path.is_dir() {
                out.push(Root { path, temp: false, prefix: None, skip: skip.clone() });
            }
        }
    }
    out
}

/// The roots of one group on this computer.
pub fn roots(group: &str) -> Vec<Root> {
    if let Some(test) = test_root() {
        return test_roots(&test, group);
    }
    #[cfg(windows)]
    return windows_roots(group);
    #[cfg(target_os = "macos")]
    return mac_roots(group);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = group;
        Vec::new()
    }
}

fn test_roots(test: &Path, group: &str) -> Vec<Root> {
    let path = test.join(group);
    match group {
        "temp" => vec![Root { path, temp: true, prefix: None, skip: None }],
        "browser" | "crash" => vec![Root::plain(path)],
        "thumbs" => vec![Root { path, temp: false, prefix: Some("thumbcache_"), skip: None }],
        _ => Vec::new(),
    }
}

#[cfg(windows)]
fn windows_roots(group: &str) -> Vec<Root> {
    let local = env_dir("LOCALAPPDATA");
    match group {
        "temp" => env_dir("TEMP").map(|path| vec![Root { path, temp: true, prefix: None, skip: None }]).unwrap_or_default(),
        "browser" => {
            let Some(local) = local else { return Vec::new() };
            let open = running(&["chrome.exe", "msedge.exe", "brave.exe", "firefox.exe"]);
            let chromium_subs = [Path::new("Cache").join("Cache_Data"), PathBuf::from("Code Cache"), PathBuf::from("GPUCache")];
            let mut out = Vec::new();
            for (base, exe, name) in [
                (local.join("Google").join("Chrome").join("User Data"), "chrome.exe", "Chrome"),
                (local.join("Microsoft").join("Edge").join("User Data"), "msedge.exe", "Edge"),
                (local.join("BraveSoftware").join("Brave-Browser").join("User Data"), "brave.exe", "Brave"),
            ] {
                let skip = open.iter().any(|n| n == exe).then(|| format!("{name} is open - its cache is skipped"));
                out.extend(profile_caches(&base, &chromium_subs, &skip));
            }
            let skip = open.iter().any(|n| n == "firefox.exe").then(|| "Firefox is open - its cache is skipped".to_string());
            out.extend(profile_caches(&local.join("Mozilla").join("Firefox").join("Profiles"), &[PathBuf::from("cache2")], &skip));
            out
        }
        "thumbs" => local.map(|l| vec![Root { path: l.join("Microsoft").join("Windows").join("Explorer"), temp: false, prefix: Some("thumbcache_"), skip: None }]).unwrap_or_default(),
        "crash" => local
            .map(|l| {
                let wer = l.join("Microsoft").join("Windows").join("WER");
                vec![Root::plain(l.join("CrashDumps")), Root::plain(wer.join("ReportArchive")), Root::plain(wer.join("ReportQueue"))]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "macos")]
fn mac_roots(group: &str) -> Vec<Root> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let caches = home.as_ref().map(|h| h.join("Library").join("Caches"));
    match group {
        "temp" => env_dir("TMPDIR").map(|path| vec![Root { path, temp: true, prefix: None, skip: None }]).unwrap_or_default(),
        "browser" => {
            let Some(caches) = caches else { return Vec::new() };
            let open = running(&["google chrome", "microsoft edge", "brave browser", "firefox"]);
            let mut out = Vec::new();
            // everything under ~/Library/Caches is a cache by definition: each browser's folder whole
            for (path, process, name) in [
                (caches.join("Google").join("Chrome"), "google chrome", "Chrome"),
                (caches.join("Microsoft Edge"), "microsoft edge", "Edge"),
                (caches.join("BraveSoftware").join("Brave-Browser"), "brave browser", "Brave"),
            ] {
                if path.is_dir() {
                    let skip = open.iter().any(|n| n == process).then(|| format!("{name} is open - its cache is skipped"));
                    out.push(Root { path, temp: false, prefix: None, skip });
                }
            }
            let skip = open.iter().any(|n| n == "firefox").then(|| "Firefox is open - its cache is skipped".to_string());
            out.extend(profile_caches(&caches.join("Firefox").join("Profiles"), &[PathBuf::from("cache2")], &skip));
            out
        }
        "crash" => home.map(|h| vec![Root::plain(h.join("Library").join("Logs").join("DiagnosticReports"))]).unwrap_or_default(),
        _ => Vec::new(),
    }
}

// ---------- measure and clean ----------

fn label(id: &str) -> String {
    GROUPS.iter().find(|(g, _)| *g == id).map(|(_, l)| l.to_string()).unwrap_or_default()
}

fn note_of(roots: &[Root]) -> String {
    let mut notes: Vec<String> = Vec::new();
    for why in roots.iter().filter_map(|r| r.skip.clone()) {
        if !notes.contains(&why) {
            notes.push(why);
        }
    }
    notes.join("; ")
}

/// What each offered group could free now; nothing is deleted.
pub fn measure() -> Vec<GroupSize> {
    let now = SystemTime::now();
    GROUPS
        .iter()
        .filter(|(id, _)| offered(id))
        .map(|(id, _)| {
            if *id == "recycle" {
                let (bytes, files) = recycle_size();
                return GroupSize { id: id.to_string(), label: label(id), bytes, files, note: String::new() };
            }
            let roots = roots(id);
            let mut size = GroupSize { id: id.to_string(), label: label(id), bytes: 0, files: 0, note: note_of(&roots) };
            for root in &roots {
                let t = sweep_root(root, true, now, &mut |_| {});
                size.bytes += t.bytes;
                size.files += t.files;
            }
            size
        })
        .collect()
}

/// One group's result.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cleaned {
    pub id: String,
    pub freed: u64,
    pub files: u64,
    pub skipped: u64,
}

/// Cleans the ticked groups one after another; `progress(group, freed so far in all)` as files go.
/// A group that is not offered here is passed over.
pub fn clean(ids: &[String], log_folder: &Path, progress: &mut dyn FnMut(&str, u64)) -> Vec<Cleaned> {
    let now = SystemTime::now();
    let mut done = Vec::new();
    let mut total = 0u64;
    for (id, _) in GROUPS.iter().filter(|(g, _)| ids.iter().any(|i| i == g) && offered(g)) {
        let mut result = Cleaned { id: id.to_string(), freed: 0, files: 0, skipped: 0 };
        if *id == "recycle" {
            let (bytes, files) = recycle_size();
            if recycle_empty() {
                result.freed = bytes;
                result.files = files;
            } else if files > 0 {
                result.skipped = files;
            }
            total += result.freed;
            progress(id, total);
        } else {
            for root in roots(id) {
                let base = total + result.freed;
                let t = sweep_root(&root, false, now, &mut |b| progress(id, base + b));
                result.freed += t.bytes;
                result.files += t.files;
                result.skipped += t.skipped;
            }
            total += result.freed;
        }
        log(log_folder, &result);
        done.push(result);
    }
    done
}

// ---------- the Recycle Bin (the shell's own count and empty) ----------

#[cfg(windows)]
fn recycle_size() -> (u64, u64) {
    use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
    // SAFETY: a null root asks about every drive; `info` is a live, sized struct for the whole call.
    let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if hr == 0 {
        (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64)
    } else {
        (0, 0)
    }
}

#[cfg(windows)]
fn recycle_empty() -> bool {
    use windows_sys::Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};
    // SAFETY: no window, a null root (every drive), flags only.
    unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND) == 0 }
}

#[cfg(not(windows))]
fn recycle_size() -> (u64, u64) {
    (0, 0)
}

#[cfg(not(windows))]
fn recycle_empty() -> bool {
    false
}

// ---------- Deep clean (Windows: the system's Temp and Windows Update downloads) ----------

/// `memorycore-castle.exe --sweep-deep <result file>`: the elevated Castle runs the same safe walk
/// over the system folders, writes its result and ends, no window (handled first in `lib.rs::run`).
pub const DEEP_FLAG: &str = "--sweep-deep";

/// The system folders Deep clean empties (inside the test root's `deep` folder while one is set).
pub fn deep_roots() -> Vec<Root> {
    if let Some(test) = test_root() {
        return vec![Root { path: test.join("deep"), temp: true, prefix: None, skip: None }];
    }
    #[cfg(windows)]
    {
        let system = env_dir("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        vec![Root { path: system.join("Temp"), temp: true, prefix: None, skip: None }, Root::plain(system.join("SoftwareDistribution").join("Download"))]
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// Runs the walk over `roots` and writes the result as JSON into `file`.
fn deep_into(roots: &[Root], file: &Path) -> Cleaned {
    let now = SystemTime::now();
    let mut result = Cleaned { id: "deep".into(), freed: 0, files: 0, skipped: 0 };
    for root in roots {
        let t = sweep_root(root, false, now, &mut |_| {});
        result.freed += t.bytes;
        result.files += t.files;
        result.skipped += t.skipped;
    }
    let _ = std::fs::write(file, serde_json::to_string(&result).unwrap_or_default());
    result
}

/// What an elevated run left in its result file.
fn read_result(file: &Path) -> Option<Cleaned> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// The `--sweep-deep <result file>` entry: the exit code.
pub fn run_deep_cli(args: &[String]) -> i32 {
    let Some(file) = args.first() else { return 2 };
    deep_into(&deep_roots(), Path::new(file));
    0
}

/// Deep clean, on the member's press: Windows asks once for administrator rights and an elevated
/// Castle cleans the system folders with the same safe walk. `Ok(None)` when the member said no.
/// With a test root set it runs here, unelevated, on the test folder (an elevated process would not
/// inherit the test root and would clean the real folders).
pub fn deep_clean(log_folder: &Path) -> Result<Option<Cleaned>, String> {
    let file = std::env::temp_dir().join(format!("castle-sweep-deep-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let result = if test_root().is_some() {
        Some(deep_into(&deep_roots(), &file))
    } else {
        match elevate_deep(&file)? {
            false => None,
            true => Some(read_result(&file).ok_or("Deep clean ran but left no result.")?),
        }
    };
    let _ = std::fs::remove_file(&file);
    if let Some(r) = &result {
        log(log_folder, r);
    }
    Ok(result)
}

/// Starts Castle itself elevated with `--sweep-deep <file>` and waits for it; false when the member
/// refused the prompt.
#[cfg(windows)]
fn elevate_deep(file: &Path) -> Result<bool, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED};
    use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let exe = std::env::current_exe().map_err(|_| "Castle could not find itself to ask for administrator rights.".to_string())?;
    let verb = wide(std::ffi::OsStr::new("runas"));
    let program = wide(exe.as_os_str());
    let params = wide(std::ffi::OsStr::new(&format!("{DEEP_FLAG} \"{}\"", file.display())));
    // SAFETY: a zeroed SHELLEXECUTEINFOW is its documented empty state; every pointer set below
    // points into a Vec that outlives the call.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = program.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = SW_HIDE;
    // SAFETY: `info` is fully set up above.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        // SAFETY: reads this thread's last error, nothing else.
        return if unsafe { GetLastError() } == ERROR_CANCELLED { Ok(false) } else { Err("Windows could not start Deep clean.".into()) };
    }
    if !info.hProcess.is_null() {
        // SAFETY: the process handle was given to us by ShellExecuteExW (SEE_MASK_NOCLOSEPROCESS) and is closed once.
        unsafe {
            WaitForSingleObject(info.hProcess, INFINITE);
            CloseHandle(info.hProcess);
        }
    }
    Ok(true)
}

#[cfg(not(windows))]
fn elevate_deep(_file: &Path) -> Result<bool, String> {
    Err("Deep clean is for Windows.".into())
}

// ---------- the log ----------

fn unix_now() -> u64 {
    SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn log(folder: &Path, c: &Cleaned) {
    use std::io::Write;
    let line = serde_json::json!({ "unix": unix_now(), "group": c.id, "freed": c.freed, "files": c.files, "skipped": c.skipped });
    let _ = std::fs::create_dir_all(folder);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(folder.join(LOG_FILE)) {
        let _ = writeln!(f, "{line}");
    }
}

/// When the last sweep ran (unix seconds), or None.
pub fn last_swept(folder: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(folder.join(LOG_FILE)).ok()?;
    text.lines().rev().find_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()?.get("unix")?.as_u64())
}

// ---------- the gear and the commands ----------

/// The three looks of the widget (the three kept designs).
pub const LOOKS: [&str; 3] = ["ring", "bar", "broom"];

/// Sweep's gear: the look chosen by hand (None: the theme's, else the bar) and whether the Recycle
/// Bin starts ticked in the sheet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SweepSettings {
    pub look: Option<String>,
    pub recycle_bin: bool,
}

/// A look Castle knows, or None.
pub fn clean_look(look: Option<String>) -> Option<String> {
    look.map(|l| l.trim().to_lowercase()).filter(|l| LOOKS.contains(&l.as_str()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepStatus {
    pub settings: SweepSettings,
    pub last_swept: Option<u64>,
    /// Deep clean is offered here (Windows, or a test root).
    pub deep: bool,
}

/// One sweep at a time (a click while one runs is told so).
static RUNNING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn status_of(store: &crate::settings::SettingsStore) -> SweepStatus {
    SweepStatus { settings: store.get().sweep, last_swept: last_swept(&store.folder()), deep: cfg!(windows) || test_root().is_some() }
}

#[tauri::command]
pub fn sweep_status(store: tauri::State<'_, crate::settings::SettingsStore>) -> SweepStatus {
    status_of(&store)
}

/// What can be freed now (a walk of every offered group: off the page's thread; nothing deleted).
#[tauri::command]
pub async fn sweep_measure() -> Result<Vec<GroupSize>, String> {
    tauri::async_runtime::spawn_blocking(measure).await.map_err(|_| "Sweep could not measure.".to_string())
}

/// Cleans the ticked groups; the page hears `castle://sweep` { group, freed } as files go.
#[tauri::command]
pub async fn sweep_clean(app: tauri::AppHandle, groups: Vec<String>) -> Result<Vec<Cleaned>, String> {
    use tauri::{Emitter, Manager};
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(_one) = RUNNING.try_lock() else { return Err("A sweep is already running.".to_string()) };
        let folder = app.state::<crate::settings::SettingsStore>().folder();
        let mut last = std::time::Instant::now() - Duration::from_secs(1);
        Ok(clean(&groups, &folder, &mut |group, freed| {
            // a few times a second is enough for the counter
            if last.elapsed() > Duration::from_millis(150) {
                let _ = app.emit("castle://sweep", serde_json::json!({ "group": group, "freed": freed }));
                last = std::time::Instant::now();
            }
        }))
    })
    .await
    .map_err(|_| "The sweep stopped.".to_string())?
}

/// Deep clean on the member's press; None when the administrator prompt was refused.
#[tauri::command]
pub async fn sweep_deep(app: tauri::AppHandle) -> Result<Option<Cleaned>, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(_one) = RUNNING.try_lock() else { return Err("A sweep is already running.".to_string()) };
        deep_clean(&app.state::<crate::settings::SettingsStore>().folder())
    })
    .await
    .map_err(|_| "Deep clean stopped.".to_string())?
}

/// The gear's choices, saved alone (the Settings page never overwrites them).
#[tauri::command]
pub fn sweep_settings_save(store: tauri::State<'_, crate::settings::SettingsStore>, look: Option<String>, recycle_bin: bool) -> Result<SweepStatus, String> {
    let mut next = store.get();
    next.sweep = SweepSettings { look: clean_look(look), recycle_bin };
    crate::settings::save(&store.path, &next).map_err(|_| "Sweep's settings could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    Ok(status_of(&store))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_looks_are_kept() {
        assert_eq!(clean_look(Some(" Broom ".into())), Some("broom".into()));
        assert_eq!(clean_look(Some("ring".into())), Some("ring".into()));
        assert_eq!(clean_look(Some("sparkle".into())), None);
        assert_eq!(clean_look(None), None);
        let older: SweepSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(older, SweepSettings { look: None, recycle_bin: false }, "an older settings file reads as the defaults");
    }

    fn tree(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("castle-sweep-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn old(path: &Path) {
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(3 * 24 * 3600)).unwrap();
    }

    fn write(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    #[test]
    fn temp_keeps_the_sessions_and_anything_fresh_and_never_the_root() {
        let base = tree("temp");
        let root = Root { path: base.clone(), temp: true, prefix: None, skip: None };
        for (rel, bytes, aged) in [("junk.tmp", 100, true), ("deep/old.log", 50, true), ("new.tmp", 7, false), ("claude/session/scratch.md", 9, true)] {
            write(&base.join(rel), bytes);
            if aged {
                old(&base.join(rel));
            }
        }
        // measuring deletes nothing
        let dry = sweep_root(&root, true, SystemTime::now(), &mut |_| {});
        assert_eq!((dry.bytes, dry.files), (150, 2));
        assert!(base.join("junk.tmp").exists());
        let done = sweep_root(&root, false, SystemTime::now(), &mut |_| {});
        assert_eq!((done.bytes, done.files, done.skipped), (150, 2, 0));
        assert!(!base.join("junk.tmp").exists() && !base.join("deep").exists(), "the old file and its emptied folder go");
        assert!(base.join("new.tmp").exists(), "fresh stays");
        assert!(base.join("claude/session/scratch.md").exists(), "a session's scratchpad stays");
        assert!(base.is_dir(), "the root itself is never removed");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn a_prefix_root_takes_only_its_own_files_and_a_skipped_root_nothing() {
        let base = tree("thumbs");
        write(&base.join("thumbcache_256.db"), 30);
        write(&base.join("iconcache_16.db"), 30);
        let root = Root { path: base.clone(), temp: false, prefix: Some("thumbcache_"), skip: None };
        assert_eq!(sweep_root(&root, false, SystemTime::now(), &mut |_| {}).bytes, 30);
        assert!(base.join("iconcache_16.db").exists());
        let skipped = Root { skip: Some("Chrome is open".into()), ..root };
        write(&base.join("thumbcache_32.db"), 5);
        assert_eq!(sweep_root(&skipped, false, SystemTime::now(), &mut |_| {}), Tally::default());
        assert!(base.join("thumbcache_32.db").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[cfg(windows)]
    #[test]
    fn a_file_in_use_is_skipped_and_a_junction_is_never_followed() {
        use std::os::windows::fs::OpenOptionsExt;
        let base = tree("locked");
        let cache = base.join("cache");
        write(&cache.join("busy.bin"), 10);
        write(&cache.join("free.bin"), 20);
        // a folder outside the root, reached through a junction inside it
        let outside = base.join("outside");
        write(&outside.join("precious.txt"), 99);
        let made = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(cache.join("link")).arg(&outside).output().map(|o| o.status.success()).unwrap_or(false);
        let busy = std::fs::OpenOptions::new().read(true).share_mode(0).open(cache.join("busy.bin")).unwrap();
        let root = Root::plain(cache.clone());
        let done = sweep_root(&root, false, SystemTime::now(), &mut |_| {});
        drop(busy);
        assert_eq!((done.bytes, done.files, done.skipped), (20, 1, 1));
        assert!(outside.join("precious.txt").exists(), "nothing behind a junction is touched");
        if made {
            assert!(cache.join("link").exists(), "the junction itself stays");
        }
        let _ = std::fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_never_followed() {
        let base = tree("symlink");
        let cache = base.join("cache");
        write(&cache.join("free.bin"), 20);
        let outside = base.join("outside");
        write(&outside.join("precious.txt"), 99);
        std::os::unix::fs::symlink(&outside, cache.join("link")).unwrap();
        let done = sweep_root(&Root::plain(cache.clone()), false, SystemTime::now(), &mut |_| {});
        assert_eq!((done.bytes, done.files), (20, 1));
        assert!(outside.join("precious.txt").exists(), "nothing behind a link is touched");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn each_system_offers_only_its_own_groups_and_a_test_root_never_the_real_bin() {
        let on = |os: &str, test: bool| GROUPS.iter().map(|(g, _)| *g).filter(|g| offered_on(g, os, test)).collect::<Vec<_>>();
        assert_eq!(on("windows", false), ["temp", "browser", "thumbs", "crash", "recycle"]);
        assert_eq!(on("macos", false), ["temp", "browser", "crash"]);
        assert_eq!(on("windows", true), ["temp", "browser", "thumbs", "crash"], "the Recycle Bin is never offered beside a test root");
        assert!(on("linux", false).is_empty());
    }

    #[test]
    fn a_test_root_points_every_group_inside_it() {
        let base = Path::new("/scratch");
        assert_eq!(test_roots(base, "temp")[0].path, base.join("temp"));
        assert!(test_roots(base, "temp")[0].temp);
        assert_eq!(test_roots(base, "thumbs")[0].prefix, Some("thumbcache_"));
        assert!(test_roots(base, "recycle").is_empty());
    }

    #[test]
    fn a_deep_run_writes_a_result_the_caller_reads_back() {
        let base = tree("deep");
        let system_temp = base.join("Temp");
        write(&system_temp.join("old.etl"), 40);
        old(&system_temp.join("old.etl"));
        write(&system_temp.join("today.etl"), 3);
        let downloads = base.join("Download");
        write(&downloads.join("kb123/update.cab"), 60);
        let file = base.join("result.json");
        let roots = [Root { path: system_temp.clone(), temp: true, prefix: None, skip: None }, Root::plain(downloads.clone())];
        let ran = deep_into(&roots, &file);
        assert_eq!((ran.freed, ran.files, ran.skipped), (100, 2, 0));
        assert_eq!(read_result(&file), Some(ran));
        assert!(system_temp.join("today.etl").exists(), "the system's fresh temp files stay");
        assert!(system_temp.is_dir() && downloads.is_dir(), "the system folders themselves stay");
        assert_eq!(read_result(&base.join("missing.json")), None);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn the_deep_entry_needs_its_result_file() {
        assert_eq!(run_deep_cli(&[]), 2);
    }

    #[test]
    fn the_log_says_when_it_last_swept() {
        let base = tree("log");
        assert_eq!(last_swept(&base), None);
        log(&base, &Cleaned { id: "temp".into(), freed: 5, files: 1, skipped: 0 });
        assert!(last_swept(&base).is_some());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    #[ignore = "measures this computer (nothing is deleted)"]
    fn real_measure() {
        for g in measure() {
            println!("{:<26} {:>12} bytes {:>7} files {}", g.label, g.bytes, g.files, g.note);
        }
    }
}
