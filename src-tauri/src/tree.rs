//! FILES in a DEV tab: the project's tree one folder level at a time, a file read to show, and
//! Explorer opened on a file. Only inside the project's folder, never written, and files named
//! like secrets (.env, keys, certificates) are listed but never opened.

use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// A text file larger than this is not shown in the reader.
pub const READ_LIMIT: u64 = 1024 * 1024;
/// Entries listed per level at most.
pub const LEVEL_LIMIT: usize = 2000;
/// What the reader says instead of opening a file named like a secret.
pub const PRIVATE_NOTE: &str = "Kept private - Castle does not open this file here.";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    /// Relative to the project, with forward slashes ("resources/js/app.js").
    pub rel: String,
    pub dir: bool,
    pub size: Option<u64>,
    /// Ignored by the project's .gitignore (shown greyed).
    pub ignored: bool,
    /// Named like a secret: listed, never opened.
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub entries: Vec<Entry>,
    /// Entries past LEVEL_LIMIT, not listed.
    pub more: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Shown {
    Text { text: String, size: u64 },
    Picture { url: String, size: u64 },
    /// Said in one line instead (private, too long, binary).
    Note { note: String, size: u64 },
}

/// A relative path with no way out: only plain folder and file names (no `..`, no drive or
/// root, no backslash tricks). "" is the project itself.
pub fn is_inside_rel(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    !rel.starts_with('/')
        && !rel.contains(':')
        && Path::new(&rel).components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// A file name that usually holds a secret: `.env`, `.env.*`, `*.pem`, `*.key`, `*.pfx`,
/// `*.p12`, `id_rsa*`, `id_ed25519*`, `credentials*` (any case).
pub fn is_private_name(name: &str) -> bool {
    let name = name.to_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || [".pem", ".key", ".pfx", ".p12"].iter().any(|end| name.ends_with(end))
        || ["id_rsa", "id_ed25519", "credentials"].iter().any(|start| name.starts_with(start))
}

/// The real path inside the project for `rel` ("" = the project itself). Two fences: `rel` may
/// hold only plain names, and the resolved path (links followed) must still start with the
/// project's own resolved folder.
pub fn inside(folder: &Path, rel: &str) -> Result<PathBuf, String> {
    if !is_inside_rel(rel) {
        return Err("That is outside the project.".into());
    }
    let root = folder.canonicalize().map_err(|_| "The project folder is not here.".to_string())?;
    let target = if rel.trim_matches(['/', '\\']).is_empty() {
        root.clone()
    } else {
        root.join(rel).canonicalize().map_err(|_| "It is not there any more.".to_string())?
    };
    if target.starts_with(&root) {
        Ok(target)
    } else {
        Err("That is outside the project.".into())
    }
}

/// Folders first, then files, each by name ignoring case (as VS Code sorts).
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

/// `git check-ignore` prints each ignored path it was given, one per line.
pub fn parse_ignored(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|line| line.trim().trim_end_matches('/').replace('\\', "/"))
        .filter(|line| !line.is_empty())
        .collect()
}

