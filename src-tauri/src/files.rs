//! Handing a file to the screen safely: only from inside a folder Castle may read, and only
//! pictures and sound, as data the screen draws or plays.

use std::path::{Path, PathBuf};

/// `path` resolved and checked to lie inside `root` (both resolved first, so `..` or a link
/// cannot step outside). The error is a sentence for the screen.
pub fn fenced(root: &Path, path: &str) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|_| "That folder is not here.".to_string())?;
    let file = Path::new(path).canonicalize().map_err(|_| "The file is gone.".to_string())?;
    if file.starts_with(&root) && file.is_file() {
        Ok(file)
    } else {
        Err("That file is outside what Castle may open.".into())
    }
}

/// Like `fenced`, but a relative path (as an index file writes them) is taken inside `root`.
pub fn fenced_in(root: &Path, path: &str) -> Result<PathBuf, String> {
    let given = Path::new(path);
    if given.is_absolute() {
        fenced(root, path)
    } else {
        fenced(root, &root.join(given).display().to_string())
    }
}

/// The type the screen needs for a picture or sound file, or None for anything else.
pub fn mime(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "m4a" => Some("audio/mp4"),
        _ => None,
    }
}

/// A picture (up to 25 MB) or sound (up to 30 MB) as a data URL.
pub fn data_url(path: &Path) -> Result<String, String> {
    use base64::Engine;
    let mime = mime(path).ok_or("That file is not a picture or a sound.")?;
    let limit: u64 = if mime.starts_with("audio") { 30 } else { 25 } * 1024 * 1024;
    let size = std::fs::metadata(path).map_err(|_| "The file is gone.".to_string())?.len();
    if size > limit {
        return Err("The file is too large to show here.".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "The file could not be read.".to_string())?;
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_files_inside_the_folder_pass() {
        let base = std::env::temp_dir().join(format!("castle-fence-{}", std::process::id()));
        let inside = base.join("diary");
        std::fs::create_dir_all(inside.join("day")).unwrap();
        std::fs::write(inside.join("day").join("a.png"), b"png").unwrap();
        std::fs::write(base.join("secret.png"), b"png").unwrap();

        assert!(fenced(&inside, &inside.join("day").join("a.png").display().to_string()).is_ok());
        let escape = inside.join("day").join("..").join("..").join("secret.png");
        assert!(fenced(&inside, &escape.display().to_string()).is_err());
        assert!(fenced(&inside, &base.join("secret.png").display().to_string()).is_err());
        assert!(fenced(&inside, &inside.join("day").display().to_string()).is_err());
        assert_eq!(data_url(&inside.join("day").join("a.png")).unwrap(), "data:image/png;base64,cG5n");
        assert!(data_url(&base.join("notes.txt")).is_err());
        // an index writes paths relative to the folder: taken inside it, and still fenced
        assert!(fenced_in(&inside, "day/a.png").is_ok());
        assert!(fenced_in(&inside, "../secret.png").is_err());
        assert!(fenced_in(&inside, &inside.join("day").join("a.png").display().to_string()).is_ok());
        let _ = std::fs::remove_dir_all(base);
    }
}
