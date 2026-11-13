//! The DEV tab's Source panel and its hands on git: status, diffs, the commit log, one commit's
//! card, committing the ticked files and pushing. Every call runs the real `git` with plain
//! arguments (never a shell, never a prompt, no window) inside the project folder, and waits a
//! fixed time at most.

use std::io::Read;
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// The format every log read uses: fields split by \x1f, records by \x1e.
const LOG_FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%aI%x1f%s%x1f%D%x1e";
/// The most a diff may bring back before it is cut.
const MAX_DIFF: usize = 2 * 1024 * 1024;
/// How many commits one page of the log holds.
const LOG_PAGE: u32 = 50;

const QUICK: Duration = Duration::from_secs(60);
const SLOW: Duration = Duration::from_secs(300);

/// One changed file in the working tree.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// The two status letters with spaces dropped: "M", "A", "D", "R", "??", "MM".
    pub code: String,
    /// The old path when the file was renamed or copied.
    pub from: Option<String>,
}

/// The branch line and the changed files.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// The branch name, or "HEAD" when no branch is checked out.
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

/// One commit as the graph draws it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub parents: Vec<String>,
    pub author: String,
    /// ISO 8601 author date.
    pub date: String,
    pub subject: String,
    /// Branch, remote and tag names pointing here, in git's order.
    pub refs: Vec<String>,
}

/// Lines added and removed in one file; None for a binary file.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

/// Everything the commit card shows.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub commit: Commit,
    /// The whole message, subject included.
    pub message: String,
    pub files: Vec<FileStat>,
    /// Like git's own "3 files changed, 10 insertions(+), 2 deletions(-)".
    pub summary: String,
}

// ---------------------------------------------------------------- runner

/// What one git run left behind.
struct Ran {
    ok: bool,
    stdout: String,
    stderr: String,
}

impl Ran {
    /// Git's own words for a failure.
    fn failure(&self) -> String {
        let err = self.stderr.trim();
        if !err.is_empty() {
            return err.to_string();
        }
        let out = self.stdout.trim();
        if !out.is_empty() {
            return out.to_string();
        }
        "git could not run.".to_string()
    }

    /// Stderr and stdout together, the way a terminal would show them.
    fn words(&self) -> String {
        let parts: Vec<&str> = [self.stderr.trim(), self.stdout.trim()]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect();
        parts.join("\n")
    }
}

/// Reads a pipe to its end on its own thread so a full pipe never stalls git.
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

/// Runs git in the folder and waits at most `timeout`.
fn run(folder: &Path, args: &[&str], timeout: Duration) -> Result<Ran, String> {
    if !folder.is_dir() {
        return Err("That project folder is not there.".to_string());
    }
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "git is not installed on this computer.".to_string()
        } else {
            "git could not run.".to_string()
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
                return Err("git took too long.".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => {
                let _ = child.kill();
                return Err("git could not run.".to_string());
            }
        }
    };
    // A helper git left running could hold a pipe open; never wait on it forever.
    let grace = Duration::from_secs(5);
    let stdout = out.recv_timeout(grace).unwrap_or_default();
    let stderr = err.recv_timeout(grace).unwrap_or_default();
    Ok(Ran {
        ok: status.success(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Runs git and returns its stdout, or git's own trimmed complaint.
fn git(folder: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    let ran = run(folder, args, timeout)?;
    if ran.ok {
        Ok(ran.stdout)
    } else {
        Err(ran.failure())
    }
}

/// True when the repo has at least one commit.
fn has_head(folder: &Path) -> Result<bool, String> {
    Ok(run(folder, &["rev-parse", "--verify", "-q", "HEAD"], QUICK)?.ok)
}

// ---------------------------------------------------------------- parsers

/// Reads `git status --porcelain=v1 -b` (the non -z form).
pub fn parse_status(porcelain: &str) -> Status {
    let mut status = Status {
        branch: String::new(),
        upstream: None,
        ahead: 0,
        behind: 0,
        files: Vec::new(),
    };
    for line in porcelain.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(head) = line.strip_prefix("## ") {
            parse_branch_line(head, &mut status);
            continue;
        }
        let (Some(letters), Some(rest)) = (line.get(..2), line.get(3..)) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let code: String = letters.chars().filter(|c| *c != ' ').collect();
        let (path, from) = if code.contains('R') || code.contains('C') {
            split_rename(rest)
        } else {
            (one_path(rest), None)
        };
        status.files.push(FileChange { path, code, from });
    }
    status
}

/// Fills branch, upstream, ahead and behind from the text after "## ".
fn parse_branch_line(head: &str, status: &mut Status) {
    let (main, bracket) = match head.rfind(" [") {
        Some(i) if head.ends_with(']') => (&head[..i], Some(&head[i + 2..head.len() - 1])),
        _ => (head, None),
    };
    let main = main
        .strip_prefix("No commits yet on ")
        .or_else(|| main.strip_prefix("Initial commit on "))
        .unwrap_or(main);
    if main.starts_with("HEAD (") {
        status.branch = "HEAD".to_string();
    } else if let Some((branch, upstream)) = main.split_once("...") {
        status.branch = branch.to_string();
        status.upstream = Some(upstream.to_string());
    } else {
        status.branch = main.to_string();
    }
    for part in bracket.unwrap_or("").split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            status.ahead = n.trim().parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            status.behind = n.trim().parse().unwrap_or(0);
        }
    }
}

