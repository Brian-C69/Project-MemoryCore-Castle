//! Downloading a pinned release once and putting it in place: the database engine (MariaDB) and
//! the helpers' Node both come this way. A download is kept only when its SHA-256 is exactly the
//! release's own, and only a whole unpack ever replaces the folder that was there.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The lowercase hex SHA-256 of some bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `bytes` are exactly the file whose SHA-256 is `expected` (lowercase hex).
pub fn matches(bytes: &[u8], expected: &str) -> bool {
    sha256_hex(bytes) == expected.to_ascii_lowercase()
}

/// Downloads `url` whole, reporting `(done, total)` about four times a second. `fallback_total` is
/// the size to report when the server does not say. `what` names the thing in the one error line.
pub fn download(url: &str, fallback_total: u64, what: &str, progress: impl Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let offline = || format!("{what} could not be downloaded. Check the internet connection and try again.");
    let mut response = reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCore-Castle/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(900))
        .build()
        .and_then(|c| c.get(url).send())
        .and_then(|r| r.error_for_status())
        .map_err(|_| offline())?;
    let total = response.content_length().unwrap_or(fallback_total);
    let mut bytes = Vec::with_capacity(total as usize);
    let mut chunk = [0u8; 256 * 1024];
    let mut last = Instant::now();
    loop {
        let n = response.read(&mut chunk).map_err(|_| offline())?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
        if last.elapsed() > Duration::from_millis(250) {
            progress(bytes.len() as u64, total);
            last = Instant::now();
        }
    }
    progress(bytes.len() as u64, total);
    Ok(bytes)
}

/// A release zip holds one top folder (`mariadb-11.4.13-winx64/`, `node-v22.x-win-x64/`); its
/// contents land in `target`, skipping what `keep` refuses. `what` names the folder in the error.
pub fn unzip_dropping_top(bytes: &[u8], target: &Path, keep: impl Fn(&Path) -> bool, what: &str) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "The download was not a readable zip.".to_string())?;
    let partial = target.with_extension("partial");
    let _ = std::fs::remove_dir_all(&partial);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|_| "The zip could not be read.".to_string())?;
        let Some(inside) = file.enclosed_name() else { continue };
        let relative: PathBuf = inside.components().skip(1).collect();
        if relative.as_os_str().is_empty() || !keep(&relative) {
            continue;
        }
        let out = partial.join(relative);
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
    put_in_place(&partial, target, what)
}

/// A release `.tar.gz` (macOS Node) holds one top folder; its contents land in `target` with their
/// file modes, so `bin/node` stays runnable.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn untar_gz_dropping_top(bytes: &[u8], target: &Path, what: &str) -> Result<(), String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(std::io::Cursor::new(bytes)));
    archive.set_preserve_permissions(true);
    let partial = target.with_extension("partial");
    let _ = std::fs::remove_dir_all(&partial);
    std::fs::create_dir_all(&partial).map_err(|_| "A folder could not be created.".to_string())?;
    let entries = archive.entries().map_err(|_| "The download was not a readable archive.".to_string())?;
    for entry in entries {
        let mut entry = entry.map_err(|_| "The archive could not be read.".to_string())?;
        let inside = entry.path().map_err(|_| "The archive could not be read.".to_string())?.into_owned();
        // never outside the folder: no absolute paths, no `..`
        if inside.components().any(|c| !matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir)) {
            continue;
        }
        let relative: PathBuf = inside.components().filter(|c| matches!(c, std::path::Component::Normal(_))).skip(1).collect();
        if relative.as_os_str().is_empty() {
            continue;
        }
        let out = partial.join(&relative);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|_| "A folder could not be created.".to_string())?;
        }
        entry.unpack(&out).map_err(|_| "A file could not be written.".to_string())?;
    }
    put_in_place(&partial, target, what)
}

/// Only a whole unpack becomes the folder.
fn put_in_place(partial: &Path, target: &Path, what: &str) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(target);
    std::fs::rename(partial, target).map_err(|_| format!("The {what} folder could not be put in place."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("castle-fetch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_download_is_kept_only_when_it_is_the_release_itself() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert!(matches(b"abc", "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"));
        assert!(!matches(b"abd", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    }

    #[test]
    fn a_zip_lands_without_its_top_folder_and_skips_what_is_refused() {
        let mut bytes = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default();
            zip.add_directory("node-v1-win-x64/", options).unwrap();
            zip.start_file("node-v1-win-x64/node.exe", options).unwrap();
            zip.write_all(b"node").unwrap();
            zip.start_file("node-v1-win-x64/include/node.h", options).unwrap();
            zip.write_all(b"header").unwrap();
            zip.finish().unwrap();
        }
        let dir = temp("zip");
        let target = dir.join("v1");
        std::fs::create_dir_all(target.join("old")).unwrap();
        unzip_dropping_top(&bytes, &target, |p| !p.starts_with("include"), "test").unwrap();
        assert_eq!(std::fs::read(target.join("node.exe")).unwrap(), b"node");
        assert!(!target.join("include").exists(), "a refused path is not unpacked");
        assert!(!target.join("old").exists(), "the whole unpack replaces what was there");
        assert!(!target.with_extension("partial").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_zip_leaves_the_old_folder_alone() {
        let dir = temp("badzip");
        let target = dir.join("v1");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("node.exe"), b"old").unwrap();
        assert!(unzip_dropping_top(b"not a zip", &target, |_| true, "test").is_err());
        assert_eq!(std::fs::read(target.join("node.exe")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tar_gz_lands_without_its_top_folder_and_keeps_the_runnable_bit() {
        let mut bytes = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut bytes, flate2::Compression::fast());
            let mut tar = tar::Builder::new(gz);
            let body = b"#!/bin/sh\necho v22.0.0\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, "node-v1-darwin-arm64/bin/node", &body[..]).unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        let dir = temp("tar");
        let target = dir.join("v1");
        untar_gz_dropping_top(&bytes, &target, "test").unwrap();
        let node = target.join("bin").join("node");
        assert!(node.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(std::fs::metadata(&node).unwrap().permissions().mode() & 0o111, 0);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
