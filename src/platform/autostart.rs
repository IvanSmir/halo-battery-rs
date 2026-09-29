//! The "start with Windows" entry under HKCU\...\CurrentVersion\Run. It
//! always starts the tray; the settings window manages it on its behalf.

use std::path::Path;

use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::{PCWSTR, w};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE: PCWSTR = w!("HaloBatteryRs");

/// The command line the Run entry holds for `exe`: its path, quoted.
fn command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Whether the Run entry exists and starts `exe`.
pub fn is_enabled(exe: &Path) -> bool {
    let mut buf = [0u16; 1024];
    let mut size = std::mem::size_of_val(&buf) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if rc.is_err() {
        return false;
    }
    let len = (size as usize / 2).saturating_sub(1);
    String::from_utf16_lossy(&buf[..len]) == command(exe)
}

/// Adds (or removes) the Run entry that starts `exe` at logon.
pub fn set_enabled(exe: &Path, enable: bool) {
    unsafe {
        if !enable {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
            return;
        }
        let wide: Vec<u16> = command(exe).encode_utf16().chain([0]).collect();
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE,
            REG_SZ.0,
            Some(wide.as_ptr().cast()),
            (wide.len() * 2) as u32,
        );
    }
}
