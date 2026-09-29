//! What the update check found, in %APPDATA%\halo-battery-rs\update.json:
//! written by the tray, shown by the settings window. The decisions made on
//! it are pure.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::source::Release;
use super::version::Version;
use crate::storage;

/// A release as stored (the version as text, so the file stays readable).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredRelease {
    pub version: String,
    pub url: String,
}

impl From<&Release> for StoredRelease {
    fn from(r: &Release) -> Self {
        Self { version: r.version.to_string(), url: r.url.clone() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateState {
    /// The newest release found by the last successful check.
    pub latest: Option<StoredRelease>,
    /// The version the user was last notified about, so each one is announced once.
    pub notified: Option<String>,
}

impl UpdateState {
    pub fn default_path() -> Option<PathBuf> {
        storage::data_dir().map(|d| d.join("update.json"))
    }

    pub fn load_from(path: &Path) -> Self {
        storage::read_json(path).unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        storage::write_json(path, self)
    }

    /// The stored release when it is newer than `current`.
    pub fn available(&self, current: &Version) -> Option<&StoredRelease> {
        self.latest.as_ref().filter(|r| Version::parse(&r.version).is_some_and(|v| &v > current))
    }
}

/// Records a release just found; `true` when the user should be notified
/// (it is newer than the running version and was not announced before).
pub fn record(state: &mut UpdateState, current: &Version, found: &Release) -> bool {
    state.latest = Some(StoredRelease::from(found));
    let newer = &found.version > current;
    let version = found.version.to_string();
    let announce = newer && state.notified.as_deref() != Some(version.as_str());
    if announce {
        state.notified = Some(version);
    }
    announce
}
