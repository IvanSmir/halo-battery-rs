//! Keeps a single instance of the tray running, and lets the settings window
//! tell whether it is.

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::{CreateMutexW, OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
use windows::core::{PCWSTR, w};

const TRAY_MUTEX: PCWSTR = w!(r"Local\HaloBatteryRs");

/// `false` when another instance of the tray is already running.
pub fn claim_single_instance() -> bool {
    // the handle stays open for the life of the process on purpose
    let created = unsafe { CreateMutexW(None, true, TRAY_MUTEX) };
    created.is_ok() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}

/// Whether the tray is running (its mutex exists).
pub fn tray_running() -> bool {
    match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, TRAY_MUTEX) } {
        Ok(h) => {
            let _ = unsafe { CloseHandle(h) };
            true
        }
        Err(_) => false,
    }
}
