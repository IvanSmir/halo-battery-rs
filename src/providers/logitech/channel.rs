//! Request/response I/O with a receiver (or a cabled device) over hidapi.

use std::ffi::CStr;
use std::thread;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice, HidResult};

use super::protocol::{self, F_INFO, F_NAME, Reply};
use crate::device::Kind;

/// How long a normal request waits for its answer.
pub const TIMEOUT: Duration = Duration::from_millis(600);
/// Pause between two reads while waiting for an answer.
const READ_GAP: Duration = Duration::from_millis(5);

/// What was learned about a paired device; read once per receiver slot.
#[derive(Clone, Debug)]
pub struct Identity {
    pub name: String,
    pub kind: Kind,
    /// Stable across receiver and cable; empty when the device has none.
    pub unit: String,
}

pub enum Answer {
    Reply(Reply),
    Timeout,
}

/// The receiver's HID++ long interface, plus the short one on which some
/// errors come back.
pub struct Channel {
    long: HidDevice,
    short: Option<HidDevice>,
}

impl Channel {
    pub fn open(api: &HidApi, short: Option<&CStr>, long: &CStr) -> HidResult<Self> {
        let long = api.open_path(long)?;
        long.set_blocking_mode(false)?;
        let short = short.and_then(|p| api.open_path(p).ok());
        if let Some(s) = &short {
            s.set_blocking_mode(false).ok();
        }
        Ok(Self { long, short })
    }

    pub fn request(&self, idx: u8, feat: u8, func: u8, params: &[u8], timeout: Duration) -> Answer {
        if self.long.write(&protocol::request(idx, feat, func, params)).is_err() {
            return Answer::Timeout;
        }
        let end = Instant::now() + timeout;
        let mut buf = [0u8; 64];
        while Instant::now() < end {
            for dev in std::iter::once(&self.long).chain(self.short.as_ref()) {
                let Ok(n) = dev.read(&mut buf) else { continue };
                if let Some(reply) = protocol::match_reply(&buf[..n], idx, feat, func) {
                    return Answer::Reply(reply);
                }
            }
            thread::sleep(READ_GAP);
        }
        Answer::Timeout
    }

    /// The params of a successful answer, `None` on an error or a timeout.
    pub fn ask(&self, idx: u8, feat: u8, func: u8, params: &[u8]) -> Option<Vec<u8>> {
        match self.request(idx, feat, func, params, TIMEOUT) {
            Answer::Reply(Reply::Ok(p)) => Some(p),
            _ => None,
        }
    }

    /// Index of a feature on the device at `idx`, 0 when it does not have it.
    pub fn feature_index(&self, idx: u8, feature_id: u16) -> u8 {
        self.ask(idx, protocol::ROOT_INDEX, protocol::FN_ROOT_GET_FEATURE, &feature_id.to_be_bytes())
            .map_or(0, |r| r[0])
    }

    /// Name, kind and unit id of the device at `idx`; parts it cannot read are
    /// left empty (and the kind defaults to a mouse).
    pub fn identity(&self, idx: u8) -> Identity {
        let mut id = Identity { name: String::new(), kind: Kind::Mouse, unit: String::new() };
        let fi = self.feature_index(idx, F_NAME);
        if fi != 0 {
            id.name = self.read_name(idx, fi);
            if let Some(r) = self.ask(idx, fi, 2, &[]) {
                id.kind = protocol::kind_from_type(r[0]);
            }
        }
        let fi = self.feature_index(idx, F_INFO);
        let info = if fi != 0 { self.ask(idx, fi, 0, &[]) } else { None };
        id.unit = info.and_then(|r| protocol::unit_id(&r)).unwrap_or_default();
        id
    }

    /// The name comes 16 characters at a time from increasing offsets.
    fn read_name(&self, idx: u8, fi: u8) -> String {
        let len = self.ask(idx, fi, 0, &[]).map_or(0, |r| r[0] as usize);
        let mut raw = Vec::with_capacity(len);
        while raw.len() < len {
            match self.ask(idx, fi, 1, &[raw.len() as u8]) {
                Some(r) => raw.extend_from_slice(&r[..16]),
                None => break,
            }
        }
        raw.truncate(len);
        String::from_utf8_lossy(&raw).trim().to_string()
    }
}
