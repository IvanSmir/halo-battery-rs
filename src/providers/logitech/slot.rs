//! Reads the battery of the device in one receiver slot, remembering what it
//! learned so the next read is a single request.
//!
//! The first read pings the slot, reads the device identity and looks up its
//! battery feature (several requests). Later reads go straight to the battery
//! feature: its answer alone tells an awake device (a reading), an asleep one
//! (no answer) and a changed or emptied slot (an error, which drops what was
//! remembered and starts over).
//!
//! A silent slot is forgotten too. Putting another device in a slot means the
//! old one disconnects first, so the device that answers after a silence is
//! identified again rather than trusted to be the same one.

use std::time::Duration;

use super::protocol::{self, BATTERY_FEATURES, Reply};
use super::transport::{Answer, Identity, Transport};

/// What is remembered about a slot between reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotCache {
    pub identity: Identity,
    /// The battery feature that answered and its index on this device.
    pub battery: Option<(u16, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotRead {
    /// No device paired in the slot.
    Empty,
    /// A device is paired but did not answer: asleep or switched off.
    Asleep,
    /// The device answered but none of its battery features gave a level.
    NoBattery,
    Battery {
        level: u8,
        charging: bool,
        feature: u16,
        raw: Vec<u8>,
    },
}

/// Reads slot `idx`, waiting up to `timeout` for the first answer. `cache` is
/// read and updated; it is cleared when the slot turns out to be empty or its
/// device stops answering.
pub fn read<T: Transport + ?Sized>(t: &T, idx: u8, cache: &mut Option<SlotCache>, timeout: Duration) -> SlotRead {
    if let Some((feature, fi)) = cache.as_ref().and_then(|c| c.battery) {
        match t.request(idx, fi, protocol::battery_function(feature), &[], timeout) {
            Answer::Timeout => {
                *cache = None;
                return SlotRead::Asleep;
            }
            Answer::Reply(Reply::Ok(raw)) => {
                if let (Some(level), charging) = protocol::parse_battery(feature, &raw) {
                    return SlotRead::Battery { level, charging, feature, raw };
                }
                // no level from the remembered feature: look again below
            }
            // not the device we remember (or none at all): start over
            Answer::Reply(Reply::Error) => *cache = None,
        }
    }

    match t.request(idx, protocol::ROOT_INDEX, protocol::FN_ROOT_PING, &[], timeout) {
        Answer::Timeout => {
            *cache = None;
            return SlotRead::Asleep;
        }
        Answer::Reply(Reply::Error) => {
            *cache = None;
            return SlotRead::Empty;
        }
        Answer::Reply(Reply::Ok(_)) => {}
    }

    let identity = match cache.take() {
        Some(c) => c.identity,
        None => t.identity(idx),
    };
    let mut result = SlotRead::NoBattery;
    let mut battery = None;
    for feature in BATTERY_FEATURES {
        let fi = t.feature_index(idx, feature);
        if fi == 0 {
            continue;
        }
        let Some(raw) = t.ask(idx, fi, protocol::battery_function(feature), &[]) else { continue };
        if let (Some(level), charging) = protocol::parse_battery(feature, &raw) {
            battery = Some((feature, fi));
            result = SlotRead::Battery { level, charging, feature, raw };
            break;
        }
    }
    *cache = Some(SlotCache { identity, battery });
    result
}
