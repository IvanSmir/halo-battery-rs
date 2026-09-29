//! Which HID interfaces are Logitech HID++ endpoints. Pure.
//!
//! A receiver (or a cabled device) exposes HID++ on the vendor page 0xFF00:
//! usage 1 is the short interface, usage 2 the long one.

use std::collections::BTreeMap;
use std::ffi::CString;

use crate::providers::inventory::HidInterface;

pub const LOGITECH_VID: u16 = 0x046D;
pub const HIDPP_USAGE_PAGE: u16 = 0xFF00;
const USAGE_SHORT: u16 = 1;
const USAGE_LONG: u16 = 2;
/// Device index of a device on the cable instead of behind a receiver.
pub const CABLED_INDEX: u8 = 0xFF;
/// Device indexes behind a receiver.
const RECEIVER_SLOTS: std::ops::RangeInclusive<u8> = 1..=6;

/// One receiver or cabled device.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Endpoint {
    pub short: Option<CString>,
    pub long: Option<CString>,
    pub product: String,
}

impl Endpoint {
    pub fn is_receiver(&self) -> bool {
        self.product.to_lowercase().contains("receiver")
    }

    /// Device indexes worth pinging.
    pub fn slots(&self) -> Vec<u8> {
        if self.is_receiver() { RECEIVER_SLOTS.collect() } else { vec![CABLED_INDEX] }
    }
}

/// Logitech HID++ endpoints by product id.
pub fn endpoints(hid: &[HidInterface]) -> BTreeMap<u16, Endpoint> {
    let mut found: BTreeMap<u16, Endpoint> = BTreeMap::new();
    for h in hid.iter().filter(|h| h.vendor_id == LOGITECH_VID && h.usage_page == HIDPP_USAGE_PAGE) {
        if !matches!(h.usage, USAGE_SHORT | USAGE_LONG) {
            continue;
        }
        let ep = found.entry(h.product_id).or_default();
        if h.usage == USAGE_SHORT {
            ep.short = Some(h.path.clone());
        } else {
            ep.long = Some(h.path.clone());
        }
        if ep.product.is_empty() {
            ep.product.clone_from(&h.product);
        }
    }
    found
}
