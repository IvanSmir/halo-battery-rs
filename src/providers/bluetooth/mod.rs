//! Bluetooth devices whose battery Windows itself knows: the level shown in
//! Settings -> Bluetooth & devices (headsets, and LE keyboards, mice and
//! controllers with the Battery Service). Nothing is sent to the devices.
//!
//! Windows keeps the last level of a paired device that is off, so only
//! devices WinRT reports as connected count as read; one that disconnects
//! stays greyed out for a while like any other provider's.
//! - [`mapping`]: nodes to devices, and what each shows as (pure)

pub mod mapping;

use std::time::{Duration, Instant};

use self::mapping::Grouped;
use super::Provider;
use super::inventory::Scan;
use super::last_seen::LastSeen;
use crate::device::DeviceStatus;
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

    /// Asks WinRT about a device with a known level, then leaves the decision
    /// to [`mapping::status_of`].
    fn read(&mut self, g: &Grouped) -> Option<DeviceStatus> {
        let level = g.level?;
        let link = g.address().and_then(|a| bluetooth::link(a, g.le));
        self.diag.push(format!(
            "  {} [{}{}] battery={level}% connected={:?}",
            mapping::display_name(g),
            g.mac,
            if g.le { " LE" } else { "" },
            link.map(|l| l.connected)
        ));
        mapping::status_of(g, link)
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
