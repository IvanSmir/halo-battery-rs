//! User settings, kept in %APPDATA%\halo-battery-rs\config.json.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Poll intervals offered in the menu, in seconds.
pub const INTERVALS: [u64; 5] = [15, 30, 60, 120, 300];
/// Low-battery thresholds offered in the menu, in percent.
pub const THRESHOLDS: [u8; 5] = [10, 15, 20, 25, 30];

/// Bounds a hand-edited file is clamped to: polling faster than this only
/// wastes power, and a threshold outside this range makes no sense.
const MIN_INTERVAL_SECS: u64 = 5;
const MAX_INTERVAL_SECS: u64 = 3600;
const MIN_THRESHOLD: u8 = 5;
const MAX_THRESHOLD: u8 = 50;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Seconds between two battery reads.
    pub interval_secs: u64,
    /// Low-battery threshold in percent: the ring turns red and a notification is shown.
    pub low_threshold: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self { interval_secs: 60, low_threshold: 20 }
    }
}

impl Config {
    /// %APPDATA%\halo-battery-rs\config.json, when %APPDATA% is set.
    pub fn default_path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("halo-battery-rs").join("config.json"))
    }

    /// The settings at the default path, or the defaults.
    pub fn load() -> Self {
        Self::default_path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    /// The settings in `path`; defaults for a missing or unreadable file and
    /// for missing fields, out-of-range values clamped.
    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Saves to the default path. Errors are ignored: losing a menu choice is
    /// not worth interrupting the app for.
    pub fn save(&self) {
        if let Some(p) = Self::default_path() {
            let _ = self.save_to(&p);
        }
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn sanitized(self) -> Self {
        Self {
            interval_secs: self.interval_secs.clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS),
            low_threshold: self.low_threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD),
        }
    }
}
