//! Battery providers: each one knows a family of devices, how to find them and
//! how to read their battery. Adding a device family means adding a module
//! here and registering it in [`all`].

pub mod bluetooth;
pub mod gamepad;
pub mod inventory;
pub mod last_seen;
pub mod logitech;

use self::inventory::Scan;
use crate::device::DeviceStatus;

pub trait Provider {
    /// Current status of every device this provider can see. `scan` lists the
    /// HID devices present, shared by all providers of one poll.
    fn poll(&mut self, scan: &Scan<'_>) -> Vec<DeviceStatus>;

    /// Human-readable lines describing the last poll, for `--list`.
    fn diagnostics(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Every provider, in the order their icons appear.
pub fn all() -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(logitech::LogitechProvider::new()),
        Box::new(gamepad::GamepadProvider::new()),
        Box::new(bluetooth::BluetoothProvider::new()),
    ]
}

pub(crate) fn hexdump(data: &[u8], limit: usize) -> String {
    data.iter().take(limit).map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}
