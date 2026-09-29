//! The HID devices present, listed once per poll and shared by every
//! provider, so the system is not enumerated once per provider.

use std::collections::HashSet;
use std::ffi::CString;

use hidapi::{BusType, HidApi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    Usb,
    Bluetooth,
    Other,
}

/// One HID interface (a top-level collection) as the system lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HidInterface {
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub bus: Bus,
    pub product: String,
    /// What to open it with.
    pub path: CString,
}

/// What one poll sees: the HID interfaces, and hidapi to open them with.
pub struct Scan<'a> {
    pub hid: Vec<HidInterface>,
    api: Option<&'a HidApi>,
}

impl Scan<'_> {
    /// A scan with no devices and nothing to open them with.
    pub fn empty() -> Scan<'static> {
        Scan { hid: Vec::new(), api: None }
    }

    /// hidapi, to open the interfaces listed in the scan.
    pub fn api(&self) -> Option<&HidApi> {
        self.api
    }
}

/// Vendor and product ids of the HID devices connected over Bluetooth.
pub fn bluetooth_ids(hid: &[HidInterface]) -> HashSet<(u16, u16)> {
    hid.iter().filter(|h| h.bus == Bus::Bluetooth).map(|h| (h.vendor_id, h.product_id)).collect()
}

/// Owns the hidapi context and lists the devices anew for every poll.
pub struct Scanner {
    api: Option<HidApi>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self { api: HidApi::new().ok() }
    }

    pub fn scan(&mut self) -> Scan<'_> {
        let Some(api) = self.api.as_mut() else { return Scan::empty() };
        let _ = api.refresh_devices();
        let hid = api
            .device_list()
            .map(|d| HidInterface {
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                bus: match d.bus_type() {
                    BusType::Usb => Bus::Usb,
                    BusType::Bluetooth => Bus::Bluetooth,
                    _ => Bus::Other,
                },
                product: d.product_string().unwrap_or_default().trim().to_string(),
                path: d.path().to_owned(),
            })
            .collect();
        Scan { hid, api: Some(api) }
    }
}
