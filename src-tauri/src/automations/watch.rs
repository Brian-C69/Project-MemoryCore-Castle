//! Phase 5: the file trigger. A folder is looked at every couple of seconds (no extra dependency,
//! no network paths); a file that matches fires its job once, after it has stopped changing.
//! The file's name and path are DATA in the payload - they never choose a program or an argument.
use super::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

static PROBLEM: Mutex<Option<String>> = Mutex::new(None);
pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}

const SCAN_EVERY: Duration = Duration::from_secs(2);
const JOBS_EVERY: Duration = Duration::from_secs(10);
/// Most files one folder scan looks at, and most runs one scan may start for one job.
const MAX_FILES: usize = 2000;
const MAX_FIRES_PER_SCAN: usize = 5;
pub const MAX_JOBS: u64 = 16;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub folder: String,
    #[serde(default = "any_name")]
    pub glob: String,
    #[serde(default = "created")]
    pub event: String,
    #[serde(default = "five")]
    pub settle_s: u64,
}
fn any_name() -> String {
    "*".into()
}
fn created() -> String {
    "created".into()
}
fn five() -> u64 {
    5
}

/// Checks the shape of the trigger only; whether the folder exists is looked at by `check_folder`
/// (when the job is saved) and by the watcher itself (which reports it).
pub fn spec(v: &Value) -> Result<File, String> {
    let s: File = serde_json::from_value(v.clone()).map_err(|_| "Check the file trigger fields.")?;
    if !["created", "changed"].contains(&s.event.as_str()) {
        return Err("Choose whether to run when a file appears or when it changes.".into());
    }
    if !(1..=300).contains(&s.settle_s) {
        return Err("Wait between 1 and 300 seconds for a file to stop changing.".into());
    }
    if s.glob.is_empty() || s.glob.len() > 64 || s.glob.chars().any(|c| c.is_control() || "/\\:|<>\"".contains(c)) {
        return Err("Name pattern: use letters, digits, * and ? only (for example *.pdf), up to 64 characters.".into());
    }
    folder_shape(&s.folder)?;
    Ok(s)
}
fn folder_shape(folder: &str) -> Result<(), String> {
    let p = Path::new(folder);
    let network = folder.starts_with("\\\\") || folder.starts_with("//");
    if folder.trim().is_empty() || folder.contains('\0') || folder.len() > 400 || !p.is_absolute() || network {
        return Err("Choose a folder on this computer (network folders are not supported).".into());
    }
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("The folder path may not contain \"..\".".into());
    }
    Ok(())
}
pub fn check_folder(s: &File) -> Result<(), String> {
    if Path::new(&s.folder).is_dir() {
        Ok(())
    } else {
        Err("That folder does not exist.".into())
    }
}

/// `*` (any run of characters) and `?` (one character), ignoring case. Nothing else is special.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while j < n.len() {
        if i < p.len() && (p[i] == '?' || p[i] == n[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some(i);
            mark = j;
            i += 1;
        } else if let Some(s) = star {
            i = s + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

/// What is known about one file: its last seen (size, modified) and when that last changed.
#[derive(Clone, Debug)]
struct Seen {
    sig: (u64, i128),
    since_ms: u64,
    /// The signature already handled (fired, or present before the watch began).
    done: Option<(u64, i128)>,
}
#[derive(Default)]
pub struct Tracker {
    files: HashMap<String, Seen>,
    baselined: bool,
}
impl Tracker {
    /// `files` is what the folder holds now; `now_ms` is any steady clock. Returns the names that
    /// are ready to fire. Files already there when watching began never fire, and a file fires
    /// once per appearance (once per change for `changed`), after `settle_s` of stillness.
    pub fn observe(&mut self, files: &[(String, u64, i128)], now_ms: u64, s: &File) -> Vec<String> {
        let first = !self.baselined;
        self.baselined = true;
        let live: std::collections::HashSet<&str> = files.iter().map(|f| f.0.as_str()).collect();
        self.files.retain(|name, _| live.contains(name.as_str()));
        let mut ready = Vec::new();
        for (name, size, modified) in files {
            let sig = (*size, *modified);
            match self.files.get_mut(name) {
                None => {
                    self.files.insert(name.clone(), Seen { sig, since_ms: now_ms, done: first.then_some(sig) });
                }
                Some(seen) => {
                    if seen.sig != sig {
                        seen.sig = sig;
                        seen.since_ms = now_ms;
                        // a change to a file that already fired or pre-existed only counts for "changed"
                        if s.event == "changed" {
                            seen.done = None;
                        } else if seen.done.is_some() {
                            seen.done = Some(sig);
                        }
                    }
                }
            }
            let seen = &self.files[name];
            if seen.done.is_none() && now_ms.saturating_sub(seen.since_ms) >= s.settle_s * 1000 {
                ready.push(name.clone());
            }
        }
        ready.sort();
        ready.truncate(MAX_FIRES_PER_SCAN);
        for name in &ready {
            if let Some(seen) = self.files.get_mut(name) {
                seen.done = Some(seen.sig);
            }
        }
        ready
    }
}

fn read_folder(s: &File) -> Result<Vec<(String, u64, i128)>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&s.folder).map_err(|_| "The folder cannot be read.".to_string())?.flatten() {
        if out.len() >= MAX_FILES {
            break;
        }
        // a link or folder is never a file to run on; only plain files
        let Ok(meta) = std::fs::symlink_metadata(entry.path()) else { continue };
        if !meta.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
        if !glob_match(&s.glob, &name) {
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as i128);
        out.push((name, meta.len(), modified));
    }
    Ok(out)
}
/// A file another program is still writing is held until it can be opened for reading.
fn readable(folder: &str, name: &str) -> bool {
    std::fs::File::open(PathBuf::from(folder).join(name)).is_ok()
}

