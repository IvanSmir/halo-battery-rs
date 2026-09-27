//! The request/response contract with a receiver, and the multi-request reads
//! built on it. [`super::channel::Channel`] implements it over hidapi; tests
//! implement it with scripted answers.

use std::time::Duration;

use super::protocol::{self, F_INFO, F_NAME, Reply};
use crate::device::Kind;

/// How long a normal request waits for its answer.
pub const TIMEOUT: Duration = Duration::from_millis(600);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Reply(Reply),
    /// No answer in time: the device is asleep or switched off.
    Timeout,
}

/// What was learned about a paired device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub kind: Kind,
    /// Stable across receiver and cable; empty when the device has none.
    pub unit: String,
}

pub trait Transport {
    /// Sends one request to the device at `idx` and waits for its answer.
    fn request(&self, idx: u8, feat: u8, func: u8, params: &[u8], timeout: Duration) -> Answer;

    /// The params of a successful answer, `None` on an error or a timeout.
    fn ask(&self, idx: u8, feat: u8, func: u8, params: &[u8]) -> Option<Vec<u8>> {
        match self.request(idx, feat, func, params, TIMEOUT) {
            Answer::Reply(Reply::Ok(p)) => Some(p),
            _ => None,
        }
    }

    /// Index of a feature on the device at `idx`, 0 when it does not have it.
    fn feature_index(&self, idx: u8, feature_id: u16) -> u8 {
        self.ask(idx, protocol::ROOT_INDEX, protocol::FN_ROOT_GET_FEATURE, &feature_id.to_be_bytes())
            .map_or(0, |r| r[0])
    }

    /// Name, kind and unit id of the device at `idx`; parts it cannot read are
    /// left empty (and the kind defaults to a mouse).
    fn identity(&self, idx: u8) -> Identity {
        let mut id = Identity { name: String::new(), kind: Kind::Mouse, unit: String::new() };
        let fi = self.feature_index(idx, F_NAME);
        if fi != 0 {
            id.name = read_name(self, idx, fi);
            if let Some(r) = self.ask(idx, fi, 2, &[]) {
                id.kind = protocol::kind_from_type(r[0]);
            }
        }
        let fi = self.feature_index(idx, F_INFO);
        let info = if fi != 0 { self.ask(idx, fi, 0, &[]) } else { None };
        id.unit = info.and_then(|r| protocol::unit_id(&r)).unwrap_or_default();
        id
    }
}

/// The name comes 16 characters at a time from increasing offsets.
fn read_name<T: Transport + ?Sized>(t: &T, idx: u8, fi: u8) -> String {
    let len = t.ask(idx, fi, 0, &[]).map_or(0, |r| r[0] as usize);
    let mut raw = Vec::with_capacity(len);
    while raw.len() < len {
        match t.ask(idx, fi, 1, &[raw.len() as u8]) {
            Some(r) => raw.extend_from_slice(&r[..16]),
            None => break,
        }
    }
    raw.truncate(len);
    String::from_utf8_lossy(&raw).trim().to_string()
}
