//! First run: finding or creating the member's MemoryCore folder, checking their CLI, filling in
//! MemoryCore's name placeholders, and writing Castle's block into CLAUDE.md and AGENTS.md so
//! Claude Code and Codex both load the memory by themselves.

use crate::env;
use crate::settings::Cli;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const MEMORYCORE_REPO: &str = "https://github.com/Kiyoraka/Poject-AI-Memory-Architecture";
const BLOCK_TEMPLATE: &str = include_str!("../resources/castle-block.md");
const BLOCK_BEGIN: &str = "<!-- castle:begin";
const BLOCK_END: &str = "<!-- castle:end -->";
const PLACEHOLDERS: [&str; 3] = ["[AI_NAME]", "[YOUR_NAME]", "[RELATIONSHIP_STYLE]"];

/// What a chosen folder is, for the setup screen.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FolderState {
    pub exists: bool,
    /// A MemoryCore folder: master-memory.md (or main/main-memory.md after Memory Consolidation).
    pub is_memorycore: bool,
    /// MemoryCore's placeholders are still there: the names have not been chosen yet.
    pub needs_names: bool,
    /// The file the companion reads first.
    pub entry_file: Option<String>,
    /// The names already written, when setup ran before (read back from Castle's block).
    pub companion_name: Option<String>,
}

fn entry_file(folder: &Path) -> Option<&'static str> {
    if folder.join("main").join("main-memory.md").is_file() {
        Some("main/main-memory.md")
    } else if folder.join("master-memory.md").is_file() {
        Some("master-memory.md")
    } else {
        None
    }
}

/// The memory files that may hold MemoryCore's placeholders: master-memory.md and main/*.md.
fn memory_files(folder: &Path) -> Vec<PathBuf> {
    let mut files = vec![folder.join("master-memory.md")];
    if let Ok(entries) = std::fs::read_dir(folder.join("main")) {
        files.extend(entries.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md")));
    }
    files.retain(|p| p.is_file());
    files
}

pub fn inspect(folder: &Path) -> FolderState {
    let entry = entry_file(folder);
    let needs_names = memory_files(folder)
        .iter()
        .any(|p| std::fs::read_to_string(p).is_ok_and(|t| t.contains("[AI_NAME]")));
    let companion_name = std::fs::read_to_string(folder.join("CLAUDE.md")).ok().and_then(|t| {
        let start = t.find("You are **")? + "You are **".len();
        let end = t[start..].find("**")?;
        Some(t[start..start + end].to_string())
    });
    FolderState {
        exists: folder.is_dir(),
        is_memorycore: entry.is_some(),
        needs_names,
        entry_file: entry.map(str::to_string),
        companion_name,
    }
}

/// Replaces MemoryCore's placeholders with the member's choices in every memory file.
/// Returns how many placeholders were filled.
pub fn fill_names(folder: &Path, companion: &str, member: &str, style: &str) -> Result<usize, String> {
    let values = [companion.trim(), member.trim(), style.trim()];
    if values[0].is_empty() || values[1].is_empty() {
        return Err("Both names are needed.".into());
    }
    let mut filled = 0;
    for file in memory_files(folder) {
        let text = std::fs::read_to_string(&file).map_err(|_| format!("{} could not be read.", file.display()))?;
        let mut changed = text.clone();
        for (placeholder, value) in PLACEHOLDERS.iter().zip(values) {
            if value.is_empty() {
                continue;
            }
            filled += changed.matches(placeholder).count();
            changed = changed.replace(placeholder, value);
        }
        if changed != text {
            std::fs::write(&file, changed).map_err(|_| format!("{} could not be written.", file.display()))?;
        }
    }
    Ok(filled)
}

/// Castle's block with the member's names in it.
pub fn castle_block(companion: &str, member: &str, entry: &str) -> String {
    BLOCK_TEMPLATE
        .replace("{{AI_NAME}}", companion.trim())
        .replace("{{USER_NAME}}", member.trim())
        .replace("{{ENTRY_FILE}}", entry)
}

/// `text` with Castle's block put in place: the old block replaced, or the block added at the
/// end. Everything the member wrote outside the markers is kept as it was.
pub fn with_block(text: &str, block: &str) -> String {
    let block = block.trim_end();
    if let (Some(start), Some(end)) = (text.find(BLOCK_BEGIN), text.find(BLOCK_END)) {
        if end > start {
            let after = end + BLOCK_END.len();
            return format!("{}{}{}", &text[..start], block, &text[after..]);
        }
    }
    if text.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n\n{block}\n", text.trim_end())
    }
}

