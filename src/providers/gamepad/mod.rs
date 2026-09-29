//! Xbox-compatible controllers through Windows.Gaming.Input, the API the Xbox
//! Accessories app uses. Many third-party Xbox-protocol controllers, e.g. the
//! GameSir G7 Pro on its 2.4 GHz receiver, never report their battery through
//! XInput, but Windows.Gaming.Input returns a real BatteryReport: remaining and
//! full charge capacity plus the charging status.
//!
//! The controller list fills asynchronously after the process starts, so the
//! first poll may see nothing yet.
//!
//! Over Bluetooth its battery report is not usable (an Xbox Wireless
//! Controller read 10% at 82%), so controllers connected over Bluetooth are
//! left to the Bluetooth provider, which reads the level Windows shows.
//! - [`mapping`]: raw values to the app's model (pure)

pub mod mapping;

use std::collections::HashSet;

use hidapi::{BusType, HidApi};
use windows::Gaming::Input::RawGameController;
use windows::System::Power::BatteryStatus;

use self::mapping::{display_name, level_from_capacity};
use super::Provider;
use crate::device::{DeviceStatus, Kind};

pub struct GamepadProvider {
    diag: Vec<String>,
}

impl Default for GamepadProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadProvider {
    pub fn new() -> Self {
        // the first touch of the API starts filling the controller list in the background
        let _ = RawGameController::RawGameControllers();
        Self { diag: Vec::new() }
    }

    fn read_controller(
        &mut self,
        n: usize,
        c: &RawGameController,
        bluetooth: &HashSet<(u16, u16)>,
    ) -> windows::core::Result<Option<DeviceStatus>> {
        let vid = c.HardwareVendorId()?;
        let pid = c.HardwareProductId()?;
        let raw_name = c.DisplayName()?.to_string();
        if bluetooth.contains(&(vid, pid)) {
            self.diag.push(format!("[WGI] '{raw_name}' {vid:04x}:{pid:04x}: over Bluetooth, left to that provider"));
            return Ok(None);
        }
        let Ok(report) = c.TryGetBatteryReport() else {
            self.diag.push(format!("[WGI] '{raw_name}' {vid:04x}:{pid:04x}: no battery report"));
            return Ok(None);
        };
        let status = report.Status()?;
        let remaining = report.RemainingCapacityInMilliwattHours().and_then(|v| v.Value()).ok();
        let full = report.FullChargeCapacityInMilliwattHours().and_then(|v| v.Value()).ok();
        let level = remaining.zip(full).and_then(|(r, f)| level_from_capacity(r, f));
        self.diag.push(format!(
            "[WGI] '{raw_name}' {vid:04x}:{pid:04x} status={status:?} remain={remaining:?} full={full:?} -> {level:?}%"
        ));
        if status == BatteryStatus::NotPresent || level.is_none() {
            return Ok(None);
        }
        Ok(Some(DeviceStatus {
            key: format!("gamepad:{vid:04x}:{pid:04x}:{n}"),
            name: display_name(&raw_name, vid),
            level,
            charging: status == BatteryStatus::Charging,
            online: true,
            kind: Kind::Gamepad,
        }))
    }

    fn read_all(&mut self) -> windows::core::Result<Vec<DeviceStatus>> {
        let bluetooth = bluetooth_hid_ids();
        let mut out = Vec::new();
        for (n, c) in RawGameController::RawGameControllers()?.into_iter().enumerate() {
            match self.read_controller(n, &c, &bluetooth) {
                Ok(st) => out.extend(st),
                Err(e) => self.diag.push(format!("[WGI] controller {n}: {e}")),
            }
        }
        if self.diag.is_empty() {
            self.diag.push("[WGI] no controllers listed".into());
        }
        Ok(out)
    }
}

/// Vendor and product ids of the HID devices connected over Bluetooth.
fn bluetooth_hid_ids() -> HashSet<(u16, u16)> {
    let Ok(api) = HidApi::new() else { return HashSet::new() };
    api.device_list()
        .filter(|d| matches!(d.bus_type(), BusType::Bluetooth))
        .map(|d| (d.vendor_id(), d.product_id()))
        .collect()
}

impl Provider for GamepadProvider {
    fn poll(&mut self) -> Vec<DeviceStatus> {
        self.diag.clear();
        self.read_all().unwrap_or_else(|e| {
            self.diag.push(format!("[WGI] unavailable: {e}"));
            Vec::new()
        })
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diag.clone()
    }
}
