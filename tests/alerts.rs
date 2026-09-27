mod common;

use common::{charging, device, offline};
use halo_battery::app::alerts::{Alert, AlertTracker};

const LOW: u8 = 20;

fn alert(key: &str, level: u8) -> Alert {
    Alert { name: format!("{key} name"), level }
}

#[test]
fn warns_when_a_device_reaches_the_threshold() {
    let mut t = AlertTracker::new();
    assert_eq!(t.update(&[device("a", Some(20))], LOW), vec![alert("a", 20)]);
}

#[test]
fn does_not_warn_above_the_threshold() {
    let mut t = AlertTracker::new();
    assert!(t.update(&[device("a", Some(21))], LOW).is_empty());
}

#[test]
fn warns_once_per_discharge() {
    let mut t = AlertTracker::new();
    t.update(&[device("a", Some(20))], LOW);
    assert!(t.update(&[device("a", Some(15))], LOW).is_empty());
    assert!(t.update(&[device("a", Some(5))], LOW).is_empty());
}

#[test]
fn charging_rearms_the_alert() {
    let mut t = AlertTracker::new();
    t.update(&[device("a", Some(20))], LOW);
    assert!(t.update(&[charging(device("a", Some(20)))], LOW).is_empty(), "no alert while charging");
    assert_eq!(t.update(&[device("a", Some(20))], LOW), vec![alert("a", 20)]);
}

#[test]
fn a_level_hovering_near_the_threshold_does_not_rearm() {
    let mut t = AlertTracker::new();
    t.update(&[device("a", Some(20))], LOW);
    assert!(t.update(&[device("a", Some(24))], LOW).is_empty());
    assert!(t.update(&[device("a", Some(19))], LOW).is_empty());
    // climbing past threshold + hysteresis re-arms it
    t.update(&[device("a", Some(26))], LOW);
    assert_eq!(t.update(&[device("a", Some(19))], LOW), vec![alert("a", 19)]);
}

#[test]
fn ignores_offline_devices_and_unknown_levels() {
    let mut t = AlertTracker::new();
    assert!(t.update(&[offline(device("a", Some(5))), device("b", None)], LOW).is_empty());
}

#[test]
fn tracks_each_device_separately() {
    let mut t = AlertTracker::new();
    assert_eq!(t.update(&[device("a", Some(10)), device("b", Some(10))], LOW), vec![alert("a", 10), alert("b", 10)]);
}

#[test]
fn reset_forgets_past_alerts() {
    let mut t = AlertTracker::new();
    t.update(&[device("a", Some(10))], LOW);
    t.reset();
    assert_eq!(t.update(&[device("a", Some(10))], LOW), vec![alert("a", 10)]);
}