/// Writes Castle's block into CLAUDE.md (Claude Code) and AGENTS.md (Codex).
pub fn write_blocks(folder: &Path, companion: &str, member: &str) -> Result<(), String> {
    let entry = entry_file(folder).ok_or("This folder has no master-memory.md, so it is not a MemoryCore folder.")?;
    let block = castle_block(companion, member, entry);
    for name in ["CLAUDE.md", "AGENTS.md"] {
        let path = folder.join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        std::fs::write(&path, with_block(&text, &block)).map_err(|_| format!("{name} could not be written."))?;
    }
    Ok(())
}

/// Runs a program with no console window and a time limit; its first output line.
fn first_line(program: &Path, args: &[&str], limit: Duration) -> Option<String> {
    let is_script = program.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    let mut command = if is_script {
        let mut c = Command::new("cmd.exe");
        c.arg("/c").arg(program);
        c
    } else {
        Command::new(program)
    };
    command.args(args).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let mut child = command.spawn().ok()?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    String::from_utf8_lossy(&output.stdout).lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

/// Whether the member's CLI is installed and signed in, with the words to fix it when not.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliCheck {
    pub cli: Cli,
    pub label: &'static str,
    pub installed: bool,
    pub version: Option<String>,
    /// None when Castle cannot tell.
    pub signed_in: Option<bool>,
    pub install_command: &'static str,
    pub sign_in_hint: &'static str,
}

/// The file each CLI keeps once the member has signed in.
fn credentials_file(cli: Cli) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("USERPROFILE")?);
    Some(match cli {
        Cli::Claude => home.join(".claude").join(".credentials.json"),
        Cli::Codex => std::env::var_os("CODEX_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home.join(".codex")).join("auth.json"),
    })
}

pub fn check_cli(cli: Cli) -> CliCheck {
    let program = env::find_on_path(cli.command());
    let version = program.as_deref().and_then(|p| first_line(p, &["--version"], Duration::from_secs(15)));
    let (install_command, sign_in_hint) = match cli {
        Cli::Claude => ("npm install -g @anthropic-ai/claude-code", "Open the Log, type /login and follow the browser."),
        Cli::Codex => ("npm install -g @openai/codex", "Run `codex login` once (the Log can do it) and sign in with your ChatGPT account."),
    };
    CliCheck {
        cli,
        label: cli.label(),
        installed: program.is_some(),
        version,
        signed_in: credentials_file(cli).map(|f| f.is_file()),
        install_command,
        sign_in_hint,
    }
}

/// Creates a fresh MemoryCore folder at `target` from the public MemoryCore repository.
/// Uses git when it is installed; otherwise downloads the repository's zip and unpacks it.
pub fn create(target: &Path) -> Result<FolderState, String> {
    if target.exists() && std::fs::read_dir(target).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err("That folder already has files in it. Choose an empty or new folder.".into());
    }
    if let Some(git) = env::find_on_path("git") {
        let status = Command::new(git)
            .args(["clone", "--depth", "1", MEMORYCORE_REPO])
            .arg(target)
            .status()
            .map_err(|_| "git could not be started.".to_string())?;
        if !status.success() {
            return Err("MemoryCore could not be downloaded with git. Check the internet connection and try again.".into());
        }
        // A fresh memory belongs to the member: it starts its own history, not MemoryCore's.
        let _ = std::fs::remove_dir_all(target.join(".git"));
    } else {
        download_zip(target)?;
    }
    let state = inspect(target);
    if !state.is_memorycore {
        return Err("The download finished but no master-memory.md was found in it.".into());
    }
    Ok(state)
}

fn download_zip(target: &Path) -> Result<(), String> {
    let url = format!("{MEMORYCORE_REPO}/archive/refs/heads/main.zip");
    let bytes = reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCore-Castle/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(120))
        .build()
        .and_then(|c| c.get(&url).send())
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|_| "MemoryCore could not be downloaded. Check the internet connection and try again.".to_string())?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "The download was not a readable zip.".to_string())?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|_| "The zip could not be read.".to_string())?;
        let Some(inside) = file.enclosed_name() else { continue };
        // the zip holds one top folder (<repo>-main/); its contents become the memory folder
        let relative: PathBuf = inside.components().skip(1).collect();
        if relative.as_os_str().is_empty() {
            continue;
        }
        let out = target.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&out).map_err(|_| "A folder could not be created.".to_string())?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|_| "A folder could not be created.".to_string())?;
            }
            let mut writer = std::fs::File::create(&out).map_err(|_| "A file could not be written.".to_string())?;
            std::io::copy(&mut file, &mut writer).map_err(|_| "A file could not be written.".to_string())?;
        }
    }
    Ok(())
}

// ---------- Commands for the setup screens ----------

