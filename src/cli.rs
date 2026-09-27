//! `--list`: prints what every provider sees, for troubleshooting.

use std::time::Duration;

use crate::{platform, providers};

pub fn list() {
    platform::console::attach_parent_console();
    platform::winrt::init();
    let mut providers = providers::all();
    // Windows.Gaming.Input fills its controller list asynchronously
    std::thread::sleep(Duration::from_millis(1500));
    for p in &mut providers {
        let devices = p.poll();
        for line in p.diagnostics() {
            println!("{line}");
        }
        for d in devices {
            println!("  => {}: {:?}% charging={} online={}", d.name, d.level, d.charging, d.online);
        }
    }
}
