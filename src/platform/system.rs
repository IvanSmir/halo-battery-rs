//! Facts about the system, for the diagnostics report.

use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::core::{HSTRING, w};

fn current_version_string(value: &str) -> Option<String> {
    let mut buf = [0u16; 256];
    let mut size = size_of_val(&buf) as u32;
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            &HSTRING::from(value),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    }
    .ok()
    .ok()?;
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// E.g. "Windows 10 Pro 25H2 (build 26200)". Windows 11 still reports
/// itself as "Windows 10" in ProductName, so the build number is included.
pub fn windows_version() -> String {
    let product = current_version_string("ProductName").unwrap_or_else(|| "Windows".into());
    let display = current_version_string("DisplayVersion").unwrap_or_default();
    let build = current_version_string("CurrentBuild").unwrap_or_default();
    format!("{product} {display} (build {build})").replace("  ", " ")
}

/// The local date and time, "2026-09-29 14:05".
pub fn local_time() -> String {
    let t = unsafe { GetLocalTime() };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute)
}
