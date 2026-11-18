//! HOME's DISKS widget: how much room each drive of this computer has left.
//!
//! Read-only and local: Castle asks the operating system for the list of drives and their sizes,
//! and sends nothing anywhere. It never writes to or cleans a drive (that is Sweep's job).

use serde::Serialize;

/// One drive as the widget draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// "C:\" on Windows, "/" or "/Volumes/Backup" elsewhere
    pub mount: String,
    /// the name the drive was given ("Games"), or empty
    pub label: String,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
    /// "SSD", "HDD" or empty when the system does not say
    pub media: String,
}

/// What the system reported for one drive, before Castle decides whether it is worth showing.
#[derive(Debug, Clone)]
pub struct Raw {
    pub mount: String,
    pub label: String,
    pub file_system: String,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
    pub media: String,
}

/// File systems that are not a place to keep things (memory disks, containers, snap packages).
const PSEUDO: &[&str] = &["tmpfs", "devtmpfs", "squashfs", "overlay", "ramfs", "proc", "sysfs", "cgroup", "efivarfs", "fuse.portal"];

/// Mounts that repeat or hide a drive that is already listed (macOS system volumes, boot firmware,
/// snap and container storage).
const HIDDEN_MOUNTS: &[&str] = &["/System/Volumes", "/boot", "/snap", "/var/lib/docker", "/private/var/vm", "/Library/Developer"];

fn hidden(mount: &str) -> bool {
    HIDDEN_MOUNTS.iter().any(|prefix| mount == *prefix || mount.starts_with(&format!("{prefix}/")))
}

/// The drives worth showing, in a steady order: no empty drives (a card reader with nothing in it),
/// no memory or container disks, no repeats; "free" never more than "total".
pub fn shape(raw: Vec<Raw>) -> Vec<Drive> {
    let mut seen = std::collections::HashSet::new();
    let mut drives: Vec<Drive> = raw
        .into_iter()
        .filter(|d| d.total > 0)
        .filter(|d| !PSEUDO.contains(&d.file_system.to_lowercase().as_str()))
        .filter(|d| !hidden(&d.mount))
        .filter(|d| seen.insert(d.mount.to_lowercase()))
        .map(|d| Drive { mount: d.mount, label: d.label.trim().to_string(), total: d.total, free: d.free.min(d.total), removable: d.removable, media: d.media })
        .collect();
    drives.sort_by(|a, b| a.mount.to_lowercase().cmp(&b.mount.to_lowercase()));
    drives
}

fn read() -> Vec<Raw> {
    use sysinfo::{DiskKind, Disks};
    let Ok(disks) = Disks::new_with_refreshed_list() else { return Vec::new() };
    disks
        .iter()
        .map(|d| Raw {
            mount: d.mount_point().to_string_lossy().to_string(),
            label: d.name().to_string_lossy().to_string(),
            file_system: d.file_system().to_string_lossy().to_string(),
            total: d.total_space(),
            free: d.available_space(),
            removable: d.is_removable(),
            media: match d.kind() {
                DiskKind::SSD => "SSD".to_string(),
                DiskKind::HDD => "HDD".to_string(),
                _ => String::new(),
            },
        })
        .collect()
}

/// The drives of this computer and how full each is.
#[tauri::command]
pub async fn disk_drives() -> Vec<Drive> {
    // reading the drive list can wait on a sleeping disk or an unreachable network drive
    tauri::async_runtime::spawn_blocking(|| shape(read())).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(mount: &str, fs: &str, total: u64, free: u64) -> Raw {
        Raw { mount: mount.into(), label: String::new(), file_system: fs.into(), total, free, removable: false, media: String::new() }
    }

    #[test]
    fn lists_each_drive_in_a_steady_order() {
        let drives = shape(vec![raw("D:\\", "NTFS", 1000, 400), raw("C:\\", "NTFS", 500, 100)]);
        let mounts: Vec<&str> = drives.iter().map(|d| d.mount.as_str()).collect();
        assert_eq!(mounts, vec!["C:\\", "D:\\"]);
    }

    #[test]
    fn leaves_out_a_drive_with_nothing_in_it() {
        assert!(shape(vec![raw("E:\\", "", 0, 0)]).is_empty());
    }

    #[test]
    fn leaves_out_memory_and_container_disks() {
        let drives = shape(vec![raw("/", "ext4", 100, 40), raw("/run", "tmpfs", 50, 50), raw("/snap/core", "squashfs", 60, 0), raw("/boot/efi", "vfat", 10, 9)]);
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].mount, "/");
    }

    #[test]
    fn leaves_out_macos_system_volumes_but_keeps_an_external_drive() {
        let drives = shape(vec![raw("/", "apfs", 500, 200), raw("/System/Volumes/Data", "apfs", 500, 200), raw("/Volumes/Backup", "exfat", 900, 300)]);
        let mounts: Vec<&str> = drives.iter().map(|d| d.mount.as_str()).collect();
        assert_eq!(mounts, vec!["/", "/Volumes/Backup"]);
    }

    #[test]
    fn a_mount_is_listed_once() {
        assert_eq!(shape(vec![raw("C:\\", "NTFS", 500, 100), raw("c:\\", "NTFS", 500, 100)]).len(), 1);
    }

    #[test]
    fn free_is_never_more_than_total() {
        assert_eq!(shape(vec![raw("C:\\", "NTFS", 500, 900)])[0].free, 500);
    }

    #[test]
    fn a_name_is_trimmed_and_kept() {
        let mut d = raw("D:\\", "NTFS", 500, 100);
        d.label = "  Games ".into();
        assert_eq!(shape(vec![d])[0].label, "Games");
    }

    #[test]
    fn a_folder_that_only_starts_like_a_hidden_mount_is_kept() {
        assert_eq!(shape(vec![raw("/bootcamp", "ext4", 100, 50)]).len(), 1);
    }
}
