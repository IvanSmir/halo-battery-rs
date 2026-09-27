//! Logitech wireless mice and keyboards over HID++ 2.0 (Lightspeed / Unifying
//! receivers, or the device itself on the cable). Works alongside G HUB.
//!
//! Every request is a read: nothing changes any setting on the device.
//! - [`protocol`]: message layout and payload decoding (pure)
//! - [`channel`]: request/response I/O over hidapi

pub mod channel;
pub mod protocol;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::time::{Duration, Instant};

use hidapi::HidApi;

use self::channel::{Answer, Channel, Identity, TIMEOUT};
use self::protocol::{BATTERY_FEATURES, Reply};
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
    /// (pid, idx) -> identity, read once per slot.
    ids: HashMap<(u16, u8), Identity>,
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
            ids: HashMap::new(),
            asleep: HashSet::new(),
            last_seen: LastSeen::new(ASLEEP_KEEP),
        }
    }

    /// Whether a device answers in `slot`; keeps track of slots gone silent.
    fn ping(&mut self, ch: &Channel, slot: (u16, u8)) -> bool {
        // A paired device that is asleep does not answer at all, which would cost
        // the full ping timeout on every poll. Once a slot has gone silent, ping it
        // with the short timeout until it answers again.
        let timeout = if self.asleep.contains(&slot) { TIMEOUT } else { PING_TIMEOUT };
        match ch.request(slot.1, protocol::ROOT_INDEX, protocol::FN_ROOT_PING, &[], timeout) {
            Answer::Reply(Reply::Ok(_)) => {
                self.asleep.remove(&slot);
                true
            }
            // empty slot
            Answer::Reply(Reply::Error) => {
                self.asleep.remove(&slot);
                false
            }
            Answer::Timeout => {
                self.asleep.insert(slot);
                let name = self.ids.get(&slot).map_or("paired device", |i| i.name.as_str());
                self.diag.push(format!("  idx={} '{name}': no answer (asleep or off)", slot.1));
                false
            }
        }
    }

    fn read_slot(&mut self, ch: &Channel, pid: u16, idx: u8) -> Option<DeviceStatus> {
        let slot = (pid, idx);
        if !self.ping(ch, slot) {
            return None;
        }
        let id = self.ids.entry(slot).or_insert_with(|| ch.identity(idx)).clone();
        let name = if id.name.is_empty() { "Logitech device".to_string() } else { id.name };
        for feature in BATTERY_FEATURES {
            let fi = ch.feature_index(idx, feature);
            if fi == 0 {
                continue;
            }
            let Some(r) = ch.ask(idx, fi, protocol::battery_function(feature), &[]) else { continue };
            let (level, charging) = protocol::parse_battery(feature, &r);
            self.diag.push(format!(
                "  idx={idx} '{name}' unit={} feature {feature:04x}: {} -> {level:?}%{}",
                if id.unit.is_empty() { "?" } else { &id.unit },
                hexdump(&r, 4),
                if charging { " (charging)" } else { "" }
            ));
            if level.is_some() {
                // the unit id is stable across receiver and cable and tells identical
                // devices apart; without one, fall back to the receiver slot
                let key = if id.unit.is_empty() {
                    format!("logitech:{pid:04x}:{idx}")
                } else {
                    format!("logitech:{}", id.unit)
                };
                return Some(DeviceStatus { key, name, level, charging, online: true, kind: id.kind });
            }
        }
        self.diag.push(format!("  idx={idx} '{name}': no battery feature answered"));
        None
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
