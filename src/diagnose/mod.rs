//! The diagnostics report someone can send to get a device supported: what
//! Halo Battery recognises, every HID interface with its report descriptor,
//! and the Bluetooth devices with the battery Windows knows.
//!
//! Everything here only reads: descriptors come from what Windows already
//! parsed, and no report is ever sent to a device.
//! - [`descriptor`]: which usage pages a descriptor declares (pure)
//! - [`report`]: the report and its text (pure)

pub mod descriptor;
pub mod report;

use std::path::PathBuf;
use std::time::Duration;

use hidapi::HidApi;

use self::report::{HidInterface, Report};
use crate::platform::{bluetooth, folders, system, winrt};
use crate::providers;
use crate::providers::inventory::Scanner;

/// Where a report goes by default: the Desktop, named after the date and time.
pub fn default_path() -> PathBuf {
    let stamp: String = system::local_time().chars().filter(|c| c.is_ascii_digit()).collect();
    folders::desktop().unwrap_or_else(|| PathBuf::from(".")).join(format!("halo-diagnostico-{stamp}.txt"))
}

/// Windows.Gaming.Input fills its controller list asynchronously.
const WARM_UP: Duration = Duration::from_millis(1500);
/// Report descriptors are at most 4 KiB (HID_API_MAX_REPORT_DESCRIPTOR_SIZE).
const MAX_DESCRIPTOR: usize = 4096;

/// What every provider sees, one line per finding. Also what `--list` prints.
pub fn provider_lines() -> Vec<String> {
    winrt::init();
    let mut all = providers::all();
    let mut scanner = Scanner::new();
    std::thread::sleep(WARM_UP);
    let scan = scanner.scan();
    let mut lines = Vec::new();
    for p in &mut all {
        let devices = p.poll(&scan);
        lines.extend(p.diagnostics());
        for d in devices {
            let level = d.level.map_or("?".to_string(), |l| format!("{l}%"));
            let state = match (d.online, d.charging) {
                (false, _) => ", asleep",
                (true, true) => ", charging",
                _ => "",
            };
            lines.push(format!("  => {}: {level}{state}", d.name));
        }
    }
    lines
}

fn hid_interfaces() -> Vec<HidInterface> {
    let Ok(api) = HidApi::new() else { return Vec::new() };
    let mut out: Vec<HidInterface> = api
        .device_list()
        .map(|d| {
            let descriptor = d.open_device(&api).map_err(|e| e.to_string()).and_then(|dev| {
                let mut buf = vec![0u8; MAX_DESCRIPTOR];
                let n = dev.get_report_descriptor(&mut buf).map_err(|e| e.to_string())?;
                buf.truncate(n);
                Ok(buf)
            });
            HidInterface {
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                manufacturer: d.manufacturer_string().unwrap_or_default().trim().to_string(),
                product: d.product_string().unwrap_or_default().trim().to_string(),
                bus: format!("{:?}", d.bus_type()),
                interface: d.interface_number(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                descriptor,
            }
        })
        .collect();
    out.sort_by_key(|h| (h.vendor_id, h.product_id, h.interface, h.usage_page, h.usage));
    out
}

/// Collects the whole report. Takes a couple of seconds (the controller list
/// needs a moment to fill).
pub fn collect() -> Report {
    Report {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        windows: system::windows_version(),
        date: system::local_time(),
        providers: provider_lines(),
        hid: hid_interfaces(),
        bluetooth: bluetooth::devices(),
    }
}
