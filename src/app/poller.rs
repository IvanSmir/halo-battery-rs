//! The background thread that reads the batteries.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use windows::Foundation::EventHandler;
use windows::Gaming::Input::RawGameController;

use crate::device::DeviceStatus;
use crate::platform::winrt;
use crate::providers;

/// Windows.Gaming.Input fills its controller list asynchronously; the first
/// read waits this long for it.
const WARM_UP: Duration = Duration::from_millis(1500);

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
            subscribe_controller_changes(events);
            thread::sleep(WARM_UP);
            let mut interval = interval;
            loop {
                let devices: Vec<DeviceStatus> = providers.iter_mut().flat_map(|p| p.poll()).collect();
                if out.send(devices).is_err() {
                    return;
                }
                match rx.recv_timeout(interval) {
                    Ok(Cmd::PollNow) | Err(RecvTimeoutError::Timeout) => {}
                    Ok(Cmd::SetInterval(i)) => interval = i,
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
