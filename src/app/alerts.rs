//! Decides when to notify: a low battery once per discharge, a full battery
//! once per charge. Pure; showing the notification is up to the caller.

use std::collections::HashSet;

use crate::config::Config;
use crate::device::DeviceStatus;

/// An alert is re-armed once the level climbs this far above the threshold,
/// so a level hovering around it does not warn again and again.
const HYSTERESIS: u8 = 5;
const FULL: u8 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Low,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alert {
    pub kind: AlertKind,
    /// The device's display name (its alias when it has one).
    pub name: String,
    pub level: u8,
}

#[derive(Default)]
pub struct AlertTracker {
    /// Devices already warned about in the current discharge cycle.
    low: HashSet<String>,
    /// Devices already reported full in the current charge.
    full: HashSet<String>,
}

impl AlertTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// The alerts to show for the latest readings. Devices are tracked even
    /// while notifications are off, so turning them back on does not replay
    /// alerts for a state that was already reached.
    pub fn update(&mut self, devices: &[DeviceStatus], cfg: &Config) -> Vec<Alert> {
        let low = cfg.low_threshold;
        let mut out = Vec::new();
        for d in devices {
            let Some(level) = d.level else { continue };
            let wanted = cfg.notifications.enabled && cfg.device(&d.key).notify;
            let mut push = |kind| {
                if wanted {
                    out.push(Alert { kind, name: cfg.display_name(&d.key, &d.name).to_string(), level });
                }
            };

            if d.charging || level > low.saturating_add(HYSTERESIS) {
                self.low.remove(&d.key);
            } else if d.online && level <= low && self.low.insert(d.key.clone()) {
                push(AlertKind::Low);
            }

            if !d.charging {
                self.full.remove(&d.key);
            } else if level >= FULL && self.full.insert(d.key.clone()) && cfg.notifications.full_charge {
                push(AlertKind::Full);
            }
        }
        out
    }

    /// Forgets past low-battery alerts, e.g. after the threshold changed.
    pub fn reset(&mut self) {
        self.low.clear();
    }
}
