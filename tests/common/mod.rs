//! Helpers shared by the integration tests.

#![allow(dead_code)] // each test crate uses a different subset

use halo_battery::device::{DeviceStatus, Kind};

/// An online, discharging mouse named after its key.
pub fn device(key: &str, level: Option<u8>) -> DeviceStatus {
    DeviceStatus {
        key: key.into(),
        name: format!("{key} name"),
        level,
        charging: false,
        online: true,
        kind: Kind::Mouse,
    }
}

pub fn charging(mut d: DeviceStatus) -> DeviceStatus {
    d.charging = true;
    d
}

pub fn offline(mut d: DeviceStatus) -> DeviceStatus {
    d.online = false;
    d
}
