//! The settings window. The UI is plain HTML/CSS/JS in ui/; this side only
//! reads and writes the files it shares with the tray:
//! - config.json, written here on every change and applied by the tray
//! - devices.json, published by the tray and pushed to the UI as it changes
//!
//! The window never talks to the hardware itself.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use halo_battery::config::Config;
use halo_battery::device::DeviceStatus;
use halo_battery::diagnose;
use halo_battery::platform::{autostart, instance, launch};
use halo_battery::snapshot::Snapshot;
use halo_battery::storage::FileWatch;
use halo_battery::update::{self, state::StoredRelease, state::UpdateState};
use serde::Serialize;
use tauri::{Emitter, Manager};

/// How often devices.json, update.json and the tray process are checked.
const WATCH: Duration = Duration::from_millis(500);

/// Everything the UI shows when it opens.
#[derive(Serialize)]
struct Initial {
    config: Config,
    devices: Vec<DeviceStatus>,
    autostart: bool,
    tray_running: bool,
    version: &'static str,
    /// A newer release the tray found, if any.
    update: Option<StoredRelease>,
}

fn tray_exe() -> Option<PathBuf> {
    launch::sibling(launch::TRAY_EXE)
}

fn devices() -> Vec<DeviceStatus> {
    Snapshot::default_path().map(|p| Snapshot::load_from(&p).devices).unwrap_or_default()
}

/// The newer release the tray recorded, when there is one.
fn available_update() -> Option<StoredRelease> {
    let state = UpdateState::load_from(&UpdateState::default_path()?);
    state.available(&update::current_version()).cloned()
}

#[tauri::command]
fn load() -> Initial {
    Initial {
        config: Config::load(),
        devices: devices(),
        autostart: tray_exe().is_some_and(|exe| autostart::is_enabled(&exe)),
        tray_running: instance::tray_running(),
        version: env!("CARGO_PKG_VERSION"),
        update: available_update(),
    }
}

/// Opens the download page of the available update. The address is read
/// from update.json here, never taken from the page.
#[tauri::command]
fn open_update() -> Result<(), String> {
    let release = available_update().ok_or("no hay ninguna actualización disponible")?;
    launch::open_url(&release.url).map_err(|e| e.to_string())
}

/// Saves the settings; the tray picks them up from the file. Returns them as
/// stored (trimmed aliases, clamped values) so the UI can show what counts.
#[tauri::command]
fn save_config(config: Config) -> Result<Config, String> {
    let config = config.sanitized();
    config.save().map_err(|e| e.to_string())?;
    Ok(config)
}

/// Turns starting the tray at logon on or off; returns the resulting state.
#[tauri::command]
fn set_autostart(enabled: bool) -> bool {
    let Some(exe) = tray_exe() else { return false };
    autostart::set_enabled(&exe, enabled);
    autostart::is_enabled(&exe)
}

#[tauri::command]
fn start_tray() -> Result<(), String> {
    launch::start_tray().map_err(|e| e.to_string())
}

/// Writes the diagnostics report to the Desktop and shows it in Explorer.
/// The tray executable collects it (`--diagnose`), so this window still never
/// talks to the hardware. Returns the file name.
#[tauri::command]
async fn export_diagnostics() -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = tray_exe().ok_or("no se encuentra halo-battery.exe")?;
    let path = diagnose::default_path();
    let out = path.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new(exe).arg("--diagnose").arg(&out).creation_flags(CREATE_NO_WINDOW).status()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if !status.success() || !path.exists() {
        return Err("no se pudo generar el informe".into());
    }
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

/// Pushes "devices" when the tray publishes new readings, "update" when it
/// records an update check and "tray" when it starts or stops.
fn spawn_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut snapshot = Snapshot::default_path().map(|p| FileWatch::new(p, WATCH));
        let mut update_state = UpdateState::default_path().map(|p| FileWatch::new(p, WATCH));
        let mut running = instance::tray_running();
        loop {
            std::thread::sleep(WATCH);
            if snapshot.as_mut().is_some_and(|w| w.changed()) {
                let _ = app.emit("devices", devices());
            }
            if update_state.as_mut().is_some_and(|w| w.changed()) {
                let _ = app.emit("update", available_update());
            }
            let now = instance::tray_running();
            if now != running {
                running = now;
                let _ = app.emit("tray", running);
            }
        }
    });
}

fn main() {
    tauri::Builder::default()
        // a second launch (a tray click while open) brings this window forward
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            spawn_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load,
            save_config,
            set_autostart,
            start_tray,
            export_diagnostics,
            open_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the settings window");
}
