//! Types shared by the battery providers and the tray.

/// What kind of device it is; picks the pictogram in the middle of the icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Headset has no provider yet
pub enum Kind {
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStatus {
    /// Stable identifier: one tray icon per key.
    pub key: String,
    pub name: String,
    /// 0..=100, `None` when the level is unknown.
    pub level: Option<u8>,
    pub charging: bool,
    /// `false` when the receiver is present but the device is asleep or off.
    pub online: bool,
    pub kind: Kind,
}

pub trait Provider {
    /// Current status of every device this provider can see.
    fn poll(&mut self) -> Vec<DeviceStatus>;

    /// Human-readable lines describing the last poll, for `--list`.
    fn diagnostics(&self) -> Vec<String> {
        Vec::new()
    }
}

pub fn hexdump(data: &[u8], limit: usize) -> String {
    data.iter()
        .take(limit)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