/// Splits `old -> new` (either side may be quoted) into (new, Some(old)).
fn split_rename(rest: &str) -> (String, Option<String>) {
    if rest.starts_with('"') {
        if let Some((old, after)) = take_quoted(rest) {
            return match after.strip_prefix(" -> ") {
                Some(new) => (one_path(new), Some(old)),
                None => (old, None),
            };
        }
    } else if let Some((old, new)) = rest.split_once(" -> ") {
        return (one_path(new), Some(old.to_string()));
    }
    (one_path(rest), None)
}

/// One path, unquoted when git quoted it.
fn one_path(text: &str) -> String {
    if text.starts_with('"') {
        if let Some((path, _)) = take_quoted(text) {
            return path;
        }
    }
    text.to_string()
}

/// Reads a C-style quoted path from the start of `text`; returns it and what follows.
fn take_quoted(text: &str) -> Option<(String, &str)> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((String::from_utf8_lossy(&out).into_owned(), &text[i + 1..])),
            b'\\' if i + 1 < bytes.len() => {
                let next = bytes[i + 1];
                if (b'0'..=b'7').contains(&next) {
                    // Octal byte like \303 (git writes non-ASCII this way).
                    let mut value: u32 = 0;
                    let mut used = 0;
                    while used < 3 && i + 1 + used < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 1 + used]) {
                        value = value * 8 + (bytes[i + 1 + used] - b'0') as u32;
                        used += 1;
                    }
                    out.push(value as u8);
                    i += 1 + used;
                    continue;
                }
                out.push(match next {
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    b'a' => 0x07,
                    b'b' => 0x08,
                    b'f' => 0x0c,
                    b'v' => 0x0b,
                    other => other, // \" and \\ and anything else
                });
                i += 2;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    None
}

/// Reads log text written with [`LOG_FORMAT`].
pub fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\x1e')
        .filter_map(|record| {
            let record = record.trim_start_matches(['\n', '\r']);
            if record.trim().is_empty() {
                return None;
            }
            let fields: Vec<&str> = record.split('\x1f').collect();
            if fields.len() < 6 {
                return None;
            }
            let hash = fields[0].trim().to_string();
            if hash.is_empty() {
                return None;
            }
            let refs = fields[5]
                .trim()
                .split(", ")
                .map(str::trim)
                .filter(|r| !r.is_empty() && *r != "HEAD")
                .map(|r| r.strip_prefix("HEAD -> ").unwrap_or(r).to_string())
                .collect();
            Some(Commit {
                short: hash.chars().take(7).collect(),
                parents: fields[1].split_whitespace().map(String::from).collect(),
                author: fields[2].to_string(),
                date: fields[3].trim().to_string(),
                subject: fields[4].to_string(),
                refs,
                hash,
            })
        })
        .collect()
}

