use halo_battery::update::github::{GitHubReleases, parse_response};
use halo_battery::update::source::{Release, UpdateError};
use halo_battery::update::state::{UpdateState, record};
use halo_battery::update::version::Version;
use halo_battery::update::{CHECK_EVERY, RETRY_AFTER, next_check};
use tempfile::TempDir;

fn v(s: &str) -> Version {
    Version::parse(s).unwrap_or_else(|| panic!("{s} should parse"))
}

fn release(version: &str) -> Release {
    Release { version: v(version), url: format!("https://github.com/IvanSmir/halo-battery-rs/releases/tag/v{version}") }
}

mod versions {
    use super::*;

    #[test]
    fn parse_with_or_without_a_leading_v() {
        assert_eq!(v("v0.2.0"), v("0.2.0"));
        assert_eq!(v("1.10.3").to_string(), "1.10.3");
    }

    #[test]
    fn parse_pre_releases_and_ignore_build_metadata() {
        assert_eq!(v("1.0.0-beta.2").pre.as_deref(), Some("beta.2"));
        assert_eq!(v("1.0.0+build.5"), v("1.0.0"));
    }

    #[test]
    fn reject_what_is_not_a_version() {
        for bad in ["", "v", "1.2", "1.2.3.4", "one.two.three", "1.2.3-"] {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn compare_numerically_not_as_text() {
        assert!(v("0.10.0") > v("0.9.2"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("0.2.1") > v("0.2.0"));
    }

    #[test]
    fn a_pre_release_comes_before_its_release() {
        assert!(v("1.0.0-beta.1") < v("1.0.0"));
        assert!(v("1.0.0-beta.2") > v("1.0.0-beta.1"));
        assert!(v("1.0.0-rc.1") > v("0.9.0"));
    }
}

mod github {
    use super::*;

    const LATEST: &str = r#"{
        "tag_name": "v0.2.0",
        "html_url": "https://github.com/IvanSmir/halo-battery-rs/releases/tag/v0.2.0",
        "draft": false,
        "prerelease": false
    }"#;

    #[test]
    fn asks_the_latest_release_of_the_repository() {
        assert_eq!(
            GitHubReleases::new("IvanSmir", "halo-battery-rs").api_url(),
            "https://api.github.com/repos/IvanSmir/halo-battery-rs/releases/latest"
        );
    }

    #[test]
    fn reads_the_version_and_the_release_page() {
        assert_eq!(parse_response(200, LATEST), Ok(release("0.2.0")));
    }

    #[test]
    fn maps_http_errors() {
        assert_eq!(parse_response(404, ""), Err(UpdateError::NotFound), "no release, or a private repository");
        assert_eq!(parse_response(403, ""), Err(UpdateError::RateLimited));
        assert_eq!(parse_response(429, ""), Err(UpdateError::RateLimited));
        assert!(matches!(parse_response(500, ""), Err(UpdateError::BadResponse(_))));
    }

    #[test]
    fn rejects_answers_it_cannot_trust() {
        assert!(matches!(parse_response(200, "{ not json"), Err(UpdateError::BadResponse(_))));
        let bad_tag = LATEST.replace("v0.2.0\",\n        \"html", "latest\",\n        \"html");
        assert!(matches!(parse_response(200, &bad_tag), Err(UpdateError::BadResponse(_))));
        let elsewhere = LATEST.replace("https://github.com/", "https://example.com/");
        assert!(matches!(parse_response(200, &elsewhere), Err(UpdateError::BadResponse(_))), "only github.com pages");
    }
}

mod announcing {
    use super::*;

    #[test]
    fn a_newer_release_is_announced_once() {
        let mut st = UpdateState::default();
        assert!(record(&mut st, &v("0.1.0"), &release("0.2.0")));
        assert!(!record(&mut st, &v("0.1.0"), &release("0.2.0")), "already announced");
        assert_eq!(st.notified.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn each_new_version_is_announced() {
        let mut st = UpdateState::default();
        record(&mut st, &v("0.1.0"), &release("0.2.0"));
        assert!(record(&mut st, &v("0.1.0"), &release("0.3.0")));
    }

    #[test]
    fn the_running_version_or_an_older_one_is_not_announced() {
        let mut st = UpdateState::default();
        assert!(!record(&mut st, &v("0.2.0"), &release("0.2.0")));
        assert!(!record(&mut st, &v("0.3.0"), &release("0.2.0")));
        assert_eq!(st.notified, None);
    }

    #[test]
    fn available_only_when_newer_than_the_running_version() {
        let mut st = UpdateState::default();
        record(&mut st, &v("0.1.0"), &release("0.2.0"));
        assert_eq!(st.available(&v("0.1.0")).map(|r| r.version.as_str()), Some("0.2.0"));
        assert_eq!(st.available(&v("0.2.0")), None, "after updating, the banner goes away");
    }

    #[test]
    fn the_state_round_trips_through_its_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("update.json");
        let mut st = UpdateState::default();
        record(&mut st, &v("0.1.0"), &release("0.2.0"));
        st.save_to(&path).unwrap();
        assert_eq!(UpdateState::load_from(&path), st);
        assert_eq!(UpdateState::load_from(&dir.path().join("missing.json")), UpdateState::default());
    }
}

mod scheduling {
    use super::*;

    #[test]
    fn checks_daily_after_an_answer_and_sooner_after_a_failure() {
        assert_eq!(next_check(&Ok(release("0.2.0"))), CHECK_EVERY);
        assert_eq!(next_check(&Err(UpdateError::NotFound)), CHECK_EVERY, "no release yet is an answer");
        assert_eq!(next_check(&Err(UpdateError::RateLimited)), RETRY_AFTER);
        assert_eq!(next_check(&Err(UpdateError::Unreachable("offline".into()))), RETRY_AFTER);
        assert!(RETRY_AFTER < CHECK_EVERY);
    }
}
