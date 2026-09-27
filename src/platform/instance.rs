//! Keeps a single instance of the app running.

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;

/// `false` when another instance is already running.
pub fn claim_single_instance() -> bool {
    // the handle stays open for the life of the process on purpose
    let created = unsafe { CreateMutexW(None, true, w!(r"Local\HaloBatteryRs")) };
    created.is_ok() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}
