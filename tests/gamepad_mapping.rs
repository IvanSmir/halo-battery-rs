use halo_battery::providers::gamepad::mapping::{display_name, level_from_capacity};

#[test]
fn generic_windows_names_are_replaced_by_the_vendor() {
    assert_eq!(display_name("Xbox 360 Controller for Windows", 0x3537), "GameSir controller");
    assert_eq!(display_name("HID-compliant game controller", 0x3537), "GameSir controller");
    assert_eq!(display_name("Xbox Wireless Controller", 0x045E), "Xbox controller");
}

#[test]
fn unknown_vendors_get_a_neutral_name() {
    assert_eq!(display_name("Xbox Controller", 0x1234), "Gamepad");
    assert_eq!(display_name("", 0x1234), "Gamepad");
}

#[test]
fn meaningful_names_are_kept_trimmed() {
    assert_eq!(display_name("  GameSir G7 Pro ", 0x3537), "GameSir G7 Pro");
}

#[test]
fn level_is_the_rounded_share_of_the_full_capacity() {
    assert_eq!(level_from_capacity(500, 1000), Some(50));
    assert_eq!(level_from_capacity(1000, 1000), Some(100));
    assert_eq!(level_from_capacity(333, 1000), Some(33));
}

#[test]
fn level_is_clamped_and_unknown_without_a_full_capacity() {
    assert_eq!(level_from_capacity(1200, 1000), Some(100));
    assert_eq!(level_from_capacity(-5, 1000), Some(0));
    assert_eq!(level_from_capacity(10, 0), None);
}
