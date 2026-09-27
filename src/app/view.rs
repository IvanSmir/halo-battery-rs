//! What the tray shows for a device: its icon state and tooltip. Pure.

use crate::device::{DeviceStatus, Kind};
use crate::icon::{self, IconState};

/// One "breath" of the charging animation, in seconds.
const BREATH_PERIOD: f32 = 3.0;
/// Key of the placeholder shown while no device is found.
pub const PLACEHOLDER_KEY: &str = "__none__";

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
    /// Seconds since the app started, for the charging animation.
    pub time: f32,
}

pub fn tooltip(d: &DeviceStatus) -> String {
    match (d.level, d.online, d.charging) {
        (None, _, _) => format!("{}: nivel desconocido", d.name),
        (Some(l), false, _) => format!("{}: dormido (último {l}%)", d.name),
        (Some(l), true, true) => format!("{}: {l}% (cargando)", d.name),
        (Some(l), true, false) => format!("{}: {l}%", d.name),
    }
}

pub fn icon_state(d: &DeviceStatus, look: Look) -> IconState {
    let pulse = if is_animated(d) { icon::breath_level((look.time / BREATH_PERIOD).fract()) } else { 1.0 };
    IconState {
        level: d.level,
        charging: d.charging,
        online: d.online,
        kind: d.kind,
        low: look.low,
        light_taskbar: look.light_taskbar,
        pulse,
    }
}

/// Whether the icon of `d` changes from frame to frame.
pub fn is_animated(d: &DeviceStatus) -> bool {
    d.charging && d.online
}

/// The icons to show for `devices`: one each, or a single placeholder when
/// there are none, so the menu (and Exit) stays reachable.
pub fn icons(devices: &[DeviceStatus], look: Look) -> Vec<IconView> {
    if devices.is_empty() {
        let placeholder = DeviceStatus {
            key: PLACEHOLDER_KEY.into(),
            name: "Halo Battery".into(),
            level: None,
            charging: false,
            online: false,
            kind: Kind::Mouse,
        };
        return vec![IconView {
            key: placeholder.key.clone(),
            state: icon_state(&placeholder, look),
            tooltip: "Halo Battery: ningún dispositivo encontrado".into(),
        }];
    }
    devices.iter().map(|d| IconView { key: d.key.clone(), state: icon_state(d, look), tooltip: tooltip(d) }).collect()
}
