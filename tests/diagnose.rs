use halo_battery::diagnose::descriptor::{PAGE_BATTERY_SYSTEM, summarize};
use halo_battery::diagnose::report::{HidInterface, Report};
use halo_battery::platform::bluetooth::BluetoothDevice;

/// The vendor channel of a Redragon H510 PRO dongle (040B:0897), as captured.
const VENDOR: &[u8] = &[
    0x06, 0x00, 0xf1, 0x09, 0x00, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x01, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95,
    0x08, 0x81, 0x02, 0x09, 0x02, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x40, 0x91, 0x02, 0xc0,
];

/// A boot keyboard: Generic Desktop, Keyboard and LEDs pages.
const KEYBOARD: &[u8] = &[
    0x05, 0x01, 0x09, 0x06, 0xa1, 0x01, 0x05, 0x07, 0x19, 0xe0, 0x29, 0xe7, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95,
    0x08, 0x81, 0x02, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0xc0,
];

/// Generic Device Controls / Battery Strength, the standard battery usage.
const BATTERY_STRENGTH: &[u8] =
    &[0x05, 0x06, 0x09, 0x20, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02];

mod descriptors {
    use super::*;

    #[test]
    fn a_vendor_channel_declares_only_its_page() {
        let s = summarize(VENDOR);
        assert_eq!(s.pages.into_iter().collect::<Vec<_>>(), [0xF100]);
    }

    #[test]
    fn a_keyboard_declares_its_pages_and_no_battery() {
        let s = summarize(KEYBOARD);
        assert_eq!(s.pages.iter().copied().collect::<Vec<_>>(), [0x01, 0x07, 0x08]);
        assert!(!s.mentions_battery());
    }

    #[test]
    fn battery_strength_is_detected() {
        let s = summarize(BATTERY_STRENGTH);
        assert!(s.battery_strength);
        assert!(s.mentions_battery());
    }

    #[test]
    fn a_four_byte_usage_carries_its_own_page() {
        // Usage (0x0006_0020): page 6, usage 0x20, while the current page is Generic Desktop
        let s = summarize(&[0x05, 0x01, 0x0b, 0x20, 0x00, 0x06, 0x00]);
        assert!(s.battery_strength);
        assert!(s.pages.contains(&0x06));
    }

    #[test]
    fn the_battery_system_page_counts_as_battery() {
        let s = summarize(&[0x05, 0x85, 0x09, 0x66]);
        assert!(s.pages.contains(&PAGE_BATTERY_SYSTEM));
        assert!(s.mentions_battery());
    }

    #[test]
    fn long_items_are_skipped() {
        // long item: FE <size=2> <tag> <2 data bytes>, then Usage Page 0x0C
        let s = summarize(&[0xfe, 0x02, 0x10, 0xaa, 0xbb, 0x05, 0x0c]);
        assert_eq!(s.pages.into_iter().collect::<Vec<_>>(), [0x0C]);
    }

    #[test]
    fn a_truncated_descriptor_does_not_panic() {
        let s = summarize(&[0x06, 0x00]);
        assert!(s.pages.is_empty());
        assert!(summarize(&[]).pages.is_empty());
    }
}

mod report {
    use super::*;

    fn interface(pid: u16, descriptor: Result<Vec<u8>, String>) -> HidInterface {
        HidInterface {
            vendor_id: 0x258A,
            product_id: pid,
            manufacturer: "RK".into(),
            product: "H81".into(),
            bus: "Usb".into(),
            interface: 1,
            usage_page: 0xFF00,
            usage: 1,
            descriptor,
        }
    }

    fn report(hid: Vec<HidInterface>, bluetooth: Vec<BluetoothDevice>) -> String {
        Report {
            app_version: "0.1.0".into(),
            windows: "Windows".into(),
            date: "hoy".into(),
            hid,
            bluetooth,
            ..Report::default()
        }
        .to_string()
    }

    #[test]
    fn says_it_only_read() {
        assert!(report(vec![], vec![]).contains("reading only"));
    }

    #[test]
    fn devices_declaring_a_battery_are_listed_first() {
        let text = report(vec![interface(0x0049, Ok(BATTERY_STRENGTH.to_vec()))], vec![]);
        let hints = text.split("== HID devices ==").next().unwrap();
        assert!(hints.contains("258A:0049 RK H81"));
        assert!(text.contains(">> declares a battery"));
    }

    #[test]
    fn interfaces_are_grouped_by_device() {
        let text =
            report(vec![interface(0x0049, Ok(KEYBOARD.to_vec())), interface(0x0049, Ok(VENDOR.to_vec()))], vec![]);
        assert_eq!(text.matches("[258A:0049]").count(), 1);
        assert_eq!(text.matches("  - bus Usb").count(), 2);
    }

    #[test]
    fn an_unreadable_descriptor_says_why() {
        let text = report(vec![interface(0x0049, Err("access denied".into()))], vec![]);
        assert!(text.contains("descriptor: unavailable (access denied)"));
    }

    #[test]
    fn descriptors_are_printed_in_hex() {
        let text = report(vec![interface(0x0049, Ok(VENDOR.to_vec()))], vec![]);
        assert!(text.contains("descriptor (36 bytes):"));
        assert!(text.contains("06 00 f1 09 00 a1 01 85"));
    }

    #[test]
    fn bluetooth_shows_devices_not_every_profile() {
        let bt = |name: &str, id: &str, battery| BluetoothDevice { name: name.into(), instance_id: id.into(), battery };
        let text = report(
            vec![],
            vec![
                bt("Auriculares", r"BTHENUM\DEV_001122334455\8&1", None),
                bt("Auriculares A2DP", r"BTHENUM\{0000110A-0000-1000-8000-00805F9B34FB}_X\8&2", None),
                bt("Teclado", r"BTHLE\SOMETHING\8&3", Some(80)),
            ],
        );
        assert!(text.contains("Auriculares · no known battery"));
        assert!(!text.contains("A2DP"), "profiles are left out");
        assert!(text.contains("Teclado · battery 80%"));
    }
}
