//! Codex's record: filled in by Task 3.2. Until then Codex sessions show in the Log only.

use super::Follower;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn newest_record(_folder: &Path, _since: SystemTime) -> Option<PathBuf> {
    None
}

pub fn follower(path: PathBuf, since: SystemTime) -> Follower {
    Follower::new(path, since, |_| Vec::new(), |_| Vec::new())
}
