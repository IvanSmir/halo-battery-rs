//! The devices the tray read last, published in
//! %APPDATA%\halo-battery-rs\devices.json for the settings window, which
//! never talks to the hardware itself.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::device::DeviceStatus;
use crate::storage;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub devices: Vec<DeviceStatus>,
}

impl Snapshot {
    pub fn default_path() -> Option<PathBuf> {
        storage::data_dir().map(|d| d.join("devices.json"))
    }

    /// The snapshot in `path`; empty when missing or unreadable.
    pub fn load_from(path: &Path) -> Self {
        storage::read_json(path).unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        storage::write_json(path, self)
    }
}
