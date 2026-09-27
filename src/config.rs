//! User settings, kept in %APPDATA%\halo-battery-rs\config.json.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const INTERVALS: [u64; 5] = [15, 30, 60, 120, 300];
pub const THRESHOLDS: [u8; 5] = [10, 15, 20, 25, 30];

#[derive(Clone, Debug, Serialize, Deserialize)]
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

fn dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("halo-battery-rs"))
}

impl Config {
    pub fn load() -> Self {
        dir()
            .and_then(|d| std::fs::read_to_string(d.join("config.json")).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(d) = dir() else { return };
        if std::fs::create_dir_all(&d).is_ok() {
            if let Ok(s) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(d.join("config.json"), s);
            }
        }
    }
}
