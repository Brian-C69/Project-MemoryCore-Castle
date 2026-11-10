//! What the companion's session is started with: which program, and which environment.
//! Keys reach the CLI only here, as environment variables for that one session.

use crate::settings::Cli;
use std::path::{Path, PathBuf};

/// Finds a command on PATH the way Windows would: each PATH folder in order, and in each folder
/// the PATHEXT endings in order (.EXE before .CMD by default).
pub fn find_on_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    find_in(command, std::env::split_paths(&path), &exts)
}

pub fn find_in(command: &str, folders: impl IntoIterator<Item = PathBuf>, pathext: &str) -> Option<PathBuf> {
    let exts: Vec<String> = pathext.split(';').filter(|e| !e.is_empty()).map(|e| e.to_ascii_lowercase()).collect();
    for folder in folders {
        for ext in &exts {
            let candidate = folder.join(format!("{command}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// The image path the companion should take: Codex draws by itself, Claude Code needs an OpenAI key.
pub fn image_mode(cli: Cli, has_openai_key: bool) -> &'static str {
    match (cli, has_openai_key) {
        (Cli::Codex, _) => "codex",
        (Cli::Claude, true) => "openai",
        (Cli::Claude, false) => "none",
    }
}

/// Every variable Castle adds to the session. `key` reads a kept key by its account name.
/// OPENAI_API_KEY is never given to Codex: Codex would switch from the member's ChatGPT login
/// to paid API billing.
pub fn session_env(cli: Cli, key: &dyn Fn(&str) -> Option<String>, tools: &Path) -> Vec<(String, String)> {
    let mut vars = vec![
        ("CASTLE_CLI".to_string(), cli.command().to_string()),
        ("CASTLE_TOOLS".to_string(), tools.display().to_string()),
    ];
    // castle-memory (in the tools folder) starts this same program with --memory
    if let Ok(exe) = std::env::current_exe() {
        vars.push(("CASTLE_EXE".into(), exe.display().to_string()));
    }
    let openai = key("openai");
    vars.push(("CASTLE_IMAGE".into(), image_mode(cli, openai.is_some()).into()));
    if cli == Cli::Claude {
        if let Some(value) = openai {
            vars.push(("OPENAI_API_KEY".into(), value));
        }
    }
    for (account, variable) in [("seedance", "ARK_API_KEY"), ("suno", "SUNOAPI_ORG_KEY"), ("elevenlabs", "ELEVENLABS_API_KEY")] {
        if let Some(value) = key(account) {
            vars.push((variable.into(), value));
        }
    }
    vars
}

/// Variables the session must not inherit from wherever Castle itself was started: a parent
/// Claude Code session's markers, and any API keys the member did not choose in Castle.
pub const NEVER_INHERITED: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SSE_PORT",
    "OPENAI_API_KEY",
    "ARK_API_KEY",
    "SUNOAPI_ORG_KEY",
    "ELEVENLABS_API_KEY",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn keys(set: &[&str]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = set.iter().map(|k| (k.to_string(), format!("value-of-{k}"))).collect();
        move |name| map.get(name).cloned()
    }

    fn get<'a>(vars: &'a [(String, String)], name: &str) -> Option<&'a str> {
        vars.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    #[test]
    fn codex_never_receives_the_openai_key() {
        let all = keys(&["openai", "seedance", "suno", "elevenlabs"]);
        let vars = session_env(Cli::Codex, &all, Path::new("C:\\tools"));
        assert_eq!(get(&vars, "OPENAI_API_KEY"), None);
        assert_eq!(get(&vars, "CASTLE_IMAGE"), Some("codex"));
        assert_eq!(get(&vars, "ARK_API_KEY"), Some("value-of-seedance"));
        assert_eq!(get(&vars, "SUNOAPI_ORG_KEY"), Some("value-of-suno"));
        assert_eq!(get(&vars, "ELEVENLABS_API_KEY"), Some("value-of-elevenlabs"));
        assert_eq!(get(&vars, "CASTLE_CLI"), Some("codex"));
    }

    #[test]
    fn claude_code_gets_the_openai_key_when_one_is_kept() {
        let vars = session_env(Cli::Claude, &keys(&["openai"]), Path::new("C:\\tools"));
        assert_eq!(get(&vars, "OPENAI_API_KEY"), Some("value-of-openai"));
        assert_eq!(get(&vars, "CASTLE_IMAGE"), Some("openai"));
        assert_eq!(get(&vars, "ARK_API_KEY"), None);
        assert_eq!(get(&vars, "CASTLE_TOOLS"), Some("C:\\tools"));
    }

    #[test]
    fn claude_code_without_a_key_draws_nothing_and_says_so() {
        let vars = session_env(Cli::Claude, &keys(&[]), Path::new("C:\\tools"));
        assert_eq!(get(&vars, "CASTLE_IMAGE"), Some("none"));
        assert_eq!(get(&vars, "OPENAI_API_KEY"), None);
    }

    #[test]
    fn a_command_is_found_by_path_order_then_pathext_order() {
        let root = std::env::temp_dir().join(format!("castle-path-{}", std::process::id()));
        let (first, second) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("tool.cmd"), "").unwrap();
        std::fs::write(second.join("tool.exe"), "").unwrap();
        // the first folder wins even though its file is a .cmd
        assert_eq!(find_in("tool", [first.clone(), second.clone()], ".EXE;.CMD"), Some(first.join("tool.cmd")));
        std::fs::write(first.join("tool.exe"), "").unwrap();
        // inside one folder .EXE comes before .CMD
        assert_eq!(find_in("tool", [first.clone(), second], ".EXE;.CMD"), Some(first.join("tool.exe")));
        assert_eq!(find_in("missing", [first], ".EXE"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
