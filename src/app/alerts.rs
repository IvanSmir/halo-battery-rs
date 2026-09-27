//! Decides when to warn about a low battery: once per discharge cycle. Pure;
//! showing the notification is up to the caller.

use std::collections::HashSet;

use crate::device::DeviceStatus;

/// An alert is re-armed once the level climbs this far above the threshold,
/// so a level hovering around it does not warn again and again.
const HYSTERESIS: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alert {
    pub name: String,
    pub level: u8,
}

#[derive(Default)]
pub struct AlertTracker {
    /// Devices already warned about in the current discharge cycle.
    fired: HashSet<String>,
}

impl AlertTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// The alerts to show for the latest readings.
    pub fn update(&mut self, devices: &[DeviceStatus], low: u8) -> Vec<Alert> {
        let mut out = Vec::new();
        for d in devices {
            let Some(level) = d.level else { continue };
            if d.charging || level > low.saturating_add(HYSTERESIS) {
                self.fired.remove(&d.key);
            } else if d.online && level <= low && self.fired.insert(d.key.clone()) {
                out.push(Alert { name: d.name.clone(), level });
            }
        }
        out
    }

    /// Forgets which devices were warned about, e.g. after the threshold changed.
    pub fn reset(&mut self) {
        self.fired.clear();
    }
}
