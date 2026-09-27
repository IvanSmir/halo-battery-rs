//! Windows Runtime initialisation.

use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};

/// Joins the calling thread to the multithreaded apartment. Needed once per
/// thread before any WinRT call (Windows.Gaming.Input).
pub fn init() {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}