/// No console window flashes up when Castle runs a program.
fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Which of the level's paths the project's ignore rules cover (none outside a git repo).
/// git is run directly, never through a shell.
fn ignored_of(root: &Path, rels: &[String]) -> Vec<String> {
    if rels.is_empty() {
        return Vec::new();
    }
    let mut command = Command::new("git");
    command.arg("check-ignore").arg("--").args(rels).current_dir(root).env("GIT_OPTIONAL_LOCKS", "0");
    no_window(&mut command);
    // exit 1 = none ignored, 128 = not a repo: both mean nothing to grey
    command.output().ok().map(|o| parse_ignored(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
}

/// One level of the project's tree, `.git` left out.
pub fn list(folder: &Path, rel: &str) -> Result<Level, String> {
    let dir = inside(folder, rel)?;
    if !dir.is_dir() {
        return Err("That is not a folder.".into());
    }
    let root = folder.canonicalize().map_err(|_| "The project folder is not here.".to_string())?;
    let prefix = rel.replace('\\', "/").trim_matches('/').to_string();
    let mut entries: Vec<Entry> = std::fs::read_dir(&dir)
        .map_err(|_| "That folder can't be read.".to_string())?
        .flatten()
        .filter_map(|item| {
            let name = item.file_name().to_string_lossy().into_owned();
            if name == ".git" {
                return None;
            }
            let meta = item.metadata().ok()?;
            let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let private = meta.is_file() && is_private_name(&name);
            Some(Entry { name, rel, dir: meta.is_dir(), size: meta.is_file().then(|| meta.len()), ignored: false, private })
        })
        .collect();
    sort_entries(&mut entries);
    let more = entries.len().saturating_sub(LEVEL_LIMIT);
    entries.truncate(LEVEL_LIMIT);
    let rels: Vec<String> = entries.iter().map(|e| e.rel.clone()).collect();
    let ignored = ignored_of(&root, &rels);
    for entry in &mut entries {
        entry.ignored = ignored.iter().any(|i| i == &entry.rel);
    }
    Ok(Level { entries, more })
}

/// What the reader shows: the text, the picture, or one line saying why not.
pub fn read(folder: &Path, rel: &str) -> Result<Shown, String> {
    let file = inside(folder, rel)?;
    if !file.is_file() {
        return Err("That is not a file.".into());
    }
    let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // the name asked for and the real one (a link may point at a key) are both checked
    let asked = Path::new(rel).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if is_private_name(&name) || is_private_name(&asked) {
        return Ok(Shown::Note { note: PRIVATE_NOTE.into(), size });
    }
    if crate::files::mime(&file).is_some_and(|m| m.starts_with("image")) {
        return Ok(match crate::files::data_url(&file) {
            Ok(url) => Shown::Picture { url, size },
            Err(why) => Shown::Note { note: why, size },
        });
    }
    if size > READ_LIMIT {
        return Ok(Shown::Note { note: "Too long to show here - open it with Reveal in Explorer.".into(), size });
    }
    let bytes = std::fs::read(&file).map_err(|_| "The file can't be read.".to_string())?;
    if is_binary(&bytes) {
        return Ok(Shown::Note { note: "A binary file - nothing to read here.".into(), size });
    }
    Ok(Shown::Text { text: String::from_utf8_lossy(&bytes).into_owned(), size })
}

/// NUL bytes in the first 8 KB mean a binary file.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|b| *b == 0)
}

/// A resolved Windows path as Explorer reads it: without the `\\?\` prefix canonicalize adds.
pub fn plain_path(path: &Path) -> String {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text
    }
}

