//! Where the latest release is looked up. The rest of the app only knows
//! this trait; GitHub is one implementation, and another place (a website
//! serving a small JSON file, say) only needs another one.

use super::version::Version;

/// The newest release a source knows of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// The page to download it from.
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// No answer: offline, DNS, TLS.
    Unreachable(String),
    /// The source asks to come back later (e.g. an API rate limit).
    RateLimited,
    /// The source has no release (yet).
    NotFound,
    /// An answer that could not be understood.
    BadResponse(String),
}

pub trait UpdateSource: Send {
    /// The newest published release.
    fn latest(&self) -> Result<Release, UpdateError>;

    /// What it is, for diagnostics ("GitHub IvanSmir/halo-battery-rs").
    fn describe(&self) -> String;
}
