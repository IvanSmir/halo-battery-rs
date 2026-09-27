use std::fs;

use halo_battery::config::Config;
use tempfile::TempDir;

fn path_in(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("sub").join("config.json")
}

#[test]
fn round_trips_through_a_file() {
    let dir = TempDir::new().unwrap();
    let path = path_in(&dir);
    let cfg = Config { interval_secs: 120, low_threshold: 25 };
    cfg.save_to(&path).unwrap();
    assert_eq!(Config::load_from(&path), cfg);
}

#[test]
fn a_missing_file_gives_the_defaults() {
    let dir = TempDir::new().unwrap();
    assert_eq!(Config::load_from(&path_in(&dir)), Config::default());
}

#[test]
fn a_corrupt_file_gives_the_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, "{ not json").unwrap();
    assert_eq!(Config::load_from(&path), Config::default());
}

#[test]
fn missing_fields_take_their_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{ "low_threshold": 30 }"#).unwrap();
    assert_eq!(Config::load_from(&path), Config { low_threshold: 30, ..Config::default() });
}

#[test]
fn out_of_range_values_are_clamped() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{ "interval_secs": 0, "low_threshold": 99 }"#).unwrap();
    let cfg = Config::load_from(&path);
    assert!(cfg.interval_secs >= 5, "a zero interval would poll in a tight loop");
    assert!(cfg.low_threshold <= 50);
}
