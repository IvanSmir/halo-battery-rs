//! Command-line modes of the tray executable:
//! - `--list`: what every provider sees, for troubleshooting
//! - `--diagnose [file]`: the full diagnostics report, to get a device supported

use std::path::PathBuf;

use crate::{diagnose, platform};

pub fn list() {
    platform::console::attach_parent_console();
    for line in diagnose::provider_lines() {
        println!("{line}");
    }
}

/// Writes the diagnostics report to `path` (the Desktop by default) and
/// prints where it went. Exits with an error status when it cannot write.
pub fn diagnose(path: Option<PathBuf>) {
    platform::console::attach_parent_console();
    let path = path.unwrap_or_else(diagnose::default_path);
    let report = diagnose::collect().to_string();
    match std::fs::write(&path, report) {
        Ok(()) => println!("{}", path.display()),
        Err(e) => {
            eprintln!("no se pudo escribir {}: {e}", path.display());
            std::process::exit(1);
        }
    }
}
