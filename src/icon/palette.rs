//! Colours of the icon, following the system battery icon.

pub type Rgb = [u8; 3];

pub const RED: Rgb = [232, 17, 35];
pub const AMBER: Rgb = [255, 185, 0];
pub const GREEN: Rgb = [16, 196, 80];
pub const WHITE: Rgb = [255, 255, 255];
pub const BLACK: Rgb = [0, 0, 0];

/// Below this the threshold is not lowered any further for the red arc.
const MIN_THRESHOLD: u8 = 10;
/// Width of the amber band above the threshold, in percentage points.
const AMBER_BAND: u8 = 10;

/// Pictogram and track colour: the taskbar's own foreground.
pub fn foreground(light_taskbar: bool) -> Rgb {
    if light_taskbar { BLACK } else { WHITE }
}

/// Colour of the charge arc: green while charging, red at or below the low
/// threshold, amber just above it, otherwise the foreground colour.
pub fn arc_color(level: Option<u8>, charging: bool, low: u8, fg: Rgb) -> Rgb {
    if charging {
        return GREEN;
    }
    let thr = low.max(MIN_THRESHOLD);
    match level {
        Some(l) if l <= thr => RED,
        Some(l) if l <= thr + AMBER_BAND => AMBER,
        _ => fg,
    }
}
