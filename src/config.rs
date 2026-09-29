//! User settings, kept in %APPDATA%\halo-battery-rs\config.json. The settings
//! window writes the file; the tray reads it at start and whenever it changes.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage;

/// Bounds a hand-edited file is clamped to: polling faster than this only
/// wastes power, and a threshold outside this range makes no sense.
const MIN_INTERVAL_SECS: u64 = 5;
const MAX_INTERVAL_SECS: u64 = 3600;
const MIN_THRESHOLD: u8 = 5;
const MAX_THRESHOLD: u8 = 50;
/// Longest custom device name kept.
const MAX_ALIAS_CHARS: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Seconds between two battery reads.
    pub interval_secs: u64,
    /// Low-battery threshold in percent: the ring turns red and a notification is shown.
    pub low_threshold: u8,
    pub notifications: Notifications,
    pub appearance: Appearance,
    pub updates: Updates,
    /// Per-device settings by device key; devices not listed use the defaults.
    pub devices: BTreeMap<String, DeviceSettings>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Updates {
    /// Look for a newer release once a day (the only thing that goes online).
    pub check: bool,
}

impl Default for Updates {
    fn default() -> Self {
        Self { check: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    /// Master switch for every notification.
    pub enabled: bool,
    /// Warn once when a charging device reaches 100 %.
    pub full_charge: bool,
    /// Play the notification sound.
    pub sound: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RingColor {
    /// The taskbar's own foreground: white on a dark taskbar, black on a light one.
    #[default]
    Auto,
    Blue,
    Violet,
    Mint,
    Rose,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub ring_color: RingColor,
    /// The arc of a charging device "breathes".
    pub animate: bool,
    /// Draw the device pictogram inside the ring.
    pub pictogram: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceSettings {
    /// A name chosen by the user; empty keeps the detected one.
    pub alias: String,
    /// Show a tray icon for the device.
    pub visible: bool,
    /// Notifications for this device.
    pub notify: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            low_threshold: 20,
            notifications: Notifications::default(),
            appearance: Appearance::default(),
            updates: Updates::default(),
            devices: BTreeMap::new(),
        }
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self { enabled: true, full_charge: false, sound: true }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self { ring_color: RingColor::Auto, animate: true, pictogram: true }
    }
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self { alias: String::new(), visible: true, notify: true }
    }
}

impl DeviceSettings {
    /// Whether it holds anything the defaults do not; such entries are not saved.
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Config {
    /// %APPDATA%\halo-battery-rs\config.json, when %APPDATA% is set.
    pub fn default_path() -> Option<PathBuf> {
        storage::data_dir().map(|d| d.join("config.json"))
    }

    /// The settings at the default path, or the defaults.
    pub fn load() -> Self {
        Self::default_path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    /// The settings in `path`; defaults for a missing or unreadable file and
    /// for missing fields, out-of-range values clamped.
    pub fn load_from(path: &Path) -> Self {
        storage::read_json::<Self>(path).unwrap_or_default().sanitized()
    }

    /// Like [`Config::load_from`], but says when an existing file cannot be
    /// used. A missing file is not a problem: it just means nothing is set yet.
    pub fn try_load_from(path: &Path) -> io::Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        let config: Self = serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(config.sanitized())
    }

    /// Saves to the default path.
    pub fn save(&self) -> io::Result<()> {
        match Self::default_path() {
            Some(p) => self.save_to(&p),
            None => Err(io::Error::other("%APPDATA% is not set")),
        }
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        storage::write_json(path, &self.clone().sanitized())
    }

    /// The settings of one device, the defaults when it has none.
    pub fn device(&self, key: &str) -> DeviceSettings {
        self.devices.get(key).cloned().unwrap_or_default()
    }

    /// The name to show for a device: its alias, or the detected name.
    pub fn display_name<'a>(&'a self, key: &str, detected: &'a str) -> &'a str {
        match self.devices.get(key) {
            Some(d) if !d.alias.is_empty() => &d.alias,
            _ => detected,
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.interval_secs = self.interval_secs.clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS);
        self.low_threshold = self.low_threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        for d in self.devices.values_mut() {
            d.alias = d.alias.trim().chars().take(MAX_ALIAS_CHARS).collect();
        }
        self.devices.retain(|_, d| !d.is_default());
        self
    }
}
