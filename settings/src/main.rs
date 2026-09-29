//! Prototype of the settings window with Tauri.
//! The UI is plain HTML/CSS/JS in ui/; Rust reads the devices in the
//! background and pushes every change to the window as a "devices" event.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::Duration;

use halo_battery::device::{DeviceStatus, Kind};
use halo_battery::{platform, providers};
use serde::Serialize;
use tauri::{Emitter, Manager};

/// Poll quickly at first, while Windows is still filling its controller list.
const FAST_POLL: Duration = Duration::from_secs(1);
const FAST_POLLS: u32 = 8;
const POLL: Duration = Duration::from_secs(15);

#[derive(Clone, Serialize, PartialEq)]
struct Device {
    key: String,
    name: String,
    kind: &'static str,
    level: Option<u8>,
    charging: bool,
    online: bool,
}

impl From<DeviceStatus> for Device {
    fn from(d: DeviceStatus) -> Self {
        let kind = match d.kind {
            Kind::Mouse => "mouse",
            Kind::Keyboard => "keyboard",
            Kind::Headset => "headset",
            Kind::Gamepad => "gamepad",
        };
        Self { key: d.key, name: d.name, kind, level: d.level, charging: d.charging, online: d.online }
    }
}

/// The last reading; `None` until the first poll is done.
#[derive(Default)]
struct Latest(Mutex<Option<Vec<Device>>>);

/// The last reading, for a window that opens (or reloads) after it was sent.
#[tauri::command]
fn devices(latest: tauri::State<'_, Latest>) -> Option<Vec<Device>> {
    latest.0.lock().unwrap().clone()
}

fn spawn_poller(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        platform::winrt::init();
        let mut all = providers::all();
        let mut polls = 0;
        loop {
            let now: Vec<Device> = all.iter_mut().flat_map(|p| p.poll()).map(Device::from).collect();
            let latest = app.state::<Latest>();
            let changed = latest.0.lock().unwrap().as_ref() != Some(&now);
            if changed {
                *latest.0.lock().unwrap() = Some(now.clone());
                let _ = app.emit("devices", now);
            }
            polls += 1;
            std::thread::sleep(if polls < FAST_POLLS { FAST_POLL } else { POLL });
        }
    });
}

fn main() {
    tauri::Builder::default()
        .manage(Latest::default())
        .setup(|app| {
            spawn_poller(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![devices])
        .run(tauri::generate_context!())
        .expect("failed to start the settings window");
}
