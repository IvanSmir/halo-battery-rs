//! Xbox-compatible controllers through Windows.Gaming.Input, the API the Xbox
//! Accessories app uses. Many third-party Xbox-protocol controllers, e.g. the
//! GameSir G7 Pro on its 2.4 GHz receiver, never report their battery through
//! XInput, but Windows.Gaming.Input returns a real BatteryReport: remaining and
//! full charge capacity plus the charging status.
//!
//! The controller list fills asynchronously after the process starts, so the
//! first poll may see nothing yet.

use windows::Gaming::Input::RawGameController;
use windows::System::Power::BatteryStatus;

use crate::device::{DeviceStatus, Kind, Provider};

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

fn vendor_name(vid: u16) -> Option<&'static str> {
    match vid {
        0x3537 => Some("GameSir controller"),
        0x045E => Some("Xbox controller"),
        _ => None,
    }
}

fn display_name(raw: &str, vid: u16) -> String {
    let lower = raw.trim().to_lowercase();
    // "HID-compliant game controller" and its translations
    if lower.contains("hid") || GENERIC_NAMES.contains(&lower.as_str()) {
        return vendor_name(vid).unwrap_or("Gamepad").to_string();
    }
    raw.trim().to_string()
}

fn level(remaining: i32, full: i32) -> Option<u8> {
    (full > 0).then(|| (remaining as f32 * 100.0 / full as f32).round().clamp(0.0, 100.0) as u8)
}

pub struct GamepadProvider {
    diag: Vec<String>,
}

impl GamepadProvider {
    pub fn new() -> Self {
        // the first touch of the API starts filling the controller list in the background
        let _ = RawGameController::RawGameControllers();
        Self { diag: Vec::new() }
    }

    fn read(&mut self) -> windows::core::Result<Vec<DeviceStatus>> {
        let mut out = Vec::new();
        let controllers = RawGameController::RawGameControllers()?;
        for (n, c) in controllers.into_iter().enumerate() {
            let vid = c.HardwareVendorId()?;
            let pid = c.HardwareProductId()?;
            let raw_name = c.DisplayName()?.to_string();
            let Ok(report) = c.TryGetBatteryReport() else {
                self.diag.push(format!("[WGI] '{raw_name}' {vid:04x}:{pid:04x}: no battery report"));
                continue;
            };
            let status = report.Status()?;
            let remaining = report.RemainingCapacityInMilliwattHours().and_then(|v| v.Value()).ok();
            let full = report.FullChargeCapacityInMilliwattHours().and_then(|v| v.Value()).ok();
            let lvl = match (remaining, full) {
                (Some(r), Some(f)) => level(r, f),
                _ => None,
            };
            self.diag.push(format!(
                "[WGI] '{raw_name}' {vid:04x}:{pid:04x} status={status:?} remain={remaining:?} full={full:?} -> {lvl:?}%"
            ));
            if status == BatteryStatus::NotPresent || lvl.is_none() {
                continue;
            }
            out.push(DeviceStatus {
                key: format!("gamepad:{vid:04x}:{pid:04x}:{n}"),
                name: display_name(&raw_name, vid),
                level: lvl,
                charging: status == BatteryStatus::Charging,
                online: true,
                kind: Kind::Gamepad,
            });
        }
        if out.is_empty() && self.diag.is_empty() {
            self.diag.push("[WGI] no controllers listed".into());
        }
        Ok(out)
    }
}

impl Provider for GamepadProvider {
    fn poll(&mut self) -> Vec<DeviceStatus> {
        self.diag.clear();
        self.read().unwrap_or_else(|e| {
            self.diag.push(format!("[WGI] unavailable: {e}"));
            Vec::new()
        })
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diag.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_generic_controllers_by_vendor() {
        assert_eq!(display_name("HID-compliant game controller", 0x3537), "GameSir controller");
        assert_eq!(display_name("Xbox Controller", 0x1234), "Gamepad");
        assert_eq!(display_name("Xbox 360 Controller for Windows", 0x3537), "GameSir controller");
        assert_eq!(display_name(" GameSir G7 Pro ", 0x3537), "GameSir G7 Pro");
    }

    #[test]
    fn computes_level_from_capacity() {
        assert_eq!(level(500, 1000), Some(50));
        assert_eq!(level(1200, 1000), Some(100));
        assert_eq!(level(10, 0), None);
    }
}