/// Windows Explorer opened with the file (or folder) selected - inside the project only.
pub fn reveal(folder: &Path, rel: &str) -> Result<(), String> {
    let target = plain_path(&inside(folder, rel)?);
    let mut command = Command::new("explorer.exe");
    #[cfg(windows)]
    {
        // Explorer wants `/select,"path"` exactly; the usual argument quoting would wrap all of it
        use std::os::windows::process::CommandExt;
        command.raw_arg(format!("/select,\"{target}\""));
    }
    #[cfg(not(windows))]
    command.arg(format!("/select,{target}"));
    no_window(&mut command);
    command.spawn().map(|_| ()).map_err(|_| "Windows could not open it.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("castle-tree-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for d in [".git", "src", "Assets"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        std::fs::write(root.join("b.txt"), "b").unwrap();
        std::fs::write(root.join("A.md"), "a").unwrap();
        std::fs::write(root.join("src/app.js"), "console.log(1)\n").unwrap();
        std::fs::write(root.join("bin.dat"), [0u8, 1, 2]).unwrap();
        std::fs::write(root.join("Assets/dot.png"), b"png").unwrap();
        root
    }

    #[test]
    fn a_level_lists_folders_first_without_git() {
        let root = scratch("level");
        let level = list(&root, "").unwrap();
        let names: Vec<&str> = level.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Assets", "src", "A.md", "b.txt", "bin.dat"]);
        assert_eq!(level.more, 0);
        assert_eq!(list(&root, "src").unwrap().entries[0].rel, "src/app.js");
        assert!(list(&root, "b.txt").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nothing_outside_the_project() {
        let root = scratch("fence");
        std::fs::write(root.parent().unwrap().join(format!("castle-tree-outside-{}.txt", std::process::id())), "x").unwrap();
        assert!(list(&root, "../").is_err());
        assert!(list(&root, "..").is_err());
        assert!(read(&root, "src/../../x").is_err());
        assert!(read(&root, &format!("../castle-tree-outside-{}.txt", std::process::id())).is_err());
        assert!(read(&root, "C:/Windows/win.ini").is_err());
        assert!(read(&root, "/etc/passwd").is_err());
        assert!(read(&root, "..\\..\\x").is_err());
        assert!(!is_inside_rel("..\\x") && !is_inside_rel("/etc") && !is_inside_rel("C:x") && !is_inside_rel("src/../.."));
        assert!(is_inside_rel("src/app.js") && is_inside_rel("") && is_inside_rel("src\\app.js"));
        let _ = std::fs::remove_file(root.parent().unwrap().join(format!("castle-tree-outside-{}.txt", std::process::id())));
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn a_link_out_of_the_project_is_refused() {
        let root = scratch("link");
        let outside = std::env::temp_dir().join(format!("castle-tree-away-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("away.txt"), "away").unwrap();
        // a junction needs no special rights on Windows
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(root.join("door"))
            .arg(&outside)
            .output()
            .is_ok_and(|o| o.status.success());
        if made {
            assert!(list(&root, "door").is_err());
            assert!(read(&root, "door/away.txt").is_err());
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[test]
    fn the_reader_shows_text_pictures_and_says_why_not() {
        let root = scratch("read");
        assert!(matches!(read(&root, "src/app.js").unwrap(), Shown::Text { ref text, .. } if text.contains("console")));
        assert!(matches!(read(&root, "Assets/dot.png").unwrap(), Shown::Picture { ref url, .. } if url.starts_with("data:image/png;base64,")));
        assert!(matches!(read(&root, "bin.dat").unwrap(), Shown::Note { .. }));
        std::fs::write(root.join("big.log"), vec![b'a'; READ_LIMIT as usize + 1]).unwrap();
        assert!(matches!(read(&root, "big.log").unwrap(), Shown::Note { ref note, .. } if note.starts_with("Too long")));
        assert!(read(&root, "src").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn files_named_like_secrets_are_never_opened() {
        for name in [".env", ".ENV.local", ".env.production", "server.pem", "app.key", "id_rsa", "id_rsa.pub", "id_ed25519", "credentials.json", "Credentials", "cert.pfx", "store.p12"] {
            assert!(is_private_name(name), "{name} should be private");
        }
        for name in ["env.js", "app.js", "keyboard.rs", "README.md", "environment.ts", "monkey.txt", ".envrc"] {
            assert!(!is_private_name(name), "{name} should be readable");
        }
        let root = scratch("secret");
        std::fs::write(root.join(".env"), "API_KEY=secret").unwrap();
        std::fs::write(root.join("src/server.key"), "-----BEGIN-----").unwrap();
        for rel in [".env", "src/server.key"] {
            match read(&root, rel).unwrap() {
                Shown::Note { note, .. } => assert_eq!(note, PRIVATE_NOTE),
                other => panic!("{rel} was opened: {other:?}"),
            }
        }
        let level = list(&root, "").unwrap();
        assert!(level.entries.iter().find(|e| e.name == ".env").unwrap().private);
        assert!(!level.entries.iter().find(|e| e.name == "b.txt").unwrap().private);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ignored_paths_are_read_from_git_lines() {
        assert_eq!(parse_ignored("node_modules\nvendor/\n\n.env\r\n"), vec!["node_modules", "vendor", ".env"]);
    }

    #[test]
    fn explorer_gets_a_plain_path() {
        assert_eq!(plain_path(Path::new(r"\\?\D:\Work\shop\a.js")), r"D:\Work\shop\a.js");
        assert_eq!(plain_path(Path::new(r"\\?\UNC\server\share\a.js")), r"\\server\share\a.js");
        assert_eq!(plain_path(Path::new(r"D:\Work\a.js")), r"D:\Work\a.js");
    }
}
