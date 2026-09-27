//! Logitech wireless mice and keyboards over HID++ 2.0 (Lightspeed / Unifying
//! receivers, or the device itself on the cable). Works alongside G HUB.
//!
//! Protocol (documented by Logitech, implemented in Solaar):
//!   * long request on the receiver's vendor interface ff00:0002:
//!     `11 <device index> <feature index> <function << 4 | swid> <params...>`
//!     device index 1..6 behind a receiver, 0xFF for a device on the cable
//!   * root feature (index 0): fn 0 maps a feature id to its index, fn 1 is a ping
//!   * battery, first one the device supports:
//!     0x1004 unified battery, fn 1: `<percent> <level flags> <charging status>`
//!     0x1000 battery status,  fn 0: `<percent> <next level> <status>`
//!     0x1001 battery voltage, fn 0: `<mV hi> <mV lo> <flags>`
//!   * 0x0005 device name (fn 0 length, fn 1 characters, fn 2 device type)
//!   * 0x0003 device information, fn 0: `<entities> <unit id: 4 bytes> ...`
//!   * an error reply (`10 <idx> 8f ...` / `11 <idx> ff ...`) comes at once for an
//!     empty slot; a paired device that is asleep or switched off does not answer
//!
//! Every request here is a read: nothing changes any setting on the device.

use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

use super::{hexdump, Provider};
use crate::device::{DeviceStatus, Kind};

const LOGITECH_VID: u16 = 0x046D;
const SWID: u8 = 0x0A;
const TIMEOUT: Duration = Duration::from_millis(600);
/// A dozing radio takes up to ~0.5 s to answer the first request.
const PING_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a silent device keeps its (greyed-out) icon.
const ASLEEP_KEEP: Duration = Duration::from_secs(300);

const F_INFO: u16 = 0x0003;
const F_NAME: u16 = 0x0005;
const F_UNIFIED: u16 = 0x1004;
const F_STATUS: u16 = 0x1000;
const F_VOLTAGE: u16 = 0x1001;

/// Li-ion discharge curve used by Solaar, mV -> %.
const VOLTAGE_CURVE: [(u32, u32); 13] = [
    (4186, 100), (4067, 90), (3989, 80), (3922, 70), (3859, 60), (3811, 50), (3778, 40),
    (3751, 30), (3717, 20), (3671, 10), (3646, 5), (3579, 2), (3500, 0),
];

fn voltage_to_percent(mv: u32) -> u8 {
    if mv >= VOLTAGE_CURVE[0].0 {
        return 100;
    }
    for w in VOLTAGE_CURVE.windows(2) {
        let ((hi_mv, hi_p), (lo_mv, lo_p)) = (w[0], w[1]);
        if mv >= lo_mv {
            let p = lo_p as f32 + (mv - lo_mv) as f32 * (hi_p - lo_p) as f32 / (hi_mv - lo_mv) as f32;
            return p.round() as u8;
        }
    }
    0
}

/// Battery level and "on external power" from the params of a battery reply.
/// A full battery still on the charger counts as charging.
fn parse_battery(feature: u16, p: &[u8]) -> (Option<u8>, bool) {
    match feature {
        // 1 charging, 2 slow charging, 3 complete
        F_UNIFIED => ((p[0] <= 100).then_some(p[0]), matches!(p[2], 1..=3)),
        // 1 recharging, 2 almost full, 3 full
        F_STATUS => ((1..=100).contains(&p[0]).then_some(p[0]), matches!(p[2], 1..=3)),
        // bit 7: external power (Solaar's rule)
        F_VOLTAGE => {
            let mv = (p[0] as u32) << 8 | p[1] as u32;
            if mv < 2500 {
                (None, false)
            } else {
                (Some(voltage_to_percent(mv)), p[2] & 0x80 != 0)
            }
        }
        _ => (None, false),
    }
}

fn kind_from_type(t: u8) -> Kind {
    match t {
        0 | 2 => Kind::Keyboard,
        _ => Kind::Mouse,
    }
}

enum Reply {
    Ok(Vec<u8>),
    /// The device answered with an error (an empty receiver slot, a missing feature).
    Error,
    Timeout,
}

/// The receiver's (or cabled device's) HID++ short and long interfaces.
struct Channel {
    long: HidDevice,
    short: Option<HidDevice>,
}

