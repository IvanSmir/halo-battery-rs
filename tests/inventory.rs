use std::ffi::CString;

use halo_battery::providers::inventory::{Bus, HidInterface, bluetooth_ids};
use halo_battery::providers::logitech::discovery::{CABLED_INDEX, Endpoint, endpoints};

fn hid(vid: u16, pid: u16, page: u16, usage: u16, bus: Bus, product: &str) -> HidInterface {
    HidInterface {
        vendor_id: vid,
        product_id: pid,
        usage_page: page,
        usage,
        bus,
        product: product.into(),
        path: CString::new(format!("{vid:04x}:{pid:04x}:{page:04x}:{usage}")).unwrap(),
    }
}

/// What a Lightspeed receiver (046D:C54D) exposes: mouse, keyboard and the
/// two HID++ interfaces on the vendor page.
fn lightspeed_receiver() -> Vec<HidInterface> {
    vec![
        hid(0x046D, 0xC54D, 0x0001, 0x0002, Bus::Usb, "USB Receiver"),
        hid(0x046D, 0xC54D, 0x0001, 0x0006, Bus::Usb, "USB Receiver"),
        hid(0x046D, 0xC54D, 0xFF00, 0x0001, Bus::Usb, "USB Receiver"),
        hid(0x046D, 0xC54D, 0xFF00, 0x0002, Bus::Usb, "USB Receiver"),
    ]
}

mod logitech_discovery {
    use super::*;

    #[test]
    fn a_receiver_is_one_endpoint_with_both_interfaces() {
        let found = endpoints(&lightspeed_receiver());
        assert_eq!(found.len(), 1);
        let ep = &found[&0xC54D];
        assert_eq!(ep.short.as_deref(), Some(c"046d:c54d:ff00:1"));
        assert_eq!(ep.long.as_deref(), Some(c"046d:c54d:ff00:2"));
        assert!(ep.is_receiver());
        assert_eq!(ep.slots(), [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_cabled_device_is_read_at_its_own_index() {
        let found = endpoints(&[hid(0x046D, 0xC094, 0xFF00, 0x0002, Bus::Usb, "PRO X 2")]);
        let ep = &found[&0xC094];
        assert!(!ep.is_receiver());
        assert_eq!(ep.slots(), [CABLED_INDEX]);
    }

    #[test]
    fn other_vendors_and_other_pages_are_ignored() {
        let found = endpoints(&[
            hid(0x1532, 0x0099, 0xFF00, 0x0002, Bus::Usb, "not Logitech"),
            hid(0x046D, 0xC232, 0x0001, 0x0006, Bus::Usb, "a keyboard"),
            hid(0x046D, 0xC232, 0xFF00, 0x0003, Bus::Usb, "another vendor usage"),
        ]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn without_the_long_interface_there_is_nothing_to_talk_to() {
        // the provider skips endpoints without a long interface
        let found = endpoints(&[hid(0x046D, 0xC54D, 0xFF00, 0x0001, Bus::Usb, "USB Receiver")]);
        let ep: &Endpoint = &found[&0xC54D];
        assert!(ep.short.is_some());
        assert!(ep.long.is_none());
    }
}

mod bluetooth_devices {
    use super::*;

    #[test]
    fn only_devices_on_the_bluetooth_bus_count() {
        let mut list = lightspeed_receiver();
        list.push(hid(0x045E, 0x0B13, 0x0001, 0x0005, Bus::Bluetooth, "Xbox Wireless Controller"));
        list.push(hid(0x3537, 0x1098, 0x0001, 0x0005, Bus::Usb, "GameSir dongle"));
        let ids = bluetooth_ids(&list);
        assert_eq!(ids.into_iter().collect::<Vec<_>>(), [(0x045E, 0x0B13)]);
    }

    #[test]
    fn nothing_over_bluetooth_gives_an_empty_set() {
        assert!(bluetooth_ids(&lightspeed_receiver()).is_empty());
    }
}
