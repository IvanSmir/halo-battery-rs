//! Well-known folders, through the shell so redirections (e.g. a Desktop
//! moved into OneDrive) are honoured.

use std::path::PathBuf;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Desktop, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

/// The user's Desktop folder.
pub fn desktop() -> Option<PathBuf> {
    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None) }.ok()?;
    let result = unsafe { path.to_string() }.ok().map(PathBuf::from);
    unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
    result
}
