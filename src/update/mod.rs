//! "An update is available": the tray checks for a newer release shortly
//! after it starts and then once a day, notifies once per new version, and
//! leaves what it found in update.json for the settings window. It only
//! points to the download page; nothing is installed.
//! - [`version`]: versions and their order (pure)
//! - [`source`]: where releases are looked up, as a trait
//! - [`github`]: the GitHub releases source
//! - [`state`]: what was found and announced, and the decisions on it (pure)

pub mod github;
pub mod source;
pub mod state;
pub mod version;

use std::time::Duration;

use self::github::GitHubReleases;
use self::source::{Release, UpdateError, UpdateSource};
use self::version::Version;

/// The first check waits a little, so it does not slow down Windows' start-up.
pub const FIRST_CHECK: Duration = Duration::from_secs(30);
pub const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed check (offline, rate limited) try again sooner.
pub const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);

/// Where releases are looked up. Changing the source is changing this line.
pub fn default_source() -> Box<dyn UpdateSource> {
    Box::new(GitHubReleases::new("IvanSmir", "halo-battery-rs"))
}

/// The running version.
pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate version is a valid version")
}

/// How long to wait before the next check after this one. Pure.
pub fn next_check(result: &Result<Release, UpdateError>) -> Duration {
    match result {
        Ok(_) | Err(UpdateError::NotFound) => CHECK_EVERY,
        Err(_) => RETRY_AFTER,
    }
}