fn payload(s: &File, name: &str) -> Value {
    let path = PathBuf::from(&s.folder).join(name).to_string_lossy().into_owned();
    json!({"cause":"file","text":name,"name":name,"path":path,"event":s.event,"time":schedule::stamp(chrono::Local::now())})
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let clock = Instant::now();
        let mut trackers: HashMap<u64, (String, Tracker)> = HashMap::new();
        let mut jobs: Vec<Value> = Vec::new();
        let mut loaded = Instant::now() - JOBS_EVERY;
        let mut generation=health::generation();let mut last_tick=chrono::Utc::now().timestamp_millis();
        loop {
            std::thread::sleep(SCAN_EVERY);
            let now=chrono::Utc::now().timestamp_millis();
            if generation!=health::generation()||now<last_tick||now-last_tick>45000{trackers.clear();jobs.clear();loaded=Instant::now()-JOBS_EVERY;generation=health::generation();}
            last_tick=now;
            let settings = app.state::<SettingsStore>();
            if !allowed(&settings.get()) {
                trackers.clear();
                *PROBLEM.lock().unwrap() = None;
                continue;
            }
            if loaded.elapsed() >= JOBS_EVERY {
                loaded = Instant::now();
                match connection(&settings).and_then(|mut c|store::list(&mut c)) {
                    Ok(list)=>{jobs=list.into_iter().filter(|j|j["state"]=="active" && j["enabled"]==1 && j["triggerKind"]=="file").collect();*PROBLEM.lock().unwrap()=None;},
                    Err(e)=>{jobs.clear();trackers.clear();*PROBLEM.lock().unwrap()=Some(e);continue;}
                }
            }
            if jobs.is_empty(){continue;}
            let live: Vec<u64> = jobs.iter().filter_map(|j| j["id"].as_u64()).collect();
            trackers.retain(|id, _| live.contains(id));
            let mut problems = Vec::new();
            for j in &jobs {
                let Some(id) = j["id"].as_u64() else { continue };
                let Ok(s) = spec(&j["trigger"]) else { continue };
                // a changed trigger starts again from what is in the folder now
                let fingerprint = j["trigger"].to_string();
                let entry = trackers.entry(id).or_insert_with(|| (fingerprint.clone(), Tracker::default()));
                if entry.0 != fingerprint {
                    *entry = (fingerprint, Tracker::default());
                }
                let files = match read_folder(&s) {
                    Ok(f) => f,
                    Err(e) => {
                        problems.push(format!("{}: {e}", j["name"].as_str().unwrap_or("A job")));
                        continue;
                    }
                };
                for name in entry.1.observe(&files, clock.elapsed().as_millis() as u64, &s) {
                    if !allowed(&settings.get()) {
                        break;
                    }
                    if !readable(&s.folder, &name) {
                        // still being written: look again next time instead of losing it
                        if let Some(seen) = entry.1.files.get_mut(&name) {
                            seen.done = None;
                            seen.since_ms = clock.elapsed().as_millis() as u64;
                        }
                        continue;
                    }
                    if let Err(e)=runner::launch_payload(&app,id,"file",payload(&s,&name),Some(j)){problems.push(e);}
                }
            }
            *PROBLEM.lock().unwrap() = (!problems.is_empty()).then(|| problems.join(" "));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn s(event: &str) -> File {
        File { folder: "C:/drop".into(), glob: "*.pdf".into(), event: event.into(), settle_s: 5 }
    }
    fn one(name: &str, size: u64, modified: i128) -> Vec<(String, u64, i128)> {
        vec![(name.into(), size, modified)]
    }

    #[test]
    fn globs_match_names_only_and_ignore_case() {
        assert!(glob_match("*.pdf", "Invoice.PDF"));
        assert!(glob_match("inv?ice*", "invoice-3.pdf"));
        assert!(glob_match("*", "a&calc.pdf"));
        assert!(!glob_match("*.pdf", "x.pdf.exe"));
        assert!(!glob_match("*.pdf", "pdf"));
        assert!(glob_match("a*b*c", "aXXbYYc"));
        assert!(!glob_match("a*b*c", "aXXbYY"));
    }

    #[test]
    fn the_shape_of_a_file_trigger_is_checked() {
        let ok = json!({"folder":"C:/drop","glob":"*.pdf","event":"created","settle_s":5});
        #[cfg(windows)]
        assert!(spec(&ok).is_ok());
        for bad in [
            json!({"folder":"C:/drop","glob":"*.pdf","event":"deleted"}),
            json!({"folder":"C:/drop","glob":"*.pdf","settle_s":0}),
            json!({"folder":"C:/drop","glob":"*.pdf","settle_s":301}),
            json!({"folder":"C:/drop","glob":"..\\*.pdf"}),
            json!({"folder":"C:/drop","glob":"a/b"}),
            json!({"folder":"C:/drop","glob":""}),
            json!({"folder":"relative/path"}),
            json!({"folder":""}),
            json!({"folder":"\\\\server\\share\\drop"}),
            json!({"folder":"//server/share/drop"}),
            json!({"folder":"C:/drop/../secret"}),
            json!({"folder":"C:/drop","program":"calc.exe"}),
        ] {
            assert!(spec(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_file_already_there_never_fires_and_a_new_one_fires_once_after_settling() {
        let mut t = Tracker::default();
        let f = s("created");
        assert!(t.observe(&one("old.pdf", 10, 1), 0, &f).is_empty());
        assert!(t.observe(&one("old.pdf", 10, 1), 60_000, &f).is_empty());
        let both = vec![("old.pdf".into(), 10, 1), ("new.pdf".into(), 5, 9)];
        assert!(t.observe(&both, 61_000, &f).is_empty()); // just appeared
        assert!(t.observe(&both, 64_000, &f).is_empty()); // not settled
        assert_eq!(t.observe(&both, 66_000, &f), vec!["new.pdf"]);
        assert!(t.observe(&both, 90_000, &f).is_empty()); // once
    }

    #[test]
    fn a_file_still_growing_waits_and_does_not_fire_half_written() {
        let mut t = Tracker::default();
        let f = s("created");
        t.observe(&[], 0, &f);
        assert!(t.observe(&one("a.pdf", 1, 1), 10_000, &f).is_empty());
        assert!(t.observe(&one("a.pdf", 2, 2), 14_000, &f).is_empty()); // grew: clock restarts
        assert!(t.observe(&one("a.pdf", 3, 3), 18_000, &f).is_empty());
        assert!(t.observe(&one("a.pdf", 3, 3), 22_000, &f).is_empty());
        assert_eq!(t.observe(&one("a.pdf", 3, 3), 23_000, &f), vec!["a.pdf"]);
    }

    #[test]
    fn created_ignores_later_changes_but_changed_runs_again() {
        for (event, again) in [("created", false), ("changed", true)] {
            let mut t = Tracker::default();
            let f = s(event);
            t.observe(&[], 0, &f);
            t.observe(&one("a.pdf", 1, 1), 1_000, &f);
            assert_eq!(t.observe(&one("a.pdf", 1, 1), 7_000, &f), vec!["a.pdf"]);
            t.observe(&one("a.pdf", 9, 9), 20_000, &f);
            let got = t.observe(&one("a.pdf", 9, 9), 26_000, &f);
            assert_eq!(!got.is_empty(), again, "{event}");
        }
    }

    #[test]
    fn a_deleted_and_returned_file_counts_as_new_and_a_flood_is_paced() {
        let mut t = Tracker::default();
        let f = s("created");
        t.observe(&[], 0, &f);
        t.observe(&one("a.pdf", 1, 1), 1_000, &f);
        assert_eq!(t.observe(&one("a.pdf", 1, 1), 7_000, &f), vec!["a.pdf"]);
        t.observe(&[], 8_000, &f);
        t.observe(&one("a.pdf", 1, 1), 9_000, &f);
        assert_eq!(t.observe(&one("a.pdf", 1, 1), 15_000, &f), vec!["a.pdf"]);
        let many: Vec<_> = (0..20).map(|n| (format!("f{n:02}.pdf"), 1, 1)).collect();
        t.observe(&many, 20_000, &f);
        assert_eq!(t.observe(&many, 30_000, &f).len(), MAX_FIRES_PER_SCAN);
        assert_eq!(t.observe(&many, 31_000, &f).len(), MAX_FIRES_PER_SCAN);
    }

    #[test]
    fn a_hostile_file_name_stays_data_in_the_payload() {
        let f = s("created");
        let p = payload(&f, "a&calc; {{payload.text}}.pdf");
        assert_eq!(p["text"], "a&calc; {{payload.text}}.pdf");
        assert_eq!(p["cause"], "file");
        assert!(p["path"].as_str().unwrap().ends_with("calc; {{payload.text}}.pdf"));
    }

    #[test]
    fn a_real_folder_is_read_without_links_folders_or_other_names() {
        let dir = std::env::temp_dir().join(format!("castle-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub.pdf")).unwrap();
        std::fs::write(dir.join("a.pdf"), b"x").unwrap();
        std::fs::write(dir.join("b.txt"), b"x").unwrap();
        let f = File { folder: dir.to_string_lossy().into_owned(), glob: "*.pdf".into(), event: "created".into(), settle_s: 1 };
        let got = read_folder(&f).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "a.pdf");
        assert!(readable(&f.folder, "a.pdf"));
        assert!(check_folder(&f).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(check_folder(&f).is_err());
    }
}
