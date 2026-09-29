//! What the tray shows for the devices: which icons, with which state and
//! tooltip, following the user's settings. Pure.

use crate::config::Config;
use crate::device::{DeviceStatus, Kind};
use crate::icon::{self, IconState, palette};

/// One "breath" of the charging animation, in seconds.
const BREATH_PERIOD: f32 = 3.0;
/// Key of the placeholder shown while no device is visible.
pub const PLACEHOLDER_KEY: &str = "__none__";

const FNV_OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
const FNV_PRIME: u128 = 0x0000000001000000000000000000013b;

/// The stable identity Windows pins a tray icon by, derived from the device
/// key: 128-bit FNV-1a, written by hand because the standard hashers may
/// change between Rust releases, and a different value would silently unpin
/// the icon. Never change this algorithm; a test fixes its results.
pub fn icon_guid(key: &str) -> u128 {
    key.bytes().fold(FNV_OFFSET, |hash, byte| (hash ^ u128::from(byte)).wrapping_mul(FNV_PRIME))
}

/// Everything one tray icon needs.
#[derive(Clone, Debug, PartialEq)]
pub struct IconView {
    pub key: String,
    pub state: IconState,
    pub tooltip: String,
}

/// Display settings shared by all icons.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub low: u8,
    pub light_taskbar: bool,
    /// Arc colour when the level is fine; `None` uses the taskbar colour.
    pub ring: Option<palette::Rgb>,
    pub animate: bool,
    pub pictogram: bool,
    /// Seconds since the app started, for the charging animation.
    pub time: f32,
}

impl Look {
    pub fn new(cfg: &Config, light_taskbar: bool, time: f32) -> Self {
        Self {
            low: cfg.low_threshold,
            light_taskbar,
            ring: palette::ring_color(cfg.appearance.ring_color),
            animate: cfg.appearance.animate,
            pictogram: cfg.appearance.pictogram,
            time,
        }
    }
}

/// The tooltip of a device shown as `name`.
pub fn tooltip(d: &DeviceStatus, name: &str) -> String {
    match (d.level, d.online, d.charging) {
        (None, _, _) => format!("{name}: nivel desconocido"),
        (Some(l), false, _) => format!("{name}: dormido (último {l}%)"),
        (Some(l), true, true) => format!("{name}: {l}% (cargando)"),
        (Some(l), true, false) => format!("{name}: {l}%"),
    }
}

pub fn icon_state(d: &DeviceStatus, look: Look) -> IconState {
    let pulse =
        if look.animate && is_animated(d) { icon::breath_level((look.time / BREATH_PERIOD).fract()) } else { 1.0 };
    IconState {
        level: d.level,
        charging: d.charging,
        online: d.online,
        kind: d.kind,
        low: look.low,
        light_taskbar: look.light_taskbar,
        ring: look.ring,
        pictogram: look.pictogram,
        pulse,
    }
}

/// Whether the icon of `d` changes from frame to frame.
pub fn is_animated(d: &DeviceStatus) -> bool {
    d.charging && d.online
}

/// The icons to show: one per device the user has not hidden, under its
/// chosen name, or a single placeholder when none is left, so the menu (and
/// Exit) stays reachable.
pub fn icons(devices: &[DeviceStatus], cfg: &Config, look: Look) -> Vec<IconView> {
    let views: Vec<IconView> = devices
        .iter()
        .filter(|d| cfg.device(&d.key).visible)
        .map(|d| IconView {
            key: d.key.clone(),
            state: icon_state(d, look),
            tooltip: tooltip(d, cfg.display_name(&d.key, &d.name)),
        })
        .collect();
    if !views.is_empty() {
        return views;
    }
    let placeholder = DeviceStatus {
        key: PLACEHOLDER_KEY.into(),
        name: "Halo Battery".into(),
        level: None,
        charging: false,
        online: false,
        kind: Kind::Mouse,
    };
    vec![IconView {
        key: placeholder.key.clone(),
        state: icon_state(&placeholder, look),
        tooltip: "Halo Battery: ningún dispositivo visible".into(),
    }]
}
