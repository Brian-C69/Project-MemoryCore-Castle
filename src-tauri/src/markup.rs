//! Marked copies of the pictures the member put in a Chat (the companion's or a DEV tab's): the
//! markup sheet draws on the picture in the page and hands the finished PNG here, which saves it
//! as a new file beside the original. The original is never written to.

use crate::chat::MemberPictures;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

/// The marked copies Castle wrote this run: only these are ever written over (when the member
/// marks the same picture again before sending it).
#[derive(Default)]
pub struct MarkedCopies(pub Mutex<HashSet<String>>);

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// A fresh name for the marked copy of `original` in `folder`: `<name>-marked.png`, then
/// `<name>-marked-2.png` and on while `taken` says a name is in use. Pure, tested.
pub fn marked_copy_path(original: &Path, folder: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let stem = original.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into());
    let first = folder.join(format!("{stem}-marked.png"));
    if !taken(&first) {
        return first;
    }
    (2..)
        .map(|n| folder.join(format!("{stem}-marked-{n}.png")))
        .find(|candidate| !taken(candidate))
        .expect("an unused name")
}

/// Saves `png` as the marked copy of `original` and returns where. `previous` is the copy shown
/// for it now; it is written over only when Castle wrote it this run (`ours`). Otherwise the copy
/// goes beside the original under a fresh name, or into `fallback` when that folder refuses it.
/// Never the original itself, and only a real PNG.
pub fn save_marked(original: &Path, png: &[u8], previous: Option<&Path>, ours: &HashSet<String>, fallback: &Path) -> Result<PathBuf, String> {
    if !png.starts_with(&PNG_SIGNATURE) {
        return Err("The marked picture is not a PNG.".into());
    }
    if !original.is_file() {
        return Err("The original picture is gone.".into());
    }
    let same_as_original = |p: &Path| p == original || std::fs::canonicalize(p).ok().zip(std::fs::canonicalize(original).ok()).is_some_and(|(a, b)| a == b);
    if let Some(previous) = previous.filter(|p| ours.contains(&p.display().to_string()) && !same_as_original(p)) {
        if std::fs::write(previous, png).is_ok() {
            return Ok(previous.to_path_buf());
        }
    }
    let taken = |p: &Path| p.exists();
    let beside = original.parent().map(|folder| marked_copy_path(original, folder, taken));
    if let Some(path) = beside.filter(|p| !same_as_original(p)) {
        if std::fs::write(&path, png).is_ok() {
            return Ok(path);
        }
    }
    std::fs::create_dir_all(fallback).map_err(|_| "The marked picture could not be saved.".to_string())?;
    let path = marked_copy_path(original, fallback, taken);
    std::fs::write(&path, png).map_err(|_| "The marked picture could not be saved.".to_string())?;
    Ok(path)
}

/// The markup sheet's "Use marked copy": `picture` is the finished PNG as a data address,
/// `original` a picture the member dropped, pasted or typed this run, `previous` the marked copy
/// shown for it now (if any). Returns the new copy's path, which Chat may then show and send.
#[tauri::command]
pub fn chat_markup_save(
    original: String,
    picture: String,
    previous: Option<String>,
    member: State<MemberPictures>,
    marked: State<MarkedCopies>,
) -> Result<String, String> {
    if !member.0.lock().unwrap().contains(&original) {
        return Err("That picture is not one you put in this chat.".into());
    }
    let (mime, bytes) = crate::chatroom::picture_from_data_address(&picture).ok_or("The marked picture could not be read.")?;
    if mime != "image/png" {
        return Err("The marked picture is not a PNG.".into());
    }
    let ours = marked.0.lock().unwrap().clone();
    let path = save_marked(Path::new(&original), &bytes, previous.as_deref().map(Path::new), &ours, &crate::clipboard::paste_folder())?;
    let text = path.display().to_string();
    marked.0.lock().unwrap().insert(text.clone());
    member.0.lock().unwrap().insert(text.clone());
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("castle-markup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn png(tail: &[u8]) -> Vec<u8> {
        [PNG_SIGNATURE.as_slice(), tail].concat()
    }

    #[test]
    fn a_marked_copy_is_named_after_the_original_and_never_takes_a_used_name() {
        let folder = Path::new("/pics");
        let original = Path::new("/pics/shot one.jpg");
        assert_eq!(marked_copy_path(original, folder, |_| false), folder.join("shot one-marked.png"));
        let used = [folder.join("shot one-marked.png"), folder.join("shot one-marked-2.png")];
        assert_eq!(marked_copy_path(original, folder, |p| used.iter().any(|u| u == p)), folder.join("shot one-marked-3.png"));
    }

    #[test]
    fn the_copy_goes_beside_the_original_and_the_original_is_kept() {
        let folder = temp("beside");
        let original = folder.join("shot.png");
        std::fs::write(&original, png(b"original")).unwrap();
        let saved = save_marked(&original, &png(b"marked"), None, &HashSet::new(), &folder.join("fallback")).unwrap();
        assert_eq!(saved, folder.join("shot-marked.png"));
        assert_eq!(std::fs::read(&saved).unwrap(), png(b"marked"));
        assert_eq!(std::fs::read(&original).unwrap(), png(b"original"));
        // a second picture of the same name never writes over a file it did not make
        let again = save_marked(&original, &png(b"other"), None, &HashSet::new(), &folder.join("fallback")).unwrap();
        assert_eq!(again, folder.join("shot-marked-2.png"));
        assert_eq!(std::fs::read(&saved).unwrap(), png(b"marked"));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn marking_again_writes_over_only_castles_own_copy() {
        let folder = temp("again");
        let original = folder.join("shot.png");
        std::fs::write(&original, png(b"original")).unwrap();
        let first = save_marked(&original, &png(b"one"), None, &HashSet::new(), &folder).unwrap();
        let ours = HashSet::from([first.display().to_string()]);
        let second = save_marked(&original, &png(b"two"), Some(&first), &ours, &folder).unwrap();
        assert_eq!(second, first);
        assert_eq!(std::fs::read(&first).unwrap(), png(b"two"));
        // a "previous" Castle did not write is left alone, and so is the original
        let stranger = folder.join("notes.png");
        std::fs::write(&stranger, b"keep").unwrap();
        let third = save_marked(&original, &png(b"three"), Some(&stranger), &ours, &folder).unwrap();
        assert_ne!(third, stranger);
        assert_eq!(std::fs::read(&stranger).unwrap(), b"keep");
        let sneaky = HashSet::from([original.display().to_string()]);
        let fourth = save_marked(&original, &png(b"four"), Some(&original), &sneaky, &folder).unwrap();
        assert_ne!(fourth, original);
        assert_eq!(std::fs::read(&original).unwrap(), png(b"original"));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn only_a_png_for_a_picture_that_exists_is_saved() {
        let folder = temp("refuse");
        let original = folder.join("shot.png");
        assert!(save_marked(&original, &png(b"x"), None, &HashSet::new(), &folder).is_err());
        std::fs::write(&original, png(b"original")).unwrap();
        assert!(save_marked(&original, b"GIF89a....", None, &HashSet::new(), &folder).is_err());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(folder);
    }
}
