//! Console output for a GUI-subsystem executable.

use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

/// Sends stdout to the console the app was started from, if any. Release
/// builds have no console of their own.
pub fn attach_parent_console() {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
