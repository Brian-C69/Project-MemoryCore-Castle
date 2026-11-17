//! Small copies of STUDIO's pictures. A drawing is a PNG of a megabyte or two, and a gallery of them
//! only shows up once every one has been read and decoded; the gallery shows these small JPEGs
//! instead (a few tens of KB each) and keeps the real picture for the viewer. They are made once, in
//! the background, kept in Castle's own settings folder (never in the memory folder), and made again
//! only when the picture changes. A picture that cannot be read simply has no small copy and shows
//! as it always did.

use crate::settings::SettingsStore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Manager, State};

/// The small copy fits inside this, whatever the picture's shape (a card shows about 200 px wide).
const FIT_WIDTH: u32 = 480;
const FIT_HEIGHT: u32 = 720;
/// A call makes at most this many new copies, so a first run with a large gallery is done in turns
/// and the window is never held for long.
const MAKE_AT_MOST: usize = 40;
/// Pictures bigger than this are left alone (they would take too long to decode here).
const SOURCE_LIMIT: u64 = 40 * 1024 * 1024;

fn folder(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("thumbs"))
}

/// The small copy's file name: the same picture, unchanged, always gives the same name; a picture
/// that was drawn again (new time or size) gives a new one. Pure, tested.
pub fn thumb_name(path: &str, modified_secs: u64, len: u64) -> String {
    let mut hash = Sha256::new();
    hash.update(path.as_bytes());
    hash.update(modified_secs.to_le_bytes());
    hash.update(len.to_le_bytes());
    let digest = hash.finalize();
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    format!("{hex}.jpg")
}

fn stamp(file: &Path) -> Option<(u64, u64)> {
    let meta = file.metadata().ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some((modified, meta.len()))
}

/// Makes the small copy of `src` at `dest` (written whole or not at all).
pub fn make(src: &Path, dest: &Path) -> Result<(), String> {
    let image = image::ImageReader::open(src)
        .map_err(|_| "The picture could not be opened.".to_string())?
        .with_guessed_format()
        .map_err(|_| "The picture's type is not known.".to_string())?
        .decode()
        .map_err(|_| "The picture could not be read.".to_string())?;
    let small = image.thumbnail(FIT_WIDTH, FIT_HEIGHT).to_rgb8();
    let partial = dest.with_extension("part");
    {
        let mut out = std::fs::File::create(&partial).map_err(|_| "The small copy could not be written.".to_string())?;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82)
            .encode_image(&small)
            .map_err(|_| "The small copy could not be made.".to_string())?;
    }
    std::fs::rename(&partial, dest).map_err(|_| "The small copy could not be kept.".to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumb {
    pub path: String,
    pub thumb: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbs {
    pub thumbs: Vec<Thumb>,
    /// pictures still without a small copy (the page asks again until it is 0)
    pub pending: usize,
}

/// Small copies for the pictures named (inside the memory folder only): the ones that exist, after
/// making up to MAKE_AT_MOST new ones.
#[tauri::command]
pub async fn studio_thumbs(app: AppHandle, paths: Vec<String>, store: State<'_, SettingsStore>) -> Result<Thumbs, String> {
    let memory = store.get().memory_folder.map(PathBuf::from).ok_or("No MemoryCore folder is chosen yet.")?;
    let dir = folder(&app).ok_or("Castle's data folder could not be found.")?;
    std::fs::create_dir_all(&dir).map_err(|_| "The small copies' folder could not be made.".to_string())?;
    // the page may show these through the asset protocol, like the memory folder's own files
    let _ = app.asset_protocol_scope().allow_directory(&dir, true);
    tauri::async_runtime::spawn_blocking(move || collect(&memory, &dir, &paths))
        .await
        .map_err(|_| "The small copies could not be made.".to_string())
}

fn collect(memory: &Path, dir: &Path, paths: &[String]) -> Thumbs {
    let mut thumbs = Vec::new();
    let (mut made, mut pending) = (0, 0);
    for path in paths {
        let Ok(file) = crate::files::fenced(memory, path) else { continue };
        let Some((modified, len)) = stamp(&file) else { continue };
        if len > SOURCE_LIMIT {
            continue;
        }
        let dest = dir.join(thumb_name(path, modified, len));
        if !dest.is_file() {
            if made >= MAKE_AT_MOST {
                pending += 1;
                continue;
            }
            made += 1;
            if make(&file, &dest).is_err() {
                continue;
            }
        }
        thumbs.push(Thumb { path: path.clone(), thumb: dest.display().to_string() });
    }
    Thumbs { thumbs, pending }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("castle-thumbs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 120])).save(path).unwrap();
    }

    #[test]
    fn the_same_picture_gives_the_same_name_and_a_changed_one_a_new_name() {
        let a = thumb_name("D:/memory/a.png", 100, 5000);
        assert_eq!(a, thumb_name("D:/memory/a.png", 100, 5000));
        assert_ne!(a, thumb_name("D:/memory/a.png", 101, 5000), "drawn again");
        assert_ne!(a, thumb_name("D:/memory/a.png", 100, 5001), "a different size");
        assert_ne!(a, thumb_name("D:/memory/b.png", 100, 5000));
        assert!(a.ends_with(".jpg") && a.len() == 36);
    }

    #[test]
    fn a_big_picture_becomes_a_small_one_that_keeps_its_shape() {
        let dir = temp("make");
        let src = dir.join("big.png");
        write_png(&src, 1024, 1536);
        let dest = dir.join("small.jpg");
        make(&src, &dest).unwrap();
        let small = image::open(&dest).unwrap();
        assert!(small.width() <= FIT_WIDTH && small.height() <= FIT_HEIGHT);
        assert_eq!((small.width() * 3 + 1) / 2, small.height(), "still 2:3");
        assert!(dest.metadata().unwrap().len() < 100 * 1024, "a small copy is a few tens of KB");
        assert!(!dest.with_extension("part").exists(), "no half-written file is left");
    }

    #[test]
    fn a_picture_that_cannot_be_read_has_no_small_copy_and_makes_no_trouble() {
        let dir = temp("bad");
        let src = dir.join("broken.png");
        std::fs::write(&src, b"not a picture").unwrap();
        assert!(make(&src, &dir.join("x.jpg")).is_err());
        assert!(!dir.join("x.jpg").exists());
    }

    #[test]
    fn only_pictures_inside_the_memory_folder_get_small_copies_and_a_run_is_made_in_turns() {
        let root = temp("collect");
        let memory = root.join("memory");
        let outside = root.join("outside");
        let thumbs = root.join("thumbs");
        for d in [&memory, &outside, &thumbs] {
            std::fs::create_dir_all(d).unwrap();
        }
        let mut paths = Vec::new();
        for n in 0..(MAKE_AT_MOST + 5) {
            let p = memory.join(format!("p{n}.png"));
            write_png(&p, 40, 60);
            paths.push(p.display().to_string());
        }
        let stray = outside.join("secret.png");
        write_png(&stray, 40, 60);
        paths.push(stray.display().to_string());
        let first = collect(&memory, &thumbs, &paths);
        assert_eq!(first.thumbs.len(), MAKE_AT_MOST);
        assert_eq!(first.pending, 5, "the rest wait for the next turn");
        assert!(!first.thumbs.iter().any(|t| t.path.contains("secret")), "outside the memory folder is never touched");
        let second = collect(&memory, &thumbs, &paths);
        assert_eq!((second.thumbs.len(), second.pending), (MAKE_AT_MOST + 5, 0), "nothing is made twice, the rest are done");
    }
}
