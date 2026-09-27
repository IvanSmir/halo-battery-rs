//! The taskbar theme, which decides whether icons are drawn white or black.

use windows::core::w;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

/// Whether the taskbar uses the light theme (icons are then drawn black).
pub fn taskbar_is_light() -> bool {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    rc.is_ok() && value == 1
}
