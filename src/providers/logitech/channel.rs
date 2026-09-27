//! [`Transport`] over hidapi: the receiver's (or cabled device's) HID++
//! interfaces.

use std::ffi::CStr;
use std::thread;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice, HidResult};

use super::protocol;
use super::transport::{Answer, Transport};

/// Pause between two reads while waiting for an answer.
const READ_GAP: Duration = Duration::from_millis(5);

/// The long interface, plus the short one on which some errors come back.
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
}

impl Transport for Channel {
    fn request(&self, idx: u8, feat: u8, func: u8, params: &[u8], timeout: Duration) -> Answer {
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
}
