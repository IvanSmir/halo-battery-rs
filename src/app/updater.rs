//! The background thread that looks for updates: shortly after start, then
//! once a day (sooner after a failure). It records what it finds in
//! update.json and sends the releases the user should be told about.

use std::sync::mpsc::Sender;
use std::thread;

use crate::config::Config;
use crate::platform::winrt;
use crate::update::source::Release;
use crate::update::state::{self, UpdateState};
use crate::update::{self, FIRST_CHECK};

/// Starts the thread; it ends once `out`'s receiver is dropped.
pub fn spawn(out: Sender<Release>) {
    thread::spawn(move || {
        winrt::init();
        let source = update::default_source();
        let current = update::current_version();
        thread::sleep(FIRST_CHECK);
        loop {
            // read every time: the settings window may have turned checks off or on
            let delay = if Config::load().updates.check {
                let result = source.latest();
                if let Ok(found) = &result
                    && let Some(path) = UpdateState::default_path()
                {
                    let mut st = UpdateState::load_from(&path);
                    let announce = state::record(&mut st, &current, found);
                    let _ = st.save_to(&path);
                    if announce && out.send(found.clone()).is_err() {
                        return;
                    }
                }
                update::next_check(&result)
            } else {
                // checks are off: look at the setting again in a while
                update::RETRY_AFTER
            };
            thread::sleep(delay);
        }
    });
}
