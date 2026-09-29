//! [`UpdateSource`] backed by the GitHub releases API
//! (`GET /repos/{owner}/{repo}/releases/latest`, which skips drafts and
//! pre-releases). Works without authentication on public repositories, at up
//! to 60 requests per hour per IP address.

use serde::Deserialize;

use super::source::{Release, UpdateError, UpdateSource};
use super::version::Version;
use crate::platform::http;

/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = concat!("halo-battery/", env!("CARGO_PKG_VERSION"));

pub struct GitHubReleases {
    owner: String,
    repo: String,
}

impl GitHubReleases {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self { owner: owner.into(), repo: repo.into() }
    }

    pub fn api_url(&self) -> String {
        format!("https://api.github.com/repos/{}/{}/releases/latest", self.owner, self.repo)
    }
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
}

/// Turns the API's answer into a release. Pure.
pub fn parse_response(status: u16, body: &str) -> Result<Release, UpdateError> {
    match status {
        200 => {}
        404 => return Err(UpdateError::NotFound),
        403 | 429 => return Err(UpdateError::RateLimited),
        other => return Err(UpdateError::BadResponse(format!("HTTP {other}"))),
    }
    let latest: LatestRelease =
        serde_json::from_str(body).map_err(|e| UpdateError::BadResponse(format!("invalid JSON: {e}")))?;
    let version = Version::parse(&latest.tag_name)
        .ok_or_else(|| UpdateError::BadResponse(format!("tag {:?} is not a version", latest.tag_name)))?;
    if !latest.html_url.starts_with("https://github.com/") {
        return Err(UpdateError::BadResponse(format!("unexpected release page {:?}", latest.html_url)));
    }
    Ok(Release { version, url: latest.html_url })
}

impl UpdateSource for GitHubReleases {
    fn latest(&self) -> Result<Release, UpdateError> {
        let headers = [("User-Agent", USER_AGENT), ("Accept", "application/vnd.github+json")];
        let response = http::get(&self.api_url(), &headers).map_err(UpdateError::Unreachable)?;
        parse_response(response.status, &response.body)
    }

    fn describe(&self) -> String {
        format!("GitHub {}/{}", self.owner, self.repo)
    }
}
