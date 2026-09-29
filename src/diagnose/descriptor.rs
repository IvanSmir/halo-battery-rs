//! Just enough of the HID report descriptor format to tell which usage pages
//! a device declares, and whether any of them is about its battery. Pure.
//!
//! A descriptor is a list of items: one prefix byte (`tag << 4 | type << 2 |
//! size`, size 3 meaning 4 bytes) followed by its data. The Usage Page is the
//! global item with tag 0; a Usage is the local item with tag 0 and, when it
//! is 4 bytes long, carries its own page in the high half.

use std::collections::BTreeSet;

const TYPE_GLOBAL: u8 = 1;
const TYPE_LOCAL: u8 = 2;
const TAG_USAGE_PAGE: u8 = 0;
const TAG_USAGE: u8 = 0;
/// Long items (prefix 0xFE) carry their own size and are skipped.
const LONG_ITEM: u8 = 0xFE;

/// Generic Device Controls page, whose usage 0x20 is Battery Strength.
pub const PAGE_GENERIC_DEVICE: u16 = 0x06;
pub const USAGE_BATTERY_STRENGTH: u16 = 0x20;
/// Power Device and Battery System pages (UPS-style battery reporting).
pub const PAGE_POWER: u16 = 0x84;
pub const PAGE_BATTERY_SYSTEM: u16 = 0x85;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Every usage page the descriptor declares, in order.
    pub pages: BTreeSet<u16>,
    /// It declares Generic Device Controls / Battery Strength.
    pub battery_strength: bool,
}

impl Summary {
    /// Whether anything in it is about a battery.
    pub fn mentions_battery(&self) -> bool {
        self.battery_strength || self.pages.contains(&PAGE_POWER) || self.pages.contains(&PAGE_BATTERY_SYSTEM)
    }
}

pub fn summarize(desc: &[u8]) -> Summary {
    let mut out = Summary::default();
    let mut page: u16 = 0;
    let mut i = 0;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == LONG_ITEM {
            let len = desc.get(i + 1).copied().unwrap_or(0) as usize;
            i += 3 + len;
            continue;
        }
        let size = match prefix & 0b11 {
            3 => 4,
            n => n as usize,
        };
        let kind = (prefix >> 2) & 0b11;
        let tag = prefix >> 4;
        let Some(data) = desc.get(i + 1..i + 1 + size) else { break };
        let value = data.iter().rev().fold(0u32, |acc, &b| (acc << 8) | b as u32);
        match (kind, tag) {
            (TYPE_GLOBAL, TAG_USAGE_PAGE) => {
                page = value as u16;
                out.pages.insert(page);
            }
            (TYPE_LOCAL, TAG_USAGE) => {
                // a 4-byte usage names its page in the high 16 bits
                let (usage_page, usage) =
                    if size == 4 { ((value >> 16) as u16, value as u16) } else { (page, value as u16) };
                if size == 4 {
                    out.pages.insert(usage_page);
                }
                if usage_page == PAGE_GENERIC_DEVICE && usage == USAGE_BATTERY_STRENGTH {
                    out.battery_strength = true;
                }
            }
            _ => {}
        }
        i += 1 + size;
    }
    out
}

/// A human name for the usage pages that matter here.
pub fn page_name(page: u16) -> &'static str {
    match page {
        0x01 => "Generic Desktop",
        0x06 => "Generic Device Controls",
        0x07 => "Keyboard",
        0x08 => "LEDs",
        0x09 => "Button",
        0x0C => "Consumer",
        0x0D => "Digitizer",
        0x84 => "Power Device",
        0x85 => "Battery System",
        0xFF00..=0xFFFF => "vendor-defined",
        _ => "",
    }
}
