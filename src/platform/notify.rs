//! Windows toast notifications.

use tauri_winrt_notification::{Sound, Toast};

/// Shows a toast, with or without the notification sound. Failures are
/// ignored: a missed notification is not worth interrupting the app for.
pub fn toast(title: &str, text: &str, sound: bool) {
    // the app is not installed with its own AppUserModelID, so borrow PowerShell's
    let _ = Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(text)
        .sound(if sound { Some(Sound::Default) } else { None })
        .show();
}
