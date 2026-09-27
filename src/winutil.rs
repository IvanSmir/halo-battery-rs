//! Small Windows helpers: taskbar theme and the "start with Windows" entry.

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("HaloBatteryRs");

/// Whether the taskbar uses the light theme (icons are then drawn black).
pub fn taskbar_is_light() -> bool {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    rc.is_ok() && value == 1
}

fn exe_command() -> Option<String> {
    std::env::current_exe().ok().map(|p| format!("\"{}\"", p.display()))
}

pub fn autostart_enabled() -> bool {
    let mut buf = [0u16; 1024];
    let mut size = std::mem::size_of_val(&buf) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if rc.is_err() {
        return false;
    }
    let len = (size as usize / 2).saturating_sub(1);
    // only counts when it points at this very executable
    Some(String::from_utf16_lossy(&buf[..len])) == exe_command()
}

pub fn set_autostart(enable: bool) {
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
            Some(wide.as_ptr() as *const _),
            (wide.len() * 2) as u32,
        );
    }
}

/// `false` when another instance is already running.
pub fn claim_single_instance() -> bool {
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    // the handle stays open for the life of the process on purpose
    let created = unsafe { CreateMutexW(None, true, w!("Local\\HaloBatteryRs")) };
    created.is_ok() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}

pub fn attach_parent_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
