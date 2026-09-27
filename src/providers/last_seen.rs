//! Keeps showing a device that stopped answering (asleep, switched off) with
//! its last reading, greyed out, for a while.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::device::DeviceStatus;

pub struct LastSeen {
    keep: Duration,
    last: HashMap<String, (DeviceStatus, Instant)>,
}

impl LastSeen {
    pub fn new(keep: Duration) -> Self {
        Self { keep, last: HashMap::new() }
    }

    /// Records the devices `found` at `now` and returns them, followed by the
    /// ones seen within the keep window that are missing now, marked offline.
    /// With `keep_missing` false (e.g. the receiver itself is gone) missing
    /// devices are dropped at once.
    pub fn merge(&mut self, found: Vec<DeviceStatus>, now: Instant, keep_missing: bool) -> Vec<DeviceStatus> {
        for st in &found {
            self.last.insert(st.key.clone(), (st.clone(), now));
        }
        let mut out = found;
        let keep = self.keep;
        self.last.retain(|key, (st, seen)| {
            if out.iter().any(|d| &d.key == key) {
                return true;
            }
            if keep_missing && now.duration_since(*seen) < keep {
                out.push(DeviceStatus { online: false, ..st.clone() });
                true
            } else {
                false
            }
        });
        out
    }
}
