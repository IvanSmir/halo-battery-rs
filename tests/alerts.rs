mod common;

use common::{charging, device, offline};
use halo_battery::app::alerts::{Alert, AlertKind, AlertTracker};
use halo_battery::config::{Config, DeviceSettings};

fn cfg() -> Config {
    Config { low_threshold: 20, ..Config::default() }
}

fn low(key: &str, level: u8) -> Alert {
    Alert { kind: AlertKind::Low, name: format!("{key} name"), level }
}

mod low_battery {
    use super::*;

    #[test]
    fn warns_when_a_device_reaches_the_threshold() {
        let mut t = AlertTracker::new();
        assert_eq!(t.update(&[device("a", Some(20))], &cfg()), vec![low("a", 20)]);
    }

    #[test]
    fn does_not_warn_above_the_threshold() {
        let mut t = AlertTracker::new();
        assert!(t.update(&[device("a", Some(21))], &cfg()).is_empty());
    }

    #[test]
    fn warns_once_per_discharge() {
        let mut t = AlertTracker::new();
        t.update(&[device("a", Some(20))], &cfg());
        assert!(t.update(&[device("a", Some(15))], &cfg()).is_empty());
        assert!(t.update(&[device("a", Some(5))], &cfg()).is_empty());
    }

    #[test]
    fn charging_rearms_the_alert() {
        let mut t = AlertTracker::new();
        t.update(&[device("a", Some(20))], &cfg());
        assert!(t.update(&[charging(device("a", Some(20)))], &cfg()).is_empty(), "no alert while charging");
        assert_eq!(t.update(&[device("a", Some(20))], &cfg()), vec![low("a", 20)]);
    }

    #[test]
    fn a_level_hovering_near_the_threshold_does_not_rearm() {
        let mut t = AlertTracker::new();
        t.update(&[device("a", Some(20))], &cfg());
        assert!(t.update(&[device("a", Some(24))], &cfg()).is_empty());
        assert!(t.update(&[device("a", Some(19))], &cfg()).is_empty());
        // climbing past threshold + hysteresis re-arms it
        t.update(&[device("a", Some(26))], &cfg());
        assert_eq!(t.update(&[device("a", Some(19))], &cfg()), vec![low("a", 19)]);
    }

    #[test]
    fn ignores_offline_devices_and_unknown_levels() {
        let mut t = AlertTracker::new();
        assert!(t.update(&[offline(device("a", Some(5))), device("b", None)], &cfg()).is_empty());
    }

    #[test]
    fn tracks_each_device_separately() {
        let mut t = AlertTracker::new();
        let alerts = t.update(&[device("a", Some(10)), device("b", Some(10))], &cfg());
        assert_eq!(alerts, vec![low("a", 10), low("b", 10)]);
    }

    #[test]
    fn reset_forgets_past_alerts() {
        let mut t = AlertTracker::new();
        t.update(&[device("a", Some(10))], &cfg());
        t.reset();
        assert_eq!(t.update(&[device("a", Some(10))], &cfg()), vec![low("a", 10)]);
    }
}

mod full_charge {
    use super::*;

    fn with_full_charge() -> Config {
        let mut c = cfg();
        c.notifications.full_charge = true;
        c
    }

    #[test]
    fn is_off_by_default() {
        let mut t = AlertTracker::new();
        assert!(t.update(&[charging(device("a", Some(100)))], &cfg()).is_empty());
    }

    #[test]
    fn notifies_once_per_charge() {
        let mut t = AlertTracker::new();
        let full = charging(device("a", Some(100)));
        let alerts = t.update(std::slice::from_ref(&full), &with_full_charge());
        assert_eq!(alerts, vec![Alert { kind: AlertKind::Full, name: "a name".into(), level: 100 }]);
        assert!(t.update(&[full], &with_full_charge()).is_empty());
    }

    #[test]
    fn unplugging_rearms_it() {
        let mut t = AlertTracker::new();
        t.update(&[charging(device("a", Some(100)))], &with_full_charge());
        t.update(&[device("a", Some(99))], &with_full_charge());
        assert_eq!(t.update(&[charging(device("a", Some(100)))], &with_full_charge()).len(), 1);
    }
}

mod settings {
    use super::*;

    #[test]
    fn the_master_switch_silences_everything() {
        let mut c = cfg();
        c.notifications.enabled = false;
        assert!(AlertTracker::new().update(&[device("a", Some(5))], &c).is_empty());
    }

    #[test]
    fn a_state_reached_while_silenced_is_not_replayed() {
        let mut t = AlertTracker::new();
        let mut c = cfg();
        c.notifications.enabled = false;
        t.update(&[device("a", Some(5))], &c);
        assert!(t.update(&[device("a", Some(5))], &cfg()).is_empty());
    }

    #[test]
    fn a_device_can_be_silenced_alone() {
        let mut c = cfg();
        c.devices.insert("a".into(), DeviceSettings { notify: false, ..DeviceSettings::default() });
        let alerts = AlertTracker::new().update(&[device("a", Some(5)), device("b", Some(5))], &c);
        assert_eq!(alerts, vec![low("b", 5)]);
    }

    #[test]
    fn alerts_use_the_alias() {
        let mut c = cfg();
        c.devices.insert("a".into(), DeviceSettings { alias: "Mi ratón".into(), ..DeviceSettings::default() });
        let alerts = AlertTracker::new().update(&[device("a", Some(5))], &c);
        assert_eq!(alerts[0].name, "Mi ratón");
    }
}
