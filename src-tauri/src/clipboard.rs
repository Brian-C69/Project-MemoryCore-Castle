//! Copy and paste through the Windows clipboard itself, not the browser's, which WebView2 can
//! refuse inside the app. A pasted picture becomes a PNG file whose path is pasted instead.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Pasted pictures older than this are cleared at start.
pub const PASTES_KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pasted {
    /// "text", "files", "picture" or "empty"
    pub kind: &'static str,
    /// What goes into the terminal: the text, the files' paths, or the saved picture's path.
    pub text: String,
}

/// Where pasted pictures are saved: %TEMP%\memorycore-castle-paste.
pub fn paste_folder() -> PathBuf {
    std::env::temp_dir().join("memorycore-castle-paste")
}

/// A path as a terminal wants it: quoted when it has a space.
pub fn quoted(path: &Path) -> String {
    let text = path.display().to_string();
    if text.contains(' ') {
        format!("\"{text}\"")
    } else {
        text
    }
}

/// Saves raw RGBA pixels as a PNG in `folder` and returns its path.
pub fn save_png(folder: &Path, width: usize, height: usize, rgba: &[u8]) -> Result<PathBuf, String> {
    let failed = || "The pasted picture could not be saved.".to_string();
    if width == 0 || height == 0 || rgba.len() < width * height * 4 {
        return Err(failed());
    }
    fs::create_dir_all(folder).map_err(|_| failed())?;
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let path = folder.join(format!("paste-{millis}.png"));
    let file = fs::File::create(&path).map_err(|_| failed())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| failed())?;
    writer.write_image_data(&rgba[..width * height * 4]).map_err(|_| failed())?;
    writer.finish().map_err(|_| failed())?;
    Ok(path)
}

/// Removes pasted pictures older than `age`. Returns how many were removed.
pub fn clear_old(folder: &Path, age: Duration) -> usize {
    let Ok(entries) = fs::read_dir(folder) else {
        return 0;
    };
    let now = SystemTime::now();
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("paste-"))
        .filter(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .map(|t| now.duration_since(t).unwrap_or_default() >= age)
                .unwrap_or(false)
        })
        .filter(|e| fs::remove_file(e.path()).is_ok())
        .count()
}

/// Reads the clipboard: copied files first (their paths), then text, then a picture (saved).
pub fn read() -> Result<Pasted, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|_| "The clipboard could not be opened.".to_string())?;

    if let Ok(files) = clipboard.get().file_list() {
        if !files.is_empty() {
            let text = files.iter().map(|p| quoted(p)).collect::<Vec<_>>().join(" ");
            return Ok(Pasted { kind: "files", text });
        }
    }
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Ok(Pasted { kind: "text", text });
        }
    }
    if let Ok(image) = clipboard.get_image() {
        let path = save_png(&paste_folder(), image.width, image.height, &image.bytes)?;
        return Ok(Pasted { kind: "picture", text: quoted(&path) });
    }
    Ok(Pasted { kind: "empty", text: String::new() })
}

pub fn write_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_string()))
        .map_err(|_| "The clipboard could not be written.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("castle-clip-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn a_picture_is_saved_as_a_readable_png() {
        let folder = temp("png");
        // 2 x 1: one purple pixel, one white pixel.
        let pixels = [168u8, 120, 255, 255, 255, 255, 255, 255];
        let path = save_png(&folder, 2, 1, &pixels).unwrap();
        let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(&path).unwrap()));
        let reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (2, 1));
        assert!(save_png(&folder, 4, 4, &pixels).is_err());
    }

    #[test]
    fn old_pastes_are_cleared_and_other_files_left() {
        let folder = temp("clear");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("paste-1.png"), b"x").unwrap();
        fs::write(folder.join("notes.txt"), b"x").unwrap();
        assert_eq!(clear_old(&folder, Duration::from_secs(3600)), 0);
        assert_eq!(clear_old(&folder, Duration::ZERO), 1);
        assert!(folder.join("notes.txt").exists());
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        assert_eq!(quoted(Path::new("C:\\a.png")), "C:\\a.png");
        assert_eq!(quoted(Path::new("D:\\Afif Personal\\a.png")), "\"D:\\Afif Personal\\a.png\"");
    }
}