#[tauri::command]
pub fn setup_inspect(folder: String) -> FolderState {
    inspect(Path::new(&folder))
}

/// Downloads a fresh MemoryCore into `folder` (new or empty).
#[tauri::command]
pub async fn setup_create(folder: String) -> Result<FolderState, String> {
    tauri::async_runtime::spawn_blocking(move || create(Path::new(&folder)))
        .await
        .map_err(|_| "The download stopped unexpectedly.".to_string())?
}

#[tauri::command]
pub async fn setup_check_cli(cli: Cli) -> Result<CliCheck, String> {
    tauri::async_runtime::spawn_blocking(move || check_cli(cli))
        .await
        .map_err(|_| "The check stopped unexpectedly.".to_string())
}

/// Writes the names into the memory (when its placeholders are still there) and Castle's block
/// into CLAUDE.md and AGENTS.md.
#[tauri::command]
pub fn setup_apply(folder: String, companion: String, member: String, style: String) -> Result<FolderState, String> {
    let path = Path::new(&folder);
    if inspect(path).needs_names {
        fill_names(path, &companion, &member, &style)?;
    }
    write_blocks(path, &companion, &member)?;
    Ok(inspect(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("castle-setup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("main")).unwrap();
        std::fs::write(root.join("master-memory.md"), "**I am [AI_NAME]** - type \"[AI_NAME]\"\n").unwrap();
        std::fs::write(root.join("main").join("identity-core.md"), "# [AI_NAME]\nbe [RELATIONSHIP_STYLE] for [YOUR_NAME]\n").unwrap();
        std::fs::write(root.join("main").join("notes.txt"), "[AI_NAME] stays in non-md files").unwrap();
        root
    }

    #[test]
    fn a_fresh_memorycore_needs_names_and_filling_them_clears_the_need() {
        let root = memory("fill");
        let state = inspect(&root);
        assert!(state.exists && state.is_memorycore && state.needs_names);
        assert_eq!(state.entry_file.as_deref(), Some("master-memory.md"));

        assert_eq!(fill_names(&root, "Lyra", "Rin", "a friendly partner").unwrap(), 5);
        assert!(!inspect(&root).needs_names);
        let identity = std::fs::read_to_string(root.join("main").join("identity-core.md")).unwrap();
        assert_eq!(identity, "# Lyra\nbe a friendly partner for Rin\n");
        assert!(std::fs::read_to_string(root.join("main").join("notes.txt")).unwrap().contains("[AI_NAME]"));
        assert!(fill_names(&root, " ", "Rin", "").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_block_is_added_once_and_replaced_without_touching_the_members_words() {
        let block_v1 = castle_block("Lyra", "Rin", "master-memory.md");
        assert!(block_v1.contains("You are **Lyra**") && block_v1.contains("read `master-memory.md`"));
        let mine = "# My notes\n\nKeep answers short.\n";
        let once = with_block(mine, &block_v1);
        assert!(once.starts_with(mine.trim_end()));
        assert_eq!(once.matches(BLOCK_BEGIN).count(), 1);

        let with_tail = format!("{once}\nMore of my own words.\n");
        let block_v2 = castle_block("Nova", "Rin", "main/main-memory.md");
        let twice = with_block(&with_tail, &block_v2);
        assert_eq!(twice.matches(BLOCK_BEGIN).count(), 1);
        assert!(twice.contains("You are **Nova**") && !twice.contains("You are **Lyra**"));
        assert!(twice.contains("Keep answers short.") && twice.contains("More of my own words."));
        assert_eq!(with_block("", &block_v1), format!("{}\n", block_v1.trim_end()));
    }

    #[test]
    fn blocks_go_to_both_cli_files_and_the_name_reads_back() {
        let root = memory("blocks");
        write_blocks(&root, "Lyra", "Rin").unwrap();
        for name in ["CLAUDE.md", "AGENTS.md"] {
            assert!(std::fs::read_to_string(root.join(name)).unwrap().contains("You are **Lyra**"));
        }
        assert_eq!(inspect(&root).companion_name.as_deref(), Some("Lyra"));
        // after Memory Consolidation the entry file moves
        std::fs::write(root.join("main").join("main-memory.md"), "x").unwrap();
        assert_eq!(inspect(&root).entry_file.as_deref(), Some("main/main-memory.md"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_folder_with_files_is_never_cloned_into() {
        let root = memory("busy");
        assert!(create(&root).unwrap_err().contains("already has files"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_missing_cli_says_how_to_install_it() {
        let check = check_cli(Cli::Codex);
        assert!(check.install_command.contains("@openai/codex"));
        assert_eq!(check.label, "Codex");
    }
}
