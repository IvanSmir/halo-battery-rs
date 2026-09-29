// no console window in release builds; the command-line modes attach to the parent's console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use halo_battery::{app, cli, platform};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--list") => return cli::list(),
        Some("--diagnose") => return cli::diagnose(args.get(1).map(PathBuf::from)),
        _ => {}
    }
    if !platform::instance::claim_single_instance() {
        return;
    }
    app::run();
}
