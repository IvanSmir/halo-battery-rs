//! Windows toast notifications.

use tauri_winrt_notification::{Sound, Toast};

/// The app is not installed with its own AppUserModelID, so it borrows PowerShell's.
fn toast_base(title: &str, text: &str, sound: bool) -> Toast {
    Toast::new(Toast::POWERSHELL_APP_ID).title(title).text1(text).sound(if sound { Some(Sound::Default) } else { None })
}

/// Shows a toast, with or without the notification sound. Failures are
/// ignored: a missed notification is not worth interrupting the app for.
pub fn toast(title: &str, text: &str, sound: bool) {
    let _ = toast_base(title, text, sound).show();
}

/// Shows a toast with one button; `on_click` runs when the button or the
/// toast itself is clicked.
pub fn toast_with_action(title: &str, text: &str, sound: bool, button: &str, on_click: impl Fn() + Send + 'static) {
    let _ = toast_base(title, text, sound)
        .add_button(button, "action")
        .on_activated(move |_| {
            on_click();
            Ok(())
        })
        .show();
}
