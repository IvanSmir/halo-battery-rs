use std::fs;

use halo_battery::config::{Config, DeviceSettings, RingColor};
use tempfile::TempDir;

fn path_in(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("sub").join("config.json")
}

fn renamed(alias: &str) -> DeviceSettings {
    DeviceSettings { alias: alias.into(), ..DeviceSettings::default() }
}

mod file {
    use super::*;

    #[test]
    fn round_trips_every_section() {
        let dir = TempDir::new().unwrap();
        let path = path_in(&dir);
        let mut cfg = Config { interval_secs: 120, low_threshold: 25, ..Config::default() };
        cfg.notifications.full_charge = true;
        cfg.appearance.ring_color = RingColor::Mint;
        cfg.devices.insert("logitech:D988095B".into(), renamed("Mi ratón"));
        cfg.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path), cfg);
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let dir = TempDir::new().unwrap();
        assert_eq!(Config::load_from(&path_in(&dir)), Config::default());
    }

    #[test]
    fn checking_a_missing_file_is_not_an_error() {
        let dir = TempDir::new().unwrap();
        assert_eq!(Config::try_load_from(&path_in(&dir)).unwrap(), Config::default());
    }

    #[test]
    fn checking_a_corrupt_file_reports_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "{ not json").unwrap();
        let err = Config::try_load_from(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn checking_a_valid_file_sanitizes_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{ "interval_secs": 1 }"#).unwrap();
        assert_eq!(Config::try_load_from(&path).unwrap().interval_secs, 5);
    }

    #[test]
    fn a_corrupt_file_gives_the_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Config::load_from(&path), Config::default());
    }

    #[test]
    fn a_file_from_an_older_version_still_loads() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{ "interval_secs": 120, "low_threshold": 30 }"#).unwrap();
        let cfg = Config::load_from(&path);
        assert_eq!((cfg.interval_secs, cfg.low_threshold), (120, 30));
        assert_eq!(cfg.appearance, Config::default().appearance);
        assert!(cfg.updates.check, "update checks default to on");
    }

    #[test]
    fn saving_leaves_no_temporary_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save_to(&path).unwrap();
        let names: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["config.json"]);
    }
}

mod sanitizing {
    use super::*;

    #[test]
    fn out_of_range_values_are_clamped() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{ "interval_secs": 0, "low_threshold": 99 }"#).unwrap();
        let cfg = Config::load_from(&path);
        assert!(cfg.interval_secs >= 5, "a zero interval would poll in a tight loop");
        assert!(cfg.low_threshold <= 50);
    }

    #[test]
    fn aliases_are_trimmed_and_shortened() {
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), renamed("  Ratón  "));
        cfg.devices.insert("b".into(), renamed(&"x".repeat(100)));
        let cfg = cfg.sanitized();
        assert_eq!(cfg.devices["a"].alias, "Ratón");
        assert_eq!(cfg.devices["b"].alias.chars().count(), 40);
    }

    #[test]
    fn devices_left_at_their_defaults_are_not_kept() {
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), DeviceSettings::default());
        cfg.devices.insert("b".into(), renamed("   "));
        assert!(cfg.sanitized().devices.is_empty());
    }
}

mod per_device {
    use super::*;

    #[test]
    fn unknown_devices_get_the_defaults() {
        let d = Config::default().device("nope");
        assert!(d.visible && d.notify && d.alias.is_empty());
    }

    #[test]
    fn the_alias_replaces_the_detected_name() {
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), renamed("Mi ratón"));
        assert_eq!(cfg.display_name("a", "PRO X 2"), "Mi ratón");
        assert_eq!(cfg.display_name("b", "GameSir controller"), "GameSir controller");
    }
}

mod device_ring_color {
    use super::*;

    fn tinted(color: RingColor) -> DeviceSettings {
        DeviceSettings { ring_color: Some(color), ..DeviceSettings::default() }
    }

    #[test]
    fn follows_the_global_colour_by_default() {
        assert_eq!(DeviceSettings::default().ring_color, None);
    }

    #[test]
    fn round_trips() {
        let dir = TempDir::new().unwrap();
        let path = path_in(&dir);
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), tinted(RingColor::Rose));
        cfg.devices.insert("b".into(), tinted(RingColor::Auto));
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded, cfg);
        assert_eq!(loaded.devices["b"].ring_color, Some(RingColor::Auto), "Auto is a choice, not 'unset'");
    }

    #[test]
    fn is_not_written_when_unset() {
        let dir = TempDir::new().unwrap();
        let path = path_in(&dir);
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), renamed("Mi ratón"));
        cfg.save_to(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(json["devices"]["a"].get("ring_color").is_none(), "an unset colour is left out, not written as null");
    }

    #[test]
    fn an_old_file_without_the_field_loads() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{ "devices": { "a": { "alias": "Mi ratón", "visible": false, "notify": true } } }"#)
            .unwrap();
        let d = &Config::load_from(&path).devices["a"];
        assert_eq!((d.alias.as_str(), d.visible, d.ring_color), ("Mi ratón", false, None));
    }

    #[test]
    fn a_null_from_the_settings_window_means_unset() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{ "devices": { "a": { "alias": "x", "ring_color": null } } }"#).unwrap();
        assert_eq!(Config::load_from(&path).devices["a"].ring_color, None);
    }

    #[test]
    fn a_device_with_only_a_colour_is_kept() {
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), tinted(RingColor::Blue));
        assert_eq!(cfg.sanitized().devices["a"].ring_color, Some(RingColor::Blue));
    }

    #[test]
    fn untouched_devices_are_still_dropped() {
        let mut cfg = Config::default();
        cfg.devices.insert("a".into(), DeviceSettings { ring_color: None, ..DeviceSettings::default() });
        assert!(cfg.sanitized().devices.is_empty());
    }
}
