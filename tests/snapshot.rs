mod common;

use common::{charging, device};
use halo_battery::snapshot::Snapshot;
use tempfile::TempDir;

#[test]
fn round_trips_through_a_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("devices.json");
    let snap = Snapshot { devices: vec![device("a", Some(84)), charging(device("b", None))] };
    snap.save_to(&path).unwrap();
    assert_eq!(Snapshot::load_from(&path), snap);
}

#[test]
fn a_missing_file_is_an_empty_snapshot() {
    let dir = TempDir::new().unwrap();
    assert_eq!(Snapshot::load_from(&dir.path().join("devices.json")), Snapshot::default());
}

#[test]
fn kinds_are_written_in_lower_case() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("devices.json");
    Snapshot { devices: vec![device("a", Some(1))] }.save_to(&path).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains(r#""kind": "mouse""#));
}
