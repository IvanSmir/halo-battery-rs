//! HID++ 2.0 message layout and payload decoding. Pure functions, no I/O.
//!
//! A long request on the receiver's vendor interface ff00:0002 is
//! `11 <device index> <feature index> <function << 4 | swid> <params...>`;
//! the device index is 1..6 behind a receiver and 0xFF for a device on the
//! cable. Feature index 0 is the root feature: function 0 maps a feature id to
//! its index, function 1 is a ping.

use crate::device::Kind;

/// Software id carried in every request; replies echo it.
pub const SWID: u8 = 0x0A;
/// Length of a long HID++ report, report id included.
pub const LONG_LEN: usize = 20;

pub const ROOT_INDEX: u8 = 0x00;
pub const FN_ROOT_GET_FEATURE: u8 = 0;
pub const FN_ROOT_PING: u8 = 1;

/// Device information: fn 0 answers `<entities> <unit id: 4 bytes> ...`.
pub const F_INFO: u16 = 0x0003;
/// Device name: fn 0 length, fn 1 characters from an offset, fn 2 device type.
pub const F_NAME: u16 = 0x0005;
/// Unified battery, fn 1: `<percent> <level flags> <charging status>`.
pub const F_UNIFIED: u16 = 0x1004;
/// Battery status, fn 0: `<percent> <next level> <status>`.
pub const F_STATUS: u16 = 0x1000;
/// Battery voltage, fn 0: `<mV hi> <mV lo> <flags>`.
pub const F_VOLTAGE: u16 = 0x1001;

/// Battery features in order of preference.
pub const BATTERY_FEATURES: [u16; 3] = [F_UNIFIED, F_STATUS, F_VOLTAGE];

/// The function that reads the battery of a battery feature.
pub fn battery_function(feature: u16) -> u8 {
    if feature == F_UNIFIED { 1 } else { 0 }
}

/// A long request, ready to write.
pub fn request(idx: u8, feat: u8, func: u8, params: &[u8]) -> [u8; LONG_LEN] {
    let mut req = [0u8; LONG_LEN];
    req[..4].copy_from_slice(&[0x11, idx, feat, (func << 4) | SWID]);
    let n = params.len().min(LONG_LEN - 4);
    req[4..4 + n].copy_from_slice(&params[..n]);
    req
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    /// The params of the answer, zero padded so fixed offsets can be indexed.
    Ok(Vec<u8>),
    /// The device answered with an error: an empty receiver slot, a missing feature.
    Error,
}

/// Matches an incoming report against a request. `None` when the report
/// belongs to something else (another slot, another request, a notification).
pub fn match_reply(report: &[u8], idx: u8, feat: u8, func: u8) -> Option<Reply> {
    if report.len() < 4 || report[1] != idx {
        return None;
    }
    // error replies: `10 <idx> 8f <feat> ...` (short) or `11 <idx> ff <feat> ...` (long)
    if matches!(report[2], 0x8F | 0xFF) && report[3] == feat {
        return Some(Reply::Error);
    }
    if report[2] == feat && report[3] == (func << 4) | SWID {
        let mut p = report[4..].to_vec();
        p.resize(p.len() + 16, 0);
        return Some(Reply::Ok(p));
    }
    None
}

/// Li-ion discharge curve used by Solaar, mV -> %.
const VOLTAGE_CURVE: [(u32, u32); 13] = [
    (4186, 100),
    (4067, 90),
    (3989, 80),
    (3922, 70),
    (3859, 60),
    (3811, 50),
    (3778, 40),
    (3751, 30),
    (3717, 20),
    (3671, 10),
    (3646, 5),
    (3579, 2),
    (3500, 0),
];

pub fn voltage_to_percent(mv: u32) -> u8 {
    if mv >= VOLTAGE_CURVE[0].0 {
        return 100;
    }
    for w in VOLTAGE_CURVE.windows(2) {
        let ((hi_mv, hi_p), (lo_mv, lo_p)) = (w[0], w[1]);
        if mv >= lo_mv {
            let p = lo_p as f32
                + (mv - lo_mv) as f32 * (hi_p - lo_p) as f32 / (hi_mv - lo_mv) as f32;
            return p.round() as u8;
        }
    }
    0
}

/// Battery level and "on external power" from the params of a battery reply.
/// A full battery still on the charger counts as charging.
pub fn parse_battery(feature: u16, p: &[u8]) -> (Option<u8>, bool) {
    if p.len() < 3 {
        return (None, false);
    }
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

/// Device type from the name feature, fn 2.
pub fn kind_from_type(t: u8) -> Kind {
    match t {
        0 | 2 => Kind::Keyboard,
        _ => Kind::Mouse,
    }
}

/// The 4-byte unit id from a device information reply, as hex; `None` when
/// the device leaves it blank.
pub fn unit_id(info: &[u8]) -> Option<String> {
    let id = info.get(1..5)?;
    id.iter().any(|&b| b != 0).then(|| id.iter().map(|b| format!("{b:02X}")).collect())
}
