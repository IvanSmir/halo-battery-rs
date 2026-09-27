//! Slot reads against a scripted receiver.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use halo_battery::device::Kind;
use halo_battery::providers::logitech::protocol::{F_NAME, F_UNIFIED, Reply};
use halo_battery::providers::logitech::slot::{self, SlotCache, SlotRead};
use halo_battery::providers::logitech::transport::{Answer, Transport};

const IDX: u8 = 1;
const T: Duration = Duration::from_millis(1);
/// Where the scripted devices keep their features.
const NAME_INDEX: u8 = 5;
const BATTERY_INDEX: u8 = 4;

fn ok(params: &[u8]) -> Answer {
    let mut p = params.to_vec();
    p.resize(32, 0);
    Answer::Reply(Reply::Ok(p))
}

/// A receiver answering from a script. Feature lookups of anything not
/// scripted answer "not present"; any other request gets an error reply.
#[derive(Default)]
struct Receiver {
    script: HashMap<(u8, u8, Vec<u8>), Answer>,
    requests: RefCell<usize>,
}

impl Receiver {
    fn on(mut self, feat: u8, func: u8, params: &[u8], answer: Answer) -> Self {
        self.script.insert((feat, func, params.to_vec()), answer);
        self
    }

    /// A paired, awake mouse with a name and the unified battery feature at `battery_index`.
    fn mouse(name: &str, level: u8, battery_index: u8) -> Self {
        let bytes = name.as_bytes();
        Self::default()
            .on(0, 1, &[], ok(&[]))
            .on(0, 0, &F_NAME.to_be_bytes(), ok(&[NAME_INDEX]))
            .on(NAME_INDEX, 0, &[], ok(&[bytes.len() as u8]))
            .on(NAME_INDEX, 1, &[0], ok(bytes))
            .on(NAME_INDEX, 2, &[], ok(&[3]))
            .on(0, 0, &F_UNIFIED.to_be_bytes(), ok(&[battery_index]))
            .on(battery_index, 1, &[], ok(&[level, 8, 0]))
    }

    fn requests(&self) -> usize {
        *self.requests.borrow()
    }
}

impl Transport for Receiver {
    fn request(&self, _idx: u8, feat: u8, func: u8, params: &[u8], _timeout: Duration) -> Answer {
        *self.requests.borrow_mut() += 1;
        if let Some(a) = self.script.get(&(feat, func, params.to_vec())) {
            return a.clone();
        }
        if feat == 0 && func == 0 { ok(&[0]) } else { Answer::Reply(Reply::Error) }
    }
}

fn level_of(read: &SlotRead) -> Option<u8> {
    match read {
        SlotRead::Battery { level, .. } => Some(*level),
        _ => None,
    }
}

/// A cache that has learned `rx`'s device.
fn learned(rx: &Receiver) -> Option<SlotCache> {
    let mut cache = None;
    slot::read(rx, IDX, &mut cache, T);
    assert!(cache.is_some());
    cache
}

#[test]
fn the_first_read_learns_the_device() {
    let rx = Receiver::mouse("PRO X 2", 84, BATTERY_INDEX);
    let mut cache = None;
    let read = slot::read(&rx, IDX, &mut cache, T);
    assert_eq!(level_of(&read), Some(84));
    assert!(matches!(read, SlotRead::Battery { charging: false, feature: F_UNIFIED, .. }));

    let cache = cache.expect("the slot is remembered");
    assert_eq!(cache.identity.name, "PRO X 2");
    assert_eq!(cache.identity.kind, Kind::Mouse);
    assert_eq!(cache.battery, Some((F_UNIFIED, BATTERY_INDEX)));
}

#[test]
fn later_reads_take_a_single_request() {
    let first = Receiver::mouse("PRO X 2", 84, BATTERY_INDEX);
    let mut cache = learned(&first);
    assert!(first.requests() > 1);

    let rx = Receiver::mouse("PRO X 2", 80, BATTERY_INDEX);
    assert_eq!(level_of(&slot::read(&rx, IDX, &mut cache, T)), Some(80));
    assert_eq!(rx.requests(), 1);
}

#[test]
fn a_silent_device_is_asleep_and_forgotten() {
    let mut cache = learned(&Receiver::mouse("PRO X 2", 84, BATTERY_INDEX));
    let silent = Receiver::default().on(BATTERY_INDEX, 1, &[], Answer::Timeout);
    assert_eq!(slot::read(&silent, IDX, &mut cache, T), SlotRead::Asleep);
    assert_eq!(cache, None, "whatever answers next is identified again");
}

#[test]
fn a_device_that_wakes_up_in_the_slot_is_identified_again() {
    let mut cache = learned(&Receiver::mouse("PRO X 2", 84, BATTERY_INDEX));
    slot::read(&Receiver::default().on(BATTERY_INDEX, 1, &[], Answer::Timeout), IDX, &mut cache, T);

    // another mouse, even one keeping its battery at the same index
    let other = Receiver::mouse("G305", 50, BATTERY_INDEX);
    assert_eq!(level_of(&slot::read(&other, IDX, &mut cache, T)), Some(50));
    assert_eq!(cache.unwrap().identity.name, "G305");
}

#[test]
fn a_battery_read_error_makes_the_slot_learned_again() {
    let mut cache = learned(&Receiver::mouse("PRO X 2", 84, BATTERY_INDEX));
    // the device in the slot now keeps its battery elsewhere
    let other = Receiver::mouse("G305", 50, 7);
    assert_eq!(level_of(&slot::read(&other, IDX, &mut cache, T)), Some(50));
    let cache = cache.unwrap();
    assert_eq!(cache.identity.name, "G305");
    assert_eq!(cache.battery, Some((F_UNIFIED, 7)));
}

#[test]
fn an_emptied_slot_is_forgotten() {
    let mut cache = learned(&Receiver::mouse("PRO X 2", 84, BATTERY_INDEX));
    // every request, the ping included, gets an error reply
    assert_eq!(slot::read(&Receiver::default(), IDX, &mut cache, T), SlotRead::Empty);
    assert_eq!(cache, None);
}

#[test]
fn a_device_without_a_battery_feature_is_remembered_as_such() {
    let rx = Receiver::default().on(0, 1, &[], ok(&[]));
    let mut cache = None;
    assert_eq!(slot::read(&rx, IDX, &mut cache, T), SlotRead::NoBattery);
    assert_eq!(cache.unwrap().battery, None);
}

#[test]
fn a_never_seen_silent_slot_is_asleep() {
    let rx = Receiver::default().on(0, 1, &[], Answer::Timeout);
    let mut cache = None;
    assert_eq!(slot::read(&rx, IDX, &mut cache, T), SlotRead::Asleep);
    assert_eq!(cache, None);
}
