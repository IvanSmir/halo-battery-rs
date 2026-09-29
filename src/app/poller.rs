//! The background thread that reads the batteries.
//!
//! It reads once at start, then at the configured interval, and at once when
//! asked to or when Windows reports a controller being connected or removed.
//! Windows.Gaming.Input fills its controller list asynchronously, so a
//! controller that is already on at start usually shows up through that
//! event rather than in the first read.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use windows::Foundation::EventHandler;
use windows::Gaming::Input::RawGameController;

use crate::device::DeviceStatus;
use crate::platform::winrt;
use crate::providers;
use crate::providers::inventory::Scanner;

enum Cmd {
    PollNow,
    SetInterval(Duration),
}

/// Handle to the poller thread. Every read sends the full device list to the
/// channel given to [`Poller::spawn`]; the thread ends once that channel's
/// receiver is dropped.
pub struct Poller {
    tx: Sender<Cmd>,
}

impl Poller {
    pub fn spawn(interval: Duration, out: Sender<Vec<DeviceStatus>>) -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let events = tx.clone();
        thread::spawn(move || {
            winrt::init();
            let mut providers = providers::all();
            // one listing of the HID devices per read, shared by every provider
            let mut scanner = Scanner::new();
            subscribe_controller_changes(events);
            let mut interval = interval;
            loop {
                let scan = scanner.scan();
                let devices: Vec<DeviceStatus> = providers.iter_mut().flat_map(|p| p.poll(&scan)).collect();
                if out.send(devices).is_err() {
                    return;
                }
                // Requests that came in during the read are served by a single
                // extra read: the read may have started before what they announce.
                if take_pending(&rx, &mut interval) {
                    continue;
                }
                match rx.recv_timeout(interval) {
                    Ok(cmd) => apply(cmd, &mut interval),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        Self { tx }
    }

    /// Reads all batteries now.
    pub fn poll_now(&self) {
        let _ = self.tx.send(Cmd::PollNow);
    }

    /// Reads now, then at the new interval.
    pub fn set_interval(&self, interval: Duration) {
        let _ = self.tx.send(Cmd::SetInterval(interval));
    }
}

fn apply(cmd: Cmd, interval: &mut Duration) {
    if let Cmd::SetInterval(i) = cmd {
        *interval = i;
    }
}

/// Applies every queued command; `true` when there was any.
fn take_pending(rx: &Receiver<Cmd>, interval: &mut Duration) -> bool {
    let mut any = false;
    while let Ok(cmd) = rx.try_recv() {
        apply(cmd, interval);
        any = true;
    }
    any
}

/// A controller switched on or off: read at once instead of waiting for the
/// next poll.
fn subscribe_controller_changes(tx: Sender<Cmd>) {
    let added = tx.clone();
    let _ = RawGameController::RawGameControllerAdded(&EventHandler::new(move |_, _| {
        let _ = added.send(Cmd::PollNow);
        Ok(())
    }));
    let _ = RawGameController::RawGameControllerRemoved(&EventHandler::new(move |_, _| {
        let _ = tx.send(Cmd::PollNow);
        Ok(())
    }));
}
