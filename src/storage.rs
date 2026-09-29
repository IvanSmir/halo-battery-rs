//! Where the app keeps its files, and how they are written. The tray and the
//! settings window share them, so a write must never leave a half-written
//! file for the other process to read.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// %APPDATA%\halo-battery-rs, when %APPDATA% is set.
pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("halo-battery-rs"))
}

/// Writes `value` as pretty JSON to a temporary file next to `path`, then
/// renames it over `path`, so readers see either the old or the new file.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

/// The JSON in `path`, or `None` when it is missing or not valid.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

/// Last modification time of `path`, to notice changes made by the other process.
pub fn modified(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Notices when another process rewrote a file, by its modification time,
/// checking at most once per `every`.
pub struct FileWatch {
    path: PathBuf,
    seen: Option<std::time::SystemTime>,
    every: std::time::Duration,
    checked: std::time::Instant,
}

impl FileWatch {
    /// Starts watching `path` as it is now: only later changes count.
    pub fn new(path: PathBuf, every: std::time::Duration) -> Self {
        let seen = modified(&path);
        Self { path, seen, every, checked: std::time::Instant::now() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `true` once per change of the file (created, rewritten or removed).
    pub fn changed(&mut self) -> bool {
        if self.checked.elapsed() < self.every {
            return false;
        }
        self.checked = std::time::Instant::now();
        let now = modified(&self.path);
        if now == self.seen {
            return false;
        }
        self.seen = now;
        true
    }

    /// Marks the current state of the file as seen, e.g. after writing it ourselves.
    pub fn mark_seen(&mut self) {
        self.seen = modified(&self.path);
    }
}
