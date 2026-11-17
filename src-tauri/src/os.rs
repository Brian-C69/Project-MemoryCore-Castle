//! What macOS (and Linux) do where Castle's Windows code calls Explorer or rundll32: open a link
//! in the browser, open a file in its own app, or show it in the Finder. Windows keeps its own calls
//! at each place; these are the other side of those `#[cfg(windows)]` branches.

use std::path::Path;
use std::process::Command;

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(all(unix, not(target_os = "macos")))]
const OPENER: &str = "xdg-open";

/// A link in the member's default browser.
#[cfg(not(windows))]
pub fn open_link(url: &str) -> Result<(), String> {
    Command::new(OPENER).arg(url).spawn().map(|_| ()).map_err(|_| "The browser could not be opened.".to_string())
}

/// A file in its own app, or (`select`) its folder in the Finder with the file selected.
#[cfg(not(windows))]
pub fn open_path(path: &Path, select: bool) -> Result<(), String> {
    let mut command = Command::new(OPENER);
    #[cfg(target_os = "macos")]
    if select {
        command.arg("-R");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    let path = if select { path.parent().unwrap_or(path) } else { path };
    command.arg(path).spawn().map(|_| ()).map_err(|_| "macOS could not open it.".to_string())
}
