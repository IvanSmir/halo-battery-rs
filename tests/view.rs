mod common;

use common::{charging, device, offline};
use halo_battery::app::view::{Look, PLACEHOLDER_KEY, icon_guid, icon_state, icons, is_animated, tooltip};
use halo_battery::config::{Config, DeviceSettings, RingColor};
use halo_battery::icon::palette::ring_color;

const LOOK: Look = Look { low: 20, light_taskbar: false, ring: None, animate: true, pictogram: true, time: 0.75 };

fn with_device(key: &str, settings: DeviceSettings) -> Config {
    let mut cfg = Config::default();
    cfg.devices.insert(key.into(), settings);
    cfg
}

mod tooltips {
    use super::*;

    #[test]
    fn shows_the_level() {
        assert_eq!(tooltip(&device("a", Some(84)), "PRO X 2"), "PRO X 2: 84%");
    }

    #[test]
    fn says_when_charging() {
        assert_eq!(tooltip(&charging(device("a", Some(60))), "x"), "x: 60% (cargando)");
    }

    #[test]
    fn says_when_asleep() {
        assert_eq!(tooltip(&offline(device("a", Some(60))), "x"), "x: dormido (último 60%)");
    }

    #[test]
    fn says_when_the_level_is_unknown() {
        assert_eq!(tooltip(&device("a", None), "x"), "x: nivel desconocido");
    }
}

mod look {
    use super::*;

    #[test]
    fn comes_from_the_settings() {
        let mut cfg = Config { low_threshold: 25, ..Config::default() };
        cfg.appearance.ring_color = RingColor::Rose;
        cfg.appearance.animate = false;
        cfg.appearance.pictogram = false;
        let look = Look::new(&cfg, true, 1.0);
        assert_eq!(look.low, 25);
        assert_eq!(look.ring, ring_color(RingColor::Rose));
        assert!(look.light_taskbar && !look.animate && !look.pictogram);
    }
}

mod icon_states {
    use super::*;

    #[test]
    fn carry_the_device_and_the_look() {
        let st = icon_state(&device("a", Some(84)), LOOK);
        assert_eq!(st.level, Some(84));
        assert_eq!(st.low, LOOK.low);
        assert_eq!(st.pulse, 1.0, "no animation while discharging");
    }

    #[test]
    fn only_online_charging_devices_are_animated() {
        assert!(is_animated(&charging(device("a", Some(60)))));
        assert!(!is_animated(&device("a", Some(60))));
        assert!(!is_animated(&offline(charging(device("a", Some(60))))));
        assert!(icon_state(&charging(device("a", Some(60))), LOOK).pulse < 1.0);
    }

    #[test]
    fn the_animation_can_be_turned_off() {
        let st = icon_state(&charging(device("a", Some(60))), Look { animate: false, ..LOOK });
        assert_eq!(st.pulse, 1.0);
    }
}

mod device_colour {
    use super::*;

    fn tinted(color: Option<RingColor>) -> Config {
        with_device(
            "a",
            DeviceSettings { ring_color: color, visible: true, alias: "x".into(), ..DeviceSettings::default() },
        )
    }

    fn ring_of(cfg: &Config, look: Look) -> Option<[u8; 3]> {
        icons(&[device("a", Some(84))], cfg, look)[0].state.ring
    }

    #[test]
    fn the_device_colour_overrides_the_global_one() {
        let global = Look { ring: ring_color(RingColor::Blue), ..LOOK };
        assert_eq!(ring_of(&tinted(Some(RingColor::Rose)), global), ring_color(RingColor::Rose));
    }

    #[test]
    fn no_device_colour_follows_the_global_one() {
        let global = Look { ring: ring_color(RingColor::Blue), ..LOOK };
        assert_eq!(ring_of(&tinted(None), global), ring_color(RingColor::Blue));
        assert_eq!(ring_of(&Config::default(), global), ring_color(RingColor::Blue));
    }

    #[test]
    fn auto_on_a_device_uses_the_taskbar_colour_even_when_the_global_is_blue() {
        let global = Look { ring: ring_color(RingColor::Blue), ..LOOK };
        assert_eq!(ring_of(&tinted(Some(RingColor::Auto)), global), None);
    }

    #[test]
    fn only_that_device_changes() {
        let cfg = tinted(Some(RingColor::Mint));
        let views = icons(&[device("a", Some(1)), device("b", Some(2))], &cfg, LOOK);
        assert_eq!(views[0].state.ring, ring_color(RingColor::Mint));
        assert_eq!(views[1].state.ring, LOOK.ring);
    }
}

mod icon_list {
    use super::*;

    #[test]
    fn one_icon_per_device_in_order() {
        let views = icons(&[device("a", Some(1)), device("b", Some(2))], &Config::default(), LOOK);
        let keys: Vec<&str> = views.iter().map(|v| v.key.as_str()).collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn hidden_devices_get_no_icon() {
        let cfg = with_device("a", DeviceSettings { visible: false, ..DeviceSettings::default() });
        let views = icons(&[device("a", Some(1)), device("b", Some(2))], &cfg, LOOK);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].key, "b");
    }

    #[test]
    fn the_tooltip_uses_the_alias() {
        let cfg = with_device("a", DeviceSettings { alias: "Mi ratón".into(), ..DeviceSettings::default() });
        assert_eq!(icons(&[device("a", Some(84))], &cfg, LOOK)[0].tooltip, "Mi ratón: 84%");
    }

    #[test]
    fn a_placeholder_when_there_are_no_devices() {
        let views = icons(&[], &Config::default(), LOOK);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].key, PLACEHOLDER_KEY);
        assert!(!views[0].state.online);
    }

    #[test]
    fn a_placeholder_when_every_device_is_hidden() {
        let cfg = with_device("a", DeviceSettings { visible: false, ..DeviceSettings::default() });
        assert_eq!(icons(&[device("a", Some(1))], &cfg, LOOK)[0].key, PLACEHOLDER_KEY);
    }
}

mod tray_identity {
    use super::*;

    #[test]
    fn the_guid_is_deterministic() {
        assert_eq!(icon_guid("logitech:D988095B"), icon_guid("logitech:D988095B"));
    }

    #[test]
    fn different_devices_get_different_guids() {
        assert_ne!(icon_guid("logitech:D988095B"), icon_guid("logitech:D988095C"));
        assert_ne!(icon_guid("a"), icon_guid("b"));
    }

    #[test]
    fn the_placeholder_has_a_guid_of_its_own() {
        assert_ne!(icon_guid(PLACEHOLDER_KEY), icon_guid("a"));
        assert_ne!(icon_guid(PLACEHOLDER_KEY), 0);
    }

    /// Windows pins an icon by its GUID: if the algorithm ever changes, every
    /// user's pinned icons silently unpin, so the exact values are fixed here.
    #[test]
    fn the_guid_values_never_change() {
        assert_eq!(icon_guid(""), 0x6c62272e07bb014262b821756295c58d);
        assert_eq!(icon_guid("logitech:D988095B"), 0x8cf31091f5492563cf28ca7df45c6975);
    }
}