/// Reads `--numstat` lines: `added<TAB>removed<TAB>path`, "-" for binary.
pub fn parse_numstat(text: &str) -> Vec<FileStat> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            let mut parts = line.splitn(3, '\t');
            let added = parts.next()?;
            let removed = parts.next()?;
            let path = parts.next()?;
            if path.is_empty() {
                return None;
            }
            Some(FileStat {
                path: one_path(path),
                added: added.trim().parse().ok(),
                removed: removed.trim().parse().ok(),
            })
        })
        .collect()
}

/// The line git prints under a diffstat, e.g. "2 files changed, 3 insertions(+)".
pub fn summary(files: &[FileStat]) -> String {
    let count = files.len();
    if count == 0 {
        return "0 files changed".to_string();
    }
    let added: u64 = files.iter().map(|f| f.added.unwrap_or(0) as u64).sum();
    let removed: u64 = files.iter().map(|f| f.removed.unwrap_or(0) as u64).sum();
    let plural = |n: u64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut text = plural(count as u64, "file changed", "files changed");
    // Git's rule: a zero side is left out unless both sides are zero.
    if added > 0 || removed == 0 {
        text.push_str(", ");
        text.push_str(&plural(added, "insertion(+)", "insertions(+)"));
    }
    if removed > 0 || added == 0 {
        text.push_str(", ");
        text.push_str(&plural(removed, "deletion(-)", "deletions(-)"));
    }
    text
}

/// Turns a GitHub remote URL into `https://github.com/<owner>/<repo>/commit/`.
pub fn github_commit_base(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let path = if let Some(rest) = remote.strip_prefix("git@github.com:") {
        rest
    } else {
        let rest = ["https://", "http://", "ssh://", "git://"]
            .iter()
            .find_map(|scheme| {
                remote
                    .get(..scheme.len())
                    .filter(|head| head.eq_ignore_ascii_case(scheme))
                    .map(|_| &remote[scheme.len()..])
            })?;
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit('@').next()?;
        let host = host.split(':').next()?;
        if !host.eq_ignore_ascii_case("github.com") && !host.eq_ignore_ascii_case("www.github.com") {
            return None;
        }
        path
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}/commit/"))
}

