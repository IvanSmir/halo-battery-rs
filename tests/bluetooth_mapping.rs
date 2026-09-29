use halo_battery::device::{DeviceStatus, Kind};
use halo_battery::platform::bluetooth::{BluetoothDevice, Link};
use halo_battery::providers::bluetooth::mapping::{
    Grouped, display_name, group, is_root, kind_from_appearance, kind_from_class, mac_of, status_of,
};

// Instance ids as Windows writes them (classic from a real machine, LE in the
// shape of an Xbox-compatible controller connected over Bluetooth LE).
const CLASSIC_ROOT: &str = r"BTHENUM\DEV_5C9BA67E10EF\8&2913C227&0&BLUETOOTHDEVICE_5C9BA67E10EF";
const CLASSIC_A2DP: &str =
    r"BTHENUM\{0000110A-0000-1000-8000-00805F9B34FB}_VID&0001004C_PID&4A43\8&2913C227&0&5C9BA67E10EF_C00000000";
const CLASSIC_HANDS_FREE: &str =
    r"BTHENUM\{0000111E-0000-1000-8000-00805F9B34FB}_VID&0001004C_PID&4A43\8&2913C227&0&5C9BA67E10EF_C00000000";
const LOCAL_SERIAL: &str =
    r"BTHENUM\{00001101-0000-1000-8000-00805F9B34FB}_LOCALMFG&0000\8&DB7880A&0&000000000000_00000000";
const LE_ROOT: &str = r"BTHLE\DEV_98B6E9016688\7&1A2B3C4D&0&98B6E9016688";
const LE_BATTERY_SERVICE: &str =
    r"BTHLEDEVICE\{0000180F-0000-1000-8000-00805F9B34FB}_DEV_VID&02045E_PID&0B13_REV&0515_98B6E9016688\8&3C1D&0&0022";

fn node(name: &str, id: &str, battery: Option<u8>) -> BluetoothDevice {
    BluetoothDevice { name: name.into(), instance_id: id.into(), battery }
}

mod addresses {
    use super::*;

    #[test]
    fn root_nodes_name_the_address() {
        assert_eq!(mac_of(CLASSIC_ROOT).as_deref(), Some("5C9BA67E10EF"));
        assert_eq!(mac_of(LE_ROOT).as_deref(), Some("98B6E9016688"));
    }

    #[test]
    fn service_nodes_carry_it_too() {
        assert_eq!(mac_of(CLASSIC_A2DP).as_deref(), Some("5C9BA67E10EF"));
        assert_eq!(mac_of(LE_BATTERY_SERVICE).as_deref(), Some("98B6E9016688"));
    }

    #[test]
    fn local_services_have_no_device_address() {
        assert_eq!(mac_of(LOCAL_SERIAL), None);
    }

    #[test]
    fn only_device_nodes_are_roots() {
        assert!(is_root(CLASSIC_ROOT) && is_root(LE_ROOT));
        assert!(!is_root(CLASSIC_A2DP) && !is_root(LE_BATTERY_SERVICE));
    }
}

mod grouping {
    use super::*;

    #[test]
    fn the_nodes_of_one_device_become_one_entry() {
        let groups = group(&[
            node("Auriculares A2DP SNK", CLASSIC_A2DP, None),
            node("Auriculares", CLASSIC_ROOT, None),
            node("Auriculares Hands-Free HF", CLASSIC_HANDS_FREE, Some(70)),
        ]);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.name, "Auriculares", "the root node names the device");
        assert_eq!(g.level, Some(70), "the level can sit on any node");
        assert!(g.audio && !g.le);
    }

    #[test]
    fn an_le_device_takes_its_level_from_the_battery_service() {
        let groups = group(&[node("Xbox Wireless Controller", LE_ROOT, None), node("", LE_BATTERY_SERVICE, Some(55))]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].level, Some(55));
        assert!(groups[0].le);
        assert_eq!(groups[0].address(), Some(0x98B6_E901_6688));
    }

    #[test]
    fn separate_devices_stay_separate_and_local_services_are_dropped() {
        let groups =
            group(&[node("a", CLASSIC_ROOT, None), node("b", LE_ROOT, None), node("serial", LOCAL_SERIAL, None)]);
        let macs: Vec<&str> = groups.iter().map(|g| g.mac.as_str()).collect();
        assert_eq!(macs, ["5C9BA67E10EF", "98B6E9016688"]);
    }
}

mod kinds {
    use super::*;

    #[test]
    fn from_the_class_of_device() {
        assert_eq!(kind_from_class(0x24_0404), Some(Kind::Headset), "wearable headset");
        assert_eq!(kind_from_class(0x00_2540), Some(Kind::Keyboard));
        assert_eq!(kind_from_class(0x00_2580), Some(Kind::Mouse));
        assert_eq!(kind_from_class(0x00_2508), Some(Kind::Gamepad));
        assert_eq!(kind_from_class(0x7A_020C), None, "a phone");
    }

    #[test]
    fn from_the_le_appearance() {
        assert_eq!(kind_from_appearance(0x03C1), Some(Kind::Keyboard));
        assert_eq!(kind_from_appearance(0x03C2), Some(Kind::Mouse));
        assert_eq!(kind_from_appearance(0x03C4), Some(Kind::Gamepad));
        assert_eq!(kind_from_appearance(0x0941), Some(Kind::Headset));
        assert_eq!(kind_from_appearance(0x0040), None, "a phone");
    }
}

mod what_shows {
    use super::*;

    fn device(level: Option<u8>, audio: bool) -> Grouped {
        Grouped { mac: "98B6E9016688".into(), le: true, name: "RK H81".into(), level, audio }
    }

    fn connected() -> Link {
        Link { connected: true, class_of_device: None, appearance: None }
    }

    #[test]
    fn a_connected_device_with_a_level_shows() {
        let st: DeviceStatus = status_of(&device(Some(64), false), Some(connected())).unwrap();
        assert_eq!(st.key, "bluetooth:98B6E9016688");
        assert_eq!(st.name, "RK H81");
        assert_eq!(st.level, Some(64));
        assert!(st.online && !st.charging);
    }

    #[test]
    fn a_device_that_is_off_keeps_a_level_in_windows_but_does_not_show() {
        let off = Link { connected: false, ..connected() };
        assert_eq!(status_of(&device(Some(64), false), Some(off)), None);
    }

    #[test]
    fn an_unknown_link_or_level_does_not_show() {
        assert_eq!(status_of(&device(Some(64), false), None), None);
        assert_eq!(status_of(&device(None, false), Some(connected())), None);
    }

    #[test]
    fn the_class_of_device_decides_the_kind_first() {
        let link = Link { class_of_device: Some(0x00_2540), appearance: Some(0x03C2), ..connected() };
        assert_eq!(status_of(&device(Some(1), true), Some(link)).unwrap().kind, Kind::Keyboard);
    }

    #[test]
    fn then_the_le_appearance() {
        let link = Link { appearance: Some(0x03C4), ..connected() };
        assert_eq!(status_of(&device(Some(1), false), Some(link)).unwrap().kind, Kind::Gamepad);
    }

    #[test]
    fn then_an_audio_profile_means_a_headset() {
        assert_eq!(status_of(&device(Some(1), true), Some(connected())).unwrap().kind, Kind::Headset);
        assert_eq!(status_of(&device(Some(1), false), Some(connected())).unwrap().kind, Kind::Mouse);
    }

    #[test]
    fn a_nameless_device_is_named_after_its_address() {
        let nameless = Grouped { name: String::new(), ..device(Some(1), false) };
        assert_eq!(display_name(&nameless), "Bluetooth 98B6E9016688");
    }
}
