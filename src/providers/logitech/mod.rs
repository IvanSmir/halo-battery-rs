//! Logitech wireless mice and keyboards over HID++ 2.0 (Lightspeed / Unifying
//! receivers, or the device itself on the cable). Works alongside G HUB.
//!
//! Every request is a read: nothing changes any setting on the device.
//! - [`protocol`]: message layout and payload decoding (pure)
//! - [`transport`]: the request/response contract and multi-request reads
//! - [`channel`]: the transport over hidapi
//! - [`slot`]: reads one receiver slot, one request once it is known

pub mod channel;
pub mod protocol;
pub mod slot;
pub mod transport;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::time::{Duration, Instant};

use hidapi::HidApi;

use self::channel::Channel;
use self::slot::{SlotCache, SlotRead};
use self::transport::{TIMEOUT, Transport};
use super::last_seen::LastSeen;
use super::{Provider, hexdump};
use crate::device::DeviceStatus;

const LOGITECH_VID: u16 = 0x046D;
/// HID++ vendor page; usage 1 is the short interface, 2 the long one.
const HIDPP_USAGE_PAGE: u16 = 0xFF00;
/// A dozing radio takes up to ~0.5 s to answer the first request.
const PING_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a silent device keeps its (greyed-out) icon.
const ASLEEP_KEEP: Duration = Duration::from_secs(300);
/// Device index of a device on the cable instead of behind a receiver.
const CABLED_INDEX: u8 = 0xFF;

/// One receiver or cabled device, as found in the HID device list.
#[derive(Default)]
struct Endpoint {
    short: Option<CString>,
    long: Option<CString>,
    product: String,
}

impl Endpoint {
    fn is_receiver(&self) -> bool {
        self.product.to_lowercase().contains("receiver")
    }

    /// Device indexes worth pinging.
    fn slots(&self) -> Vec<u8> {
        if self.is_receiver() { (1..=6).collect() } else { vec![CABLED_INDEX] }
    }
}

/// Logitech HID++ endpoints by product id.
fn discover(api: &mut HidApi) -> HashMap<u16, Endpoint> {
    api.reset_devices().ok();
    api.add_devices(LOGITECH_VID, 0).ok();
    let mut found: HashMap<u16, Endpoint> = HashMap::new();
    for d in api.device_list() {
        if d.usage_page() != HIDPP_USAGE_PAGE {
            continue;
        }
        let ep = found.entry(d.product_id()).or_default();
        match d.usage() {
            1 => ep.short = Some(d.path().to_owned()),
            2 => ep.long = Some(d.path().to_owned()),
            _ => continue,
        }
        if ep.product.is_empty() {
            ep.product = d.product_string().unwrap_or_default().to_string();
        }
    }
    found
}

pub struct LogitechProvider {
    api: Option<HidApi>,
    diag: Vec<String>,
    /// What each (pid, idx) slot is known to hold.
    slots: HashMap<(u16, u8), SlotCache>,
    /// Paired slots that stopped answering.
    asleep: HashSet<(u16, u8)>,
    last_seen: LastSeen,
}

impl Default for LogitechProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LogitechProvider {
    pub fn new() -> Self {
        Self {
            api: HidApi::new().ok(),
            diag: Vec::new(),
            slots: HashMap::new(),
            asleep: HashSet::new(),
            last_seen: LastSeen::new(ASLEEP_KEEP),
        }
    }

    fn read_slot(&mut self, t: &impl Transport, pid: u16, idx: u8) -> Option<DeviceStatus> {
        let key = (pid, idx);
        // A paired device that is asleep does not answer at all, which would cost
        // the full ping timeout on every poll. Once a slot has gone silent, use
        // the short timeout until it answers again.
        let timeout = if self.asleep.contains(&key) { TIMEOUT } else { PING_TIMEOUT };
        let mut cache = self.slots.remove(&key);
        let read = slot::read(t, idx, &mut cache, timeout);
        let identity = cache.as_ref().map(|c| c.identity.clone());
        if let Some(c) = cache {
            self.slots.insert(key, c);
        }
        let name = match &identity {
            Some(i) if !i.name.is_empty() => i.name.clone(),
            _ => "Logitech device".to_string(),
        };

        match read {
            SlotRead::Empty => {
                self.asleep.remove(&key);
                None
            }
            SlotRead::Asleep => {
                self.asleep.insert(key);
                self.diag.push(format!("  idx={idx} '{name}': no answer (asleep or off)"));
                None
            }
            SlotRead::NoBattery => {
                self.asleep.remove(&key);
                self.diag.push(format!("  idx={idx} '{name}': no battery feature answered"));
                None
            }
            SlotRead::Battery { level, charging, feature, raw } => {
                self.asleep.remove(&key);
                let id = identity?;
                self.diag.push(format!(
                    "  idx={idx} '{name}' unit={} feature {feature:04x}: {} -> {level}%{}",
                    if id.unit.is_empty() { "?" } else { &id.unit },
                    hexdump(&raw, 4),
                    if charging { " (charging)" } else { "" }
                ));
                // the unit id is stable across receiver and cable and tells identical
                // devices apart; without one, fall back to the receiver slot
                let key = if id.unit.is_empty() {
                    format!("logitech:{pid:04x}:{idx}")
                } else {
                    format!("logitech:{}", id.unit)
                };
                Some(DeviceStatus { key, name, level: Some(level), charging, online: true, kind: id.kind })
            }
        }
    }
}

impl Provider for LogitechProvider {
    fn poll(&mut self) -> Vec<DeviceStatus> {
        self.diag.clear();
        let Some(mut api) = self.api.take() else {
            self.diag.push("[Logitech] hidapi unavailable".into());
            return Vec::new();
        };
        let endpoints = discover(&mut api);

        let mut found: Vec<DeviceStatus> = Vec::new();
        for (pid, ep) in &endpoints {
            let Some(long) = &ep.long else { continue };
            self.diag.push(format!("[Logitech] pid={pid:04x} '{}'", ep.product));
            let ch = match Channel::open(&api, ep.short.as_deref(), long) {
                Ok(ch) => ch,
                Err(e) => {
                    self.diag.push(format!("  open: {e}"));
                    continue;
                }
            };
            for idx in ep.slots() {
                let Some(st) = self.read_slot(&ch, *pid, idx) else { continue };
                // the same device on the cable and through the receiver: charging wins
                match found.iter_mut().find(|d| d.key == st.key) {
                    Some(prev) if st.charging => *prev = st,
                    Some(_) => {}
                    None => found.push(st),
                }
            }
        }
        self.api = Some(api);
        self.last_seen.merge(found, Instant::now(), !endpoints.is_empty())
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diag.clone()
    }
}