impl Channel {
    fn open(api: &HidApi, short: Option<&std::ffi::CStr>, long: &std::ffi::CStr) -> hidapi::HidResult<Self> {
        let long = api.open_path(long)?;
        long.set_blocking_mode(false)?;
        // errors come back as short reports
        let short = short.and_then(|p| api.open_path(p).ok());
        if let Some(s) = &short {
            s.set_blocking_mode(false).ok();
        }
        Ok(Self { long, short })
    }

    fn request(&self, idx: u8, feat: u8, func: u8, params: &[u8], timeout: Duration) -> Reply {
        let mut req = [0u8; 20];
        req[..4].copy_from_slice(&[0x11, idx, feat, (func << 4) | SWID]);
        req[4..4 + params.len()].copy_from_slice(params);
        if self.long.write(&req).is_err() {
            return Reply::Timeout;
        }
        let end = Instant::now() + timeout;
        let mut buf = [0u8; 64];
        while Instant::now() < end {
            for dev in std::iter::once(&self.long).chain(self.short.as_ref()) {
                let n = match dev.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let r = &buf[..n];
                if r.len() < 4 || r[1] != idx {
                    continue;
                }
                if matches!(r[2], 0x8F | 0xFF) && r[3] == feat {
                    return Reply::Error;
                }
                if r[2] == feat && r[3] == (func << 4) | SWID {
                    let mut p = r[4..].to_vec();
                    p.resize(p.len() + 16, 0);
                    return Reply::Ok(p);
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        Reply::Timeout
    }

    fn ask(&self, idx: u8, feat: u8, func: u8, params: &[u8]) -> Option<Vec<u8>> {
        match self.request(idx, feat, func, params, TIMEOUT) {
            Reply::Ok(p) => Some(p),
            _ => None,
        }
    }

    fn feature_index(&self, idx: u8, feature_id: u16) -> u8 {
        self.ask(idx, 0, 0, &feature_id.to_be_bytes()).map_or(0, |r| r[0])
    }

    /// (name, kind, unit id) of the device at idx; parts it cannot read are empty.
    fn identity(&self, idx: u8) -> Identity {
        let mut id = Identity { name: String::new(), kind: Kind::Mouse, unit: String::new() };
        let fi = self.feature_index(idx, F_NAME);
        if fi != 0 {
            let len = self.ask(idx, fi, 0, &[]).map_or(0, |r| r[0] as usize);
            let mut raw = Vec::new();
            while raw.len() < len {
                match self.ask(idx, fi, 1, &[raw.len() as u8]) {
                    Some(r) => raw.extend_from_slice(&r[..16]),
                    None => break,
                }
            }
            raw.truncate(len);
            id.name = String::from_utf8_lossy(&raw).trim().to_string();
            if let Some(r) = self.ask(idx, fi, 2, &[]) {
                id.kind = kind_from_type(r[0]);
            }
        }
        let fi = self.feature_index(idx, F_INFO);
        if fi != 0 {
            if let Some(r) = self.ask(idx, fi, 0, &[]) {
                if r[1..5].iter().any(|&b| b != 0) {
                    id.unit = r[1..5].iter().map(|b| format!("{b:02X}")).collect();
                }
            }
        }
        id
    }
}

#[derive(Clone)]
struct Identity {
    name: String,
    kind: Kind,
    unit: String,
}

pub struct LogitechProvider {
    api: Option<HidApi>,
    diag: Vec<String>,
    /// (pid, idx) -> identity, read once per slot.
    ids: HashMap<(u16, u8), Identity>,
    /// Paired slots that stopped answering.
    asleep: HashSet<(u16, u8)>,
    last: HashMap<String, (DeviceStatus, Instant)>,
}

impl LogitechProvider {
    pub fn new() -> Self {
        Self {
            api: HidApi::new().ok(),
            diag: Vec::new(),
            ids: HashMap::new(),
            asleep: HashSet::new(),
            last: HashMap::new(),
        }
    }

    fn read_slot(&mut self, ch: &Channel, pid: u16, idx: u8) -> Option<DeviceStatus> {
        let slot = (pid, idx);
        // A paired device that is asleep does not answer at all, which would cost
        // the full ping timeout on every poll. Once a slot has gone silent, ping it
        // with the short timeout until it answers again.
        let timeout = if self.asleep.contains(&slot) { TIMEOUT } else { PING_TIMEOUT };
        match ch.request(idx, 0, 1, &[], timeout) {
            Reply::Ok(_) => {}
            Reply::Error => {
                // empty slot
                self.asleep.remove(&slot);
                return None;
            }
            Reply::Timeout => {
                self.asleep.insert(slot);
                let name = self.ids.get(&slot).map_or("paired device", |i| i.name.as_str());
                self.diag.push(format!("  idx={idx} '{name}': no answer (asleep or off)"));
                return None;
            }
        }
        self.asleep.remove(&slot);
        let id = self.ids.entry(slot).or_insert_with(|| ch.identity(idx)).clone();
        let name = if id.name.is_empty() { "Logitech device".to_string() } else { id.name };
        for feature in [F_UNIFIED, F_STATUS, F_VOLTAGE] {
            let fi = ch.feature_index(idx, feature);
            if fi == 0 {
                continue;
            }
            let func = if feature == F_UNIFIED { 1 } else { 0 };
            let Some(r) = ch.ask(idx, fi, func, &[]) else { continue };
            let (level, charging) = parse_battery(feature, &r);
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
        api.reset_devices().ok();
        api.add_devices(LOGITECH_VID, 0).ok();

        // pid -> (short path, long path, product string)
        let mut groups: HashMap<u16, (Option<std::ffi::CString>, Option<std::ffi::CString>, String)> =
            HashMap::new();
        for d in api.device_list() {
            if d.usage_page() != 0xFF00 || !matches!(d.usage(), 1 | 2) {
                continue;
            }
            let g = groups.entry(d.product_id()).or_default();
            if d.usage() == 1 {
                g.0 = Some(d.path().to_owned());
            } else {
                g.1 = Some(d.path().to_owned());
            }
            if g.2.is_empty() {
                g.2 = d.product_string().unwrap_or_default().to_string();
            }
        }

        let mut found: HashMap<String, DeviceStatus> = HashMap::new();
        for (pid, (short, long, product)) in &groups {
            let Some(long) = long else { continue };
            let receiver = product.to_lowercase().contains("receiver");
            self.diag.push(format!("[Logitech] pid={pid:04x} '{product}'"));
            let ch = match Channel::open(&api, short.as_deref(), long) {
                Ok(ch) => ch,
                Err(e) => {
                    self.diag.push(format!("  open: {e}"));
                    continue;
                }
            };
            let slots: Vec<u8> = if receiver { (1..=6).collect() } else { vec![0xFF] };
            for idx in slots {
                if let Some(st) = self.read_slot(&ch, *pid, idx) {
                    // the same device on the cable and through the receiver: charging wins
                    if !found.contains_key(&st.key) || st.charging {
                        found.insert(st.key.clone(), st);
                    }
                }
            }
        }
        self.api = Some(api);

        let now = Instant::now();
        let mut out: Vec<DeviceStatus> = found.values().cloned().collect();
        for st in &out {
            self.last.insert(st.key.clone(), (st.clone(), now));
        }
        // asleep or switched off: keep the last value greyed out for a while
        let has_receiver = !groups.is_empty();
        self.last.retain(|key, (st, t)| {
            if found.contains_key(key) {
                return true;
            }
            if has_receiver && now.duration_since(*t) < ASLEEP_KEEP {
                out.push(DeviceStatus { online: false, ..st.clone() });
                true
            } else {
                false
            }
        });
        out
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diag.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltage_curve_interpolates() {
        assert_eq!(voltage_to_percent(4200), 100);
        assert_eq!(voltage_to_percent(3811), 50);
        assert_eq!(voltage_to_percent(3835), 55);
        assert_eq!(voltage_to_percent(3400), 0);
    }

    #[test]
    fn parses_unified_battery() {
        assert_eq!(parse_battery(F_UNIFIED, &[80, 8, 0]), (Some(80), false));
        assert_eq!(parse_battery(F_UNIFIED, &[100, 8, 3]), (Some(100), true));
        assert_eq!(parse_battery(F_UNIFIED, &[0xFF, 0, 0]), (None, false));
    }

    #[test]
    fn parses_voltage_battery() {
        assert_eq!(parse_battery(F_VOLTAGE, &[0x0E, 0xE3, 0x80]), (Some(50), true));
        assert_eq!(parse_battery(F_VOLTAGE, &[0x00, 0x10, 0x00]), (None, false));
    }
}
