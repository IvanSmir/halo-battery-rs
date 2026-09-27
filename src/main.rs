// no console window in release builds; `--list` attaches to the parent's console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use halo_battery::{app, cli, platform};

fn main() {
    if std::env::args().any(|a| a == "--list") {
        cli::list();
        return;
    }
    if !platform::instance::claim_single_instance() {
        return;
    }
    app::run();
}
