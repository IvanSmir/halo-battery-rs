//! The two executables find each other: they are shipped side by side.

use std::path::PathBuf;
use std::process::Command;

pub const TRAY_EXE: &str = "halo-battery.exe";
pub const SETTINGS_EXE: &str = "halo-settings.exe";

/// `name` in the directory of the running executable.
pub fn sibling(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(name))
}

/// Opens the settings window. It keeps a single instance itself, so asking
/// twice only brings the open window to the front.
pub fn open_settings() -> std::io::Result<()> {
    let exe = sibling(SETTINGS_EXE).ok_or_else(|| std::io::Error::other("cannot locate the settings window"))?;
    Command::new(exe).spawn().map(drop)
}

/// Starts the tray (from the settings window, when it is not running).
pub fn start_tray() -> std::io::Result<()> {
    let exe = sibling(TRAY_EXE).ok_or_else(|| std::io::Error::other("cannot locate the tray"))?;
    Command::new(exe).spawn().map(drop)
}

/// Opens an `https://` address in the default browser. Anything else is
/// refused, so a stored URL can never start a program.
pub fn open_url(url: &str) -> std::io::Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, w};

    if !url.starts_with("https://") {
        return Err(std::io::Error::other(format!("not an https address: {url}")));
    }
    let result = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(url), None, None, SW_SHOWNORMAL) };
    // ShellExecute reports success with a value above 32
    if result.0 as usize > 32 { Ok(()) } else { Err(std::io::Error::other("could not open the browser")) }
}
