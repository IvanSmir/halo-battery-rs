mod common;

use common::{charging, device, offline};
use halo_battery::app::view::{icon_state, icons, is_animated, tooltip, Look, PLACEHOLDER_KEY};

const LOOK: Look = Look { low: 20, light_taskbar: false, time: 0.75 };

mod tooltips {
    use super::*;

    #[test]
    fn shows_the_level() {
        assert_eq!(tooltip(&device("a", Some(84))), "a name: 84%");
    }

    #[test]
    fn says_when_charging() {
        assert_eq!(tooltip(&charging(device("a", Some(60)))), "a name: 60% (cargando)");
    }

    #[test]
    fn says_when_asleep() {
        assert_eq!(tooltip(&offline(device("a", Some(60)))), "a name: dormido (último 60%)");
    }

    #[test]
    fn says_when_the_level_is_unknown() {
        assert_eq!(tooltip(&device("a", None)), "a name: nivel desconocido");
    }
}

mod icon_states {
    use super::*;

    #[test]
    fn carry_the_device_and_the_look() {
        let st = icon_state(&device("a", Some(84)), LOOK);
        assert_eq!(st.level, Some(84));
        assert_eq!(st.low, LOOK.low);
        assert!(!st.light_taskbar);
        assert_eq!(st.pulse, 1.0, "no animation while discharging");
    }

    #[test]
    fn only_online_charging_devices_are_animated() {
        assert!(is_animated(&charging(device("a", Some(60)))));
        assert!(!is_animated(&device("a", Some(60))));
        assert!(!is_animated(&offline(charging(device("a", Some(60))))));
        assert!(icon_state(&charging(device("a", Some(60))), LOOK).pulse < 1.0);
    }
}

mod icon_list {
    use super::*;

    #[test]
    fn one_icon_per_device_in_order() {
        let views = icons(&[device("a", Some(1)), device("b", Some(2))], LOOK);
        let keys: Vec<&str> = views.iter().map(|v| v.key.as_str()).collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn a_placeholder_when_there_are_no_devices() {
        let views = icons(&[], LOOK);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].key, PLACEHOLDER_KEY);
        assert!(!views[0].state.online);
    }
}
