//! The data model shared by the providers, the icons and the tray.

/// What kind of device it is; picks the pictogram in the middle of the icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
}

/// One device's battery as last read.
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
