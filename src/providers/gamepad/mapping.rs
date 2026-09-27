//! Turns the raw values Windows.Gaming.Input reports into the app's model.
//! Pure functions, no I/O.

/// Generic names Windows gives most Xbox-protocol controllers; a better one is
/// derived from the hardware vendor id instead.
const GENERIC_NAMES: [&str; 6] = [
    "",
    "xbox controller",
    "xbox one controller",
    "controller (xbox one for windows)",
    "xbox wireless controller",
    "xbox 360 controller for windows",
];

const VID_GAMESIR: u16 = 0x3537;
const VID_MICROSOFT: u16 = 0x045E;

fn vendor_name(vid: u16) -> Option<&'static str> {
    match vid {
        VID_GAMESIR => Some("GameSir controller"),
        VID_MICROSOFT => Some("Xbox controller"),
        _ => None,
    }
}

/// The name shown for a controller: its own when it has a meaningful one,
/// otherwise one derived from the vendor id.
pub fn display_name(raw: &str, vid: u16) -> String {
    let name = raw.trim();
    let lower = name.to_lowercase();
    // "HID-compliant game controller" and its translations
    if lower.contains("hid") || GENERIC_NAMES.contains(&lower.as_str()) {
        return vendor_name(vid).unwrap_or("Gamepad").to_string();
    }
    name.to_string()
}

/// Battery percentage from the remaining and full charge capacity (mWh);
/// `None` when the full capacity is unknown.
pub fn level_from_capacity(remaining: i32, full: i32) -> Option<u8> {
    (full > 0).then(|| (remaining as f32 * 100.0 / full as f32).round().clamp(0.0, 100.0) as u8)
}
