mod common;

use std::time::{Duration, Instant};

use common::device;
use halo_battery::providers::last_seen::LastSeen;

const KEEP: Duration = Duration::from_secs(300);

#[test]
fn devices_found_pass_through() {
    let mut ls = LastSeen::new(KEEP);
    let out = ls.merge(vec![device("a", Some(50))], Instant::now(), true);
    assert_eq!(out, vec![device("a", Some(50))]);
}

#[test]
fn a_missing_device_stays_offline_with_its_last_level() {
    let mut ls = LastSeen::new(KEEP);
    let t0 = Instant::now();
    ls.merge(vec![device("a", Some(50))], t0, true);

    let out = ls.merge(vec![], t0 + Duration::from_secs(60), true);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].level, Some(50));
    assert!(!out[0].online);
}

#[test]
fn a_missing_device_is_dropped_after_the_keep_window() {
    let mut ls = LastSeen::new(KEEP);
    let t0 = Instant::now();
    ls.merge(vec![device("a", Some(50))], t0, true);
    assert!(ls.merge(vec![], t0 + KEEP, true).is_empty());
}

#[test]
fn missing_devices_are_dropped_at_once_when_not_kept() {
    let mut ls = LastSeen::new(KEEP);
    let t0 = Instant::now();
    ls.merge(vec![device("a", Some(50))], t0, true);
    assert!(ls.merge(vec![], t0 + Duration::from_secs(1), false).is_empty());
}

#[test]
fn a_fresh_reading_replaces_the_remembered_one() {
    let mut ls = LastSeen::new(KEEP);
    let t0 = Instant::now();
    ls.merge(vec![device("a", Some(50))], t0, true);
    let out = ls.merge(vec![device("a", Some(40)), device("b", Some(90))], t0 + Duration::from_secs(1), true);
    assert_eq!(out, vec![device("a", Some(40)), device("b", Some(90))]);
}
