// no console window in release builds; `--list` attaches to the parent's console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod device;
mod icon;
mod providers;
mod winutil;

use std::time::Duration;

fn main() {
    if std::env::args().any(|a| a == "--list") {
        list();
        return;
    }
    if !winutil::claim_single_instance() {
        return;
    }
    app::run();
}

/// Prints what every provider sees, for troubleshooting.
fn list() {
    winutil::attach_parent_console();
    app::init_winrt();
    let mut providers = app::all_providers();
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
