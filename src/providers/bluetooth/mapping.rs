//! From Windows' Bluetooth device nodes to one entry per physical device.
//! Pure functions, no I/O.
//!
//! One device is several nodes: the root node (`BTHENUM\DEV_<MAC>` for
//! classic Bluetooth, `BTHLE\DEV_<MAC>` for LE) and one node per service it
//! offers. Every node's instance id carries the device's MAC address, which
//! is how they are grouped.

use crate::device::Kind;
use crate::platform::bluetooth::BluetoothDevice;

/// Service UUIDs (their first 32 bits) of audio profiles: A2DP sink,
/// hands-free, headset, headset HS. A device offering one is a headset.
const AUDIO_SERVICES: [&str; 4] = ["0000110B-", "0000111E-", "00001108-", "00001131-"];

/// One physical device and what its nodes say about it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grouped {
    /// The MAC address as 12 upper-case hex digits.
    pub mac: String,
    /// Bluetooth LE rather than classic.
    pub le: bool,
    /// The root node's name, else the first name found.
    pub name: String,
    /// The battery level Windows knows, from whichever node holds it.
    pub level: Option<u8>,
    /// It offers an audio profile.
    pub audio: bool,
}

impl Grouped {
    /// The address as the number WinRT expects.
    pub fn address(&self) -> Option<u64> {
        u64::from_str_radix(&self.mac, 16).ok()
    }
}

fn is_hex(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

/// Every run of exactly 12 hex digits in `s`.
fn hex12_runs(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !is_hex(b[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && is_hex(b[i]) {
            i += 1;
        }
        if i - start == 12 {
            out.push(&s[start..i]);
        }
    }
    out
}

/// The device's MAC address from the instance id of any of its nodes.
pub fn mac_of(instance_id: &str) -> Option<String> {
    let id = instance_id.to_ascii_uppercase();
    // root nodes name it explicitly: ...\DEV_<MAC>\...
    if let Some(pos) = id.find(r"\DEV_") {
        let rest = &id[pos + 5..];
        let mac = rest.get(..12).filter(|m| m.bytes().all(is_hex));
        let ends = rest.as_bytes().get(12).is_none_or(|&c| !is_hex(c));
        if let (Some(mac), true) = (mac, ends) {
            return Some(mac.to_string());
        }
    }
    // service nodes end with it: ...&0&5C9BA67E10EF_C00000000
    // (an all-zero address belongs to a local service, e.g. a serial port, not a device)
    hex12_runs(&id).last().map(|s| s.to_string()).filter(|mac| mac != NO_ADDRESS)
}

const NO_ADDRESS: &str = "000000000000";

/// Whether the node is a device's root node (`BTHENUM\DEV_…` / `BTHLE\DEV_…`).
pub fn is_root(instance_id: &str) -> bool {
    let id = instance_id.to_ascii_uppercase();
    id.starts_with(r"BTHENUM\DEV_") || id.starts_with(r"BTHLE\DEV_")
}

/// One entry per MAC address, in the order the devices first appear.
pub fn group(nodes: &[BluetoothDevice]) -> Vec<Grouped> {
    let mut out: Vec<Grouped> = Vec::new();
    for node in nodes {
        let Some(mac) = mac_of(&node.instance_id) else { continue };
        let id = node.instance_id.to_ascii_uppercase();
        let i = match out.iter().position(|g| g.mac == mac) {
            Some(i) => i,
            None => {
                out.push(Grouped { mac, ..Grouped::default() });
                out.len() - 1
            }
        };
        let g = &mut out[i];
        // BTHLE\ (the device) or BTHLEDEVICE\ (one of its services)
        if id.starts_with("BTHLE") {
            g.le = true;
        }
        // the root node's name wins; before it shows up, any name will do
        if (is_root(&id) && !node.name.is_empty()) || g.name.is_empty() {
            g.name.clone_from(&node.name);
        }
        if g.level.is_none() {
            g.level = node.battery;
        }
        if AUDIO_SERVICES.iter().any(|s| id.contains(s)) {
            g.audio = true;
        }
    }
    out
}

/// The kind of a classic device from its Class of Device.
pub fn kind_from_class(class: u32) -> Option<Kind> {
    let major = (class >> 8) & 0x1F;
    let minor = (class >> 2) & 0x3F;
    match major {
        // audio/video: wearable headset, hands-free, headphones
        4 if matches!(minor, 1 | 2 | 6) => Some(Kind::Headset),
        // peripheral: low bits joystick/gamepad, high bits keyboard/pointer
        5 if matches!(minor & 0x0F, 1 | 2) => Some(Kind::Gamepad),
        5 => match minor >> 4 {
            1 | 3 => Some(Kind::Keyboard),
            2 => Some(Kind::Mouse),
            _ => None,
        },
        _ => None,
    }
}

/// The kind of an LE device from its GAP Appearance.
pub fn kind_from_appearance(appearance: u16) -> Option<Kind> {
    let (category, sub) = (appearance >> 6, appearance & 0x3F);
    match category {
        // Human Interface Device
        0x0F => match sub {
            1 => Some(Kind::Keyboard),
            2 => Some(Kind::Mouse),
            3 | 4 => Some(Kind::Gamepad),
            _ => None,
        },
        // Wearable Audio Device
        0x25 => Some(Kind::Headset),
        _ => None,
    }
}
