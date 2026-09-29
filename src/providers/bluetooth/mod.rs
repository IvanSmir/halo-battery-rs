//! Bluetooth devices whose battery Windows itself knows: the level shown in
//! Settings -> Bluetooth & devices (headsets, and LE keyboards, mice and
//! controllers with the Battery Service). Nothing is sent to the devices.
//!
//! Windows keeps the last level of a paired device that is off, so only
//! devices WinRT reports as connected count as read; one that disconnects
//! stays greyed out for a while like any other provider's.
//! - [`mapping`]: nodes to devices, kinds from class and appearance (pure)

pub mod mapping;

use std::time::{Duration, Instant};

use self::mapping::{Grouped, kind_from_appearance, kind_from_class};
use super::Provider;
use super::inventory::Scan;
use super::last_seen::LastSeen;
use crate::device::{DeviceStatus, Kind};
use crate::platform::bluetooth;

/// How long a device that disconnected keeps its (greyed-out) icon.
const ASLEEP_KEEP: Duration = Duration::from_secs(300);

pub struct BluetoothProvider {
    diag: Vec<String>,
    last_seen: LastSeen,
}

impl Default for BluetoothProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothProvider {
    pub fn new() -> Self {
        Self { diag: Vec::new(), last_seen: LastSeen::new(ASLEEP_KEEP) }
    }

    fn read(&mut self, g: &Grouped) -> Option<DeviceStatus> {
        let level = g.level?;
        let link = g.address().and_then(|a| bluetooth::link(a, g.le));
        let name = if g.name.is_empty() { format!("Bluetooth {}", g.mac) } else { g.name.clone() };
        self.diag.push(format!(
            "  {name} [{}{}] battery={level}% connected={:?}",
            g.mac,
            if g.le { " LE" } else { "" },
            link.map(|l| l.connected)
        ));
        if !link.is_some_and(|l| l.connected) {
            return None;
        }
        let kind = link
            .and_then(|l| l.class_of_device.and_then(kind_from_class).or(l.appearance.and_then(kind_from_appearance)))
            .unwrap_or(if g.audio { Kind::Headset } else { Kind::Mouse });
        Some(DeviceStatus {
            key: format!("bluetooth:{}", g.mac),
            name,
            level: Some(level),
            // Windows does not say whether a Bluetooth device is charging
            charging: false,
            online: true,
            kind,
        })
    }
}

impl Provider for BluetoothProvider {
    fn poll(&mut self, _scan: &Scan<'_>) -> Vec<DeviceStatus> {
        self.diag.clear();
        let groups = mapping::group(&bluetooth::devices());
        self.diag.push(format!(
            "[Bluetooth] {} paired, {} with a battery level",
            groups.len(),
            groups.iter().filter(|g| g.level.is_some()).count()
        ));
        let found: Vec<DeviceStatus> = groups.iter().filter_map(|g| self.read(g)).collect();
        self.last_seen.merge(found, Instant::now(), true)
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diag.clone()
    }
}
