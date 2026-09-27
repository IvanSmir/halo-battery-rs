//! The "start with Windows" entry under HKCU\...\CurrentVersion\Run.

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE: PCWSTR = w!("HaloBatteryRs");

/// The command line the Run entry holds: this executable, quoted.
fn exe_command() -> Option<String> {
    std::env::current_exe().ok().map(|p| format!("\"{}\"", p.display()))
}

/// Whether the Run entry exists and points at this very executable.
pub fn is_enabled() -> bool {
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
    Some(String::from_utf16_lossy(&buf[..len])) == exe_command()
}

pub fn set_enabled(enable: bool) {
    unsafe {
        if !enable {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
            return;
        }
        let Some(cmd) = exe_command() else { return };
        let wide: Vec<u16> = cmd.encode_utf16().chain([0]).collect();
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