/// True for 4 to 40 hex characters, the shapes a commit hash takes.
pub fn is_hash(text: &str) -> bool {
    (4..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A path the panel may hand to git: relative, inside the repo, never an option.
fn check_path(path: &str) -> Result<(), String> {
    let bad = || Err(format!("That path cannot be used: {path}"));
    if path.trim().is_empty() || path.starts_with('-') || path.contains('\0') {
        return bad();
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains(':') {
        return bad();
    }
    if path.split(['/', '\\']).any(|part| part == "..") {
        return bad();
    }
    if Path::new(path)
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return bad();
    }
    Ok(())
}

/// Cuts text at 2 MB on a character boundary.
fn cap(mut text: String) -> String {
    if text.len() <= MAX_DIFF {
        return text;
    }
    let mut end = MAX_DIFF;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("... (cut at 2 MB)");
    text
}

/// An untracked file shown as a diff where every line is new.
fn added_file_diff(folder: &Path, path: &str) -> Result<String, String> {
    let file = std::fs::File::open(folder.join(path)).map_err(|_| format!("{path} could not be read."))?;
    let mut bytes = Vec::new();
    file.take(MAX_DIFF as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("{path} could not be read."))?;
    let cut = bytes.len() > MAX_DIFF;
    bytes.truncate(MAX_DIFF);
    let mut text = format!("diff --git a/{path} b/{path}\nnew file\n--- /dev/null\n+++ b/{path}\n");
    if bytes.contains(&0) {
        text.push_str("Binary file, not shown.\n");
        return Ok(text);
    }
    let body = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = body.lines().collect();
    text.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
    for line in lines {
        text.push('+');
        text.push_str(line);
        text.push('\n');
    }
    if cut {
        text.push_str("... (cut at 2 MB)");
    }
    Ok(cap(text))
}

// ---------------------------------------------------------------- commands

/// The branch and the changed files.
pub fn status(folder: &Path) -> Result<Status, String> {
    git(folder, &["status", "--porcelain=v1", "-b"], QUICK).map(|text| parse_status(&text))
}

/// The diff of one file: in the working tree, or inside one commit.
pub fn diff(folder: &Path, path: &str, commit: Option<&str>) -> Result<String, String> {
    check_path(path)?;
    if let Some(hash) = commit {
        if !is_hash(hash) {
            return Err("That is not a commit.".to_string());
        }
        let text = git(folder, &["show", "--no-color", "--format=", hash, "--", path], QUICK)?;
        return Ok(cap(text));
    }
    // Untracked? Then git has no diff for it; show its lines as all new.
    let untracked = git(folder, &["ls-files", "--others", "--exclude-standard", "--", path], QUICK)?;
    let untracked: Vec<String> = untracked.lines().map(one_path).filter(|l| !l.is_empty()).collect();
    if !untracked.is_empty() {
        let target = folder.join(path);
        if target.is_dir() {
            let mut text = format!("New folder {path} with {} file(s):\n", untracked.len());
            for file in untracked {
                text.push('+');
                text.push_str(&file);
                text.push('\n');
            }
            return Ok(cap(text));
        }
        return added_file_diff(folder, path);
    }
    if has_head(folder)? {
        let text = git(folder, &["diff", "--no-color", "HEAD", "--", path], QUICK)?;
        return Ok(cap(text));
    }
    // No commits yet: what is staged, then what is not.
    let staged = git(folder, &["diff", "--no-color", "--cached", "--", path], QUICK)?;
    let loose = git(folder, &["diff", "--no-color", "--", path], QUICK)?;
    Ok(cap(staged + &loose))
}

/// One page of 50 commits, newest first, starting after `skip`.
pub fn log(folder: &Path, skip: u32) -> Result<Vec<Commit>, String> {
    if !has_head(folder)? {
        return Ok(Vec::new());
    }
    let skip = format!("--skip={skip}");
    let count = LOG_PAGE.to_string();
    let text = git(folder, &["log", "--no-color", &skip, "-n", &count, LOG_FORMAT], QUICK)?;
    Ok(parse_log(&text))
}

/// Everything the commit card needs for one commit.
pub fn show(folder: &Path, hash: &str) -> Result<CommitDetail, String> {
    if !is_hash(hash) {
        return Err("That is not a commit.".to_string());
    }
    let commit = parse_log(&git(folder, &["log", "--no-color", "-n", "1", LOG_FORMAT, hash], QUICK)?)
        .into_iter()
        .next()
        .ok_or_else(|| "That commit was not found.".to_string())?;
    let message = git(folder, &["show", "--no-color", "-s", "--format=%B", hash], QUICK)?
        .trim_end()
        .to_string();
    let files = parse_numstat(&git(folder, &["show", "--no-color", "--numstat", "--format=", hash], QUICK)?);
    let summary = summary(&files);
    Ok(CommitDetail { commit, message, files, summary })
}

/// Stages the ticked files and commits them with the message; returns git's words.
pub fn commit(folder: &Path, paths: &[String], message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Write a message first.".to_string());
    }
    if paths.is_empty() {
        return Err("Tick at least one file.".to_string());
    }
    for path in paths {
        check_path(path)?;
    }
    let mut add: Vec<&str> = vec!["add", "--"];
    add.extend(paths.iter().map(String::as_str));
    git(folder, &add, QUICK * 2)?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file = std::env::temp_dir().join(format!("castle-commit-{}-{stamp}.txt", std::process::id()));
    std::fs::write(&file, message).map_err(|_| "The message could not be written down.".to_string())?;
    let file_text = file.to_string_lossy().into_owned();
    // Only the ticked files go in (the member's ticks decide, not whatever else was staged); a
    // rename's old path comes ticked with it from the screen.
    let mut commit: Vec<&str> = vec!["commit", "-F", &file_text, "--"];
    commit.extend(paths.iter().map(String::as_str));
    let ran = run(folder, &commit, SLOW);
    let _ = std::fs::remove_file(&file);
    let ran = ran?;
    if ran.ok {
        Ok(ran.words())
    } else {
        Err(ran.failure())
    }
}

/// Pushes the current branch (setting its upstream the first time); never forces.
pub fn push(folder: &Path) -> Result<String, String> {
    let now = status(folder)?;
    let real_branch = !now.branch.is_empty() && now.branch != "HEAD" && !now.branch.starts_with('-');
    let ran = if now.upstream.is_none() && real_branch {
        run(folder, &["push", "-u", "origin", &now.branch], SLOW)?
    } else {
        run(folder, &["push"], SLOW)?
    };
    if ran.ok {
        let words = ran.words();
        Ok(if words.is_empty() { "Pushed.".to_string() } else { words })
    } else {
        Err(ran.failure())
    }
}

/// The "Open on GitHub" base for this repo, when origin lives on GitHub.
pub fn remote_commit_base(folder: &Path) -> Option<String> {
    let url = git(folder, &["remote", "get-url", "origin"], Duration::from_secs(15)).ok()?;
    github_commit_base(url.trim())
}

/// A link the Source panel may open in the browser: a GitHub page, the GitHub CLI's own page
/// (https://cli.github.com, where the New project screen sends a member without it), or a server
/// on this PC (http://localhost:<port> or http://127.0.0.1:<port>). Nothing else.
pub fn is_openable_link(url: &str) -> bool {
    if url.len() > 500 || url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '\'') {
        return false;
    }
    if let Some(rest) = url.strip_prefix("http://localhost:").or_else(|| url.strip_prefix("http://127.0.0.1:")) {
        let port: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let after = &rest[port.len()..];
        return !port.is_empty() && port.len() <= 5 && (after.is_empty() || after.starts_with('/'));
    }
    if let Some(rest) = url.strip_prefix("https://cli.github.com") {
        return rest.is_empty() || rest.starts_with('/');
    }
    url.starts_with("https://github.com/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: &str, code: &str, from: Option<&str>) -> FileChange {
        FileChange { path: path.into(), code: code.into(), from: from.map(String::from) }
    }

    fn stat(path: &str, added: Option<u32>, removed: Option<u32>) -> FileStat {
        FileStat { path: path.into(), added, removed }
    }

    #[test]
    fn branch_with_upstream_ahead_and_behind() {
        let s = parse_status("## main...origin/main [ahead 2, behind 1]\n");
        assert_eq!(s.branch, "main");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert!(s.files.is_empty());
    }

    #[test]
    fn branch_ahead_only_and_behind_only() {
        let s = parse_status("## dev...origin/dev [ahead 5]");
        assert_eq!((s.ahead, s.behind), (5, 0));
        let s = parse_status("## dev...origin/dev [behind 3]");
        assert_eq!((s.ahead, s.behind), (0, 3));
    }

    #[test]
    fn branch_in_step_and_gone() {
        let s = parse_status("## main...origin/main\n");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (0, 0));
        let s = parse_status("## main...origin/main [gone]\n");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (0, 0));
    }

    #[test]
    fn branch_without_upstream() {
        let s = parse_status("## feature/x\n M a.rs\n");
        assert_eq!(s.branch, "feature/x");
        assert_eq!(s.upstream, None);
        assert_eq!(s.files, vec![change("a.rs", "M", None)]);
    }

    #[test]
    fn detached_and_empty_repo() {
        assert_eq!(parse_status("## HEAD (no branch)\n").branch, "HEAD");
        let s = parse_status("## No commits yet on main\n?? new.txt\n");
        assert_eq!(s.branch, "main");
        assert_eq!(s.upstream, None);
        assert_eq!(s.files, vec![change("new.txt", "??", None)]);
        assert_eq!(parse_status("## Initial commit on trunk").branch, "trunk");
    }

    #[test]
    fn file_codes() {
        let s = parse_status("## main\n M src/a.rs\nM  b.rs\nMM c.rs\nA  d.rs\n D e.rs\n?? f/\nAM g.rs\r\n");
        let codes: Vec<&str> = s.files.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(codes, ["M", "M", "MM", "A", "D", "??", "AM"]);
        assert_eq!(s.files[0].path, "src/a.rs");
        assert_eq!(s.files[5].path, "f/");
        assert_eq!(s.files[6].path, "g.rs");
    }

    #[test]
    fn renames() {
        let s = parse_status("## main\nR  old.rs -> new.rs\nRM src/a.rs -> src/b.rs\n");
        assert_eq!(s.files[0], change("new.rs", "R", Some("old.rs")));
        assert_eq!(s.files[1], change("src/b.rs", "RM", Some("src/a.rs")));
    }

    #[test]
    fn quoted_paths() {
        let s = parse_status(concat!(
            "## main\n",
            "?? \"a b.txt\"\n",
            " M \"say \\\"hi\\\".md\"\n",
            " M \"back\\\\slash.txt\"\n",
            "?? \"caf\\303\\251.txt\"\n",
            "R  \"old name.txt\" -> \"new name.txt\"\n",
            "R  plain.txt -> \"with space.txt\"\n",
        ));
        assert_eq!(s.files[0], change("a b.txt", "??", None));
        assert_eq!(s.files[1].path, "say \"hi\".md");
        assert_eq!(s.files[2].path, "back\\slash.txt");
        assert_eq!(s.files[3].path, "café.txt");
        assert_eq!(s.files[4], change("new name.txt", "R", Some("old name.txt")));
        assert_eq!(s.files[5], change("with space.txt", "R", Some("plain.txt")));
    }

    #[test]
    fn log_records_and_refs() {
        let text = concat!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1fbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccccccccccccccccccccc",
            "\x1fAda\x1f2026-11-13T10:00:00+08:00\x1fMerge it\x1fHEAD -> main, origin/main, tag: v1.0\x1e\n",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\x1f\x1fAda\x1f2026-11-12T09:00:00+08:00\x1fFirst, with comma\x1f\x1e\n",
            "dddddddddddddddddddddddddddddddddddddddd\x1f\x1fA\x1f2026-01-01T00:00:00Z\x1fdetached\x1fHEAD, origin/dev\x1e\n",
        );
        let log = parse_log(text);
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].short, "aaaaaaa");
        assert_eq!(log[0].parents.len(), 2);
        assert_eq!(log[0].author, "Ada");
        assert_eq!(log[0].date, "2026-11-13T10:00:00+08:00");
        assert_eq!(log[0].subject, "Merge it");
        assert_eq!(log[0].refs, ["main", "origin/main", "tag: v1.0"]);
        assert!(log[1].parents.is_empty());
        assert!(log[1].refs.is_empty());
        assert_eq!(log[1].subject, "First, with comma");
        assert_eq!(log[2].refs, ["origin/dev"]);
        assert!(parse_log("").is_empty());
        assert!(parse_log("\n").is_empty());
    }

    #[test]
    fn numstat_lines() {
        let files = parse_numstat("3\t1\tsrc/a.rs\n-\t-\tbin.png\n0\t0\tsrc/{old => new}/x.rs\n\n");
        assert_eq!(
            files,
            vec![
                stat("src/a.rs", Some(3), Some(1)),
                stat("bin.png", None, None),
                stat("src/{old => new}/x.rs", Some(0), Some(0)),
            ]
        );
    }

    #[test]
    fn summary_like_git() {
        let five = vec![stat("a", Some(1), Some(1)); 5];
        assert_eq!(summary(&five), "5 files changed, 5 insertions(+), 5 deletions(-)");
        assert_eq!(summary(&[stat("a", Some(1), Some(1))]), "1 file changed, 1 insertion(+), 1 deletion(-)");
        assert_eq!(summary(&[stat("a", Some(2), Some(0))]), "1 file changed, 2 insertions(+)");
        assert_eq!(summary(&[stat("a", Some(0), Some(3))]), "1 file changed, 3 deletions(-)");
        assert_eq!(
            summary(&[stat("bin.png", None, None)]),
            "1 file changed, 0 insertions(+), 0 deletions(-)"
        );
        assert_eq!(summary(&[]), "0 files changed");
    }

    #[test]
    fn github_links() {
        let want = Some("https://github.com/o/r/commit/".to_string());
        assert_eq!(github_commit_base("https://github.com/o/r.git"), want);
        assert_eq!(github_commit_base("https://github.com/o/r"), want);
        assert_eq!(github_commit_base("https://github.com/o/r/"), want);
        assert_eq!(github_commit_base("git@github.com:o/r.git"), want);
        assert_eq!(github_commit_base("ssh://git@github.com/o/r.git"), want);
        assert_eq!(github_commit_base("https://user@github.com/o/r.git\n"), want);
        assert_eq!(github_commit_base("ssh://git@github.com:22/o/r.git"), want);
        assert_eq!(github_commit_base("https://gitlab.com/o/r.git"), None);
        assert_eq!(github_commit_base("git@bitbucket.org:o/r.git"), None);
        assert_eq!(github_commit_base("https://github.com.evil.io/o/r"), None);
        assert_eq!(github_commit_base("https://github.com/o"), None);
        assert_eq!(github_commit_base("D:/repos/r"), None);
        assert_eq!(github_commit_base(""), None);
    }

    #[test]
    fn hashes() {
        assert!(is_hash("abcd"));
        assert!(is_hash("e9a5add"));
        assert!(is_hash(&"a".repeat(40)));
        assert!(is_hash("ABCDEF12"));
        assert!(!is_hash("abc"));
        assert!(!is_hash(&"a".repeat(41)));
        assert!(!is_hash("xyz1234"));
        assert!(!is_hash("--all"));
        assert!(!is_hash(""));
    }

    #[test]
    fn paths_checked() {
        assert!(check_path("src/a.rs").is_ok());
        assert!(check_path("a b.txt").is_ok());
        assert!(check_path("dir/").is_ok());
        for bad in ["", "-f", "--force", "../x", "a/../../b", "a\\..\\b", "/etc/x", "C:\\x", "\\\\server\\x"] {
            assert!(check_path(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn commit_refuses_before_git() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(commit(here, &["a.rs".into()], "  "), Err("Write a message first.".into()));
        assert_eq!(commit(here, &[], "hi"), Err("Tick at least one file.".into()));
        assert!(commit(here, &["../x".into()], "hi").is_err());
    }

    #[test]
    fn only_github_and_local_links_open() {
        assert!(is_openable_link("http://localhost:5173"));
        assert!(is_openable_link("http://localhost:8080/api/health"));
        assert!(is_openable_link("http://127.0.0.1:3000"));
        assert!(is_openable_link("https://github.com/o/r/commit/e9a5add"));
        assert!(!is_openable_link("http://localhost:"));
        assert!(!is_openable_link("http://localhost:80evil"));
        assert!(!is_openable_link("http://localhost:123456"));
        assert!(!is_openable_link("https://evil.com/github.com/"));
        assert!(!is_openable_link("https://github.com.evil.io/o/r"));
        assert!(!is_openable_link("http://github.com/o/r"));
        assert!(!is_openable_link("file:///C:/Windows"));
        assert!(!is_openable_link("https://github.com/a b"));
        assert!(!is_openable_link("https://github.com/a\"&calc"));
        assert!(!is_openable_link("C:\\Windows\\notepad.exe"));
        assert!(!is_openable_link(""));
    }

    #[test]
    fn the_github_cli_page_opens_and_nothing_that_only_looks_like_it() {
        assert!(is_openable_link("https://cli.github.com"));
        assert!(is_openable_link("https://cli.github.com/"));
        assert!(is_openable_link("https://cli.github.com/manual/gh_auth_login"));
        assert!(!is_openable_link("https://cli.github.com.evil.io/"));
        assert!(!is_openable_link("https://cli.github.comevil.io"));
        assert!(!is_openable_link("https://cli.github.com@evil.io/"));
        assert!(!is_openable_link("http://cli.github.com"));
        assert!(!is_openable_link("https://cli.github.com/a b"));
    }

    #[test]
    fn cap_cuts_long_text() {
        let text = cap("é".repeat(MAX_DIFF));
        assert!(text.ends_with("... (cut at 2 MB)"));
        assert!(text.len() <= MAX_DIFF + 32);
        assert_eq!(cap("short".into()), "short");
    }

    /// A throwaway repository with a bare "origin" beside it, in the temp folder.
    struct Scratch {
        root: std::path::PathBuf,
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn a_real_repo_commits_only_the_ticked_files_and_pushes_once() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let scratch = Scratch { root: std::env::temp_dir().join(format!("castle-git-test-{}-{stamp}", std::process::id())) };
        let work = scratch.root.join("work");
        let origin = scratch.root.join("origin.git");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(&origin).unwrap();
        // No git on this computer: nothing to try.
        if run(&work, &["--version"], QUICK).is_err() {
            return;
        }
        git(&origin, &["init", "--bare", "-q"], QUICK).unwrap();
        git(&work, &["init", "-q"], QUICK).unwrap();
        git(&work, &["symbolic-ref", "HEAD", "refs/heads/main"], QUICK).unwrap();
        for (key, value) in [("user.name", "Test"), ("user.email", "test@example.com"), ("commit.gpgsign", "false"), ("core.autocrlf", "false")] {
            git(&work, &["config", key, value], QUICK).unwrap();
        }
        // Hooks from this PC's own git setup never run in the test repository.
        let no_hooks = scratch.root.join("hooks");
        std::fs::create_dir_all(&no_hooks).unwrap();
        git(&work, &["config", "core.hooksPath", &no_hooks.to_string_lossy()], QUICK).unwrap();

        assert!(log(&work, 0).unwrap().is_empty(), "no commits yet");
        std::fs::write(work.join("a.txt"), "one\n").unwrap();
        std::fs::write(work.join("b.txt"), "hello b\n").unwrap();
        assert!(diff(&work, "b.txt", None).unwrap().contains("+hello b"));

        commit(&work, &["a.txt".into()], "First\n\nThe body line.").unwrap();
        let now = status(&work).unwrap();
        assert_eq!(now.branch, "main");
        assert_eq!(now.upstream, None);
        assert_eq!(now.files, vec![change("b.txt", "??", None)], "b.txt was not ticked, so it stays out");

        let page = log(&work, 0).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].subject, "First");
        let card = show(&work, &page[0].short).unwrap();
        assert_eq!(card.message, "First\n\nThe body line.");
        assert_eq!(card.files, vec![stat("a.txt", Some(1), Some(0))]);
        assert_eq!(card.summary, "1 file changed, 1 insertion(+)");
        assert!(diff(&work, "a.txt", Some(&page[0].hash)).unwrap().contains("+one"));

        std::fs::write(work.join("a.txt"), "two\n").unwrap();
        let changed = diff(&work, "a.txt", None).unwrap();
        assert!(changed.contains("-one") && changed.contains("+two"));

        assert_eq!(remote_commit_base(&work), None, "no origin yet");
        let origin_text = origin.to_string_lossy().into_owned();
        git(&work, &["remote", "add", "origin", &origin_text], QUICK).unwrap();
        push(&work).unwrap();
        let now = status(&work).unwrap();
        assert_eq!(now.upstream.as_deref(), Some("origin/main"), "the first push sets the upstream");
        assert_eq!(now.ahead, 0);
    }

    /// A read-only look at Castle's own repository (run by hand with --ignored).
    #[test]
    #[ignore]
    fn live_repo() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let now = status(repo).expect("status");
        println!("branch {} upstream {:?} ahead {} behind {}", now.branch, now.upstream, now.ahead, now.behind);
        for file in &now.files {
            println!("  {:>2} {} {:?}", file.code, file.path, file.from);
        }
        let page = log(repo, 0).expect("log");
        for c in page.iter().take(3) {
            println!("{} {} {:?} {} | {}", c.short, c.date, c.refs, c.parents.len(), c.subject);
        }
        if let Some(top) = page.first() {
            let card = show(repo, &top.short).expect("show");
            println!("card {}: {} | {} file(s)", card.commit.short, card.summary, card.files.len());
            if let Some(file) = card.files.first() {
                let text = diff(repo, &file.path, Some(&top.hash)).expect("commit diff");
                println!("commit diff of {}: {} bytes", file.path, text.len());
            }
        }
        if let Some(file) = now.files.first() {
            let text = diff(repo, &file.path, None).expect("diff");
            println!("working diff of {}: {} bytes", file.path, text.len());
        }
        println!("github base {:?}", remote_commit_base(repo));
    }
}
