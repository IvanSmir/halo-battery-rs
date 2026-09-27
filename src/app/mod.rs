//! The tray application: a background thread reads the batteries, the main
//! thread owns the tray icons and runs the Win32 message loop.
//! - [`poller`]: the battery-reading thread
//! - [`view`]: device status to icon state and tooltip (pure)
//! - [`alerts`]: when to warn about a low battery (pure)
//! - [`menu`]: the context menu, as [`menu::Action`]s
//! - [`tray`]: keeps the system tray in line with the views

pub mod alerts;
pub mod menu;
pub mod poller;
pub mod tray;
pub mod view;

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use tray_icon::menu::MenuEvent;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT, TranslateMessage, WM_QUIT,
};

use self::alerts::AlertTracker;
use self::menu::{Action, TrayMenu};
use self::poller::Poller;
use self::tray::Tray;
use self::view::Look;
use crate::config::Config;
use crate::device::DeviceStatus;
use crate::platform::{autostart, notify, theme};

/// Frame time of the loop, and of the charging animation.
const TICK: Duration = Duration::from_millis(100);
/// How often the taskbar theme is re-read.
const THEME_CHECK: Duration = Duration::from_secs(5);

struct App {
    cfg: Config,
    menu: TrayMenu,
    tray: Tray,
    poller: Poller,
    readings: Receiver<Vec<DeviceStatus>>,
    alerts: AlertTracker,
    devices: Vec<DeviceStatus>,
    light_taskbar: bool,
    theme_checked: Instant,
    started: Instant,
}

impl App {
    fn new() -> Self {
        let cfg = Config::load();
        let (tx, readings) = mpsc::channel();
        let poller = Poller::spawn(Duration::from_secs(cfg.interval_secs), tx);
        let menu = TrayMenu::new(&cfg, autostart::is_enabled());
        let tray = Tray::new(menu.handle());
        let now = Instant::now();
        Self {
            cfg,
            menu,
            tray,
            poller,
            readings,
            alerts: AlertTracker::new(),
            devices: Vec::new(),
            light_taskbar: theme::taskbar_is_light(),
            theme_checked: now,
            started: now,
        }
    }

    fn look(&self) -> Look {
        Look {
            low: self.cfg.low_threshold,
            light_taskbar: self.light_taskbar,
            time: self.started.elapsed().as_secs_f32(),
        }
    }

    fn redraw(&mut self) {
        self.tray.sync(&view::icons(&self.devices, self.look()));
    }

    fn raise_alerts(&mut self) {
        for a in self.alerts.update(&self.devices, self.cfg.low_threshold) {
            notify::toast("Batería baja", &format!("{} está al {}%", a.name, a.level));
        }
    }

    /// Takes in the newest readings; `true` when there were any.
    fn take_readings(&mut self) -> bool {
        let mut changed = false;
        while let Ok(devices) = self.readings.try_recv() {
            self.devices = devices;
            changed = true;
        }
        if changed {
            self.raise_alerts();
        }
        changed
    }

    /// Re-reads the taskbar theme now and then; `true` when it changed.
    fn check_theme(&mut self) -> bool {
        if self.theme_checked.elapsed() < THEME_CHECK {
            return false;
        }
        self.theme_checked = Instant::now();
        let light = theme::taskbar_is_light();
        let changed = light != self.light_taskbar;
        self.light_taskbar = light;
        changed
    }

    /// Applies a menu action; `false` when the app should quit.
    fn apply(&mut self, action: Action) -> bool {
        match action {
            Action::Exit => return false,
            Action::Refresh => self.poller.poll_now(),
            Action::SetAutostart(enable) => {
                autostart::set_enabled(enable);
                self.menu.show_autostart(autostart::is_enabled());
            }
            Action::SetInterval(secs) => {
                self.cfg.interval_secs = secs;
                self.cfg.save();
                self.menu.show_interval(secs);
                self.poller.set_interval(Duration::from_secs(secs));
            }
            Action::SetThreshold(low) => {
                self.cfg.low_threshold = low;
                self.cfg.save();
                self.menu.show_threshold(low);
                self.alerts.reset();
                self.raise_alerts();
                self.redraw();
            }
        }
        true
    }

    /// One turn of the loop; `false` when the app should quit.
    fn tick(&mut self) -> bool {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let Some(action) = self.menu.action(&ev.id) else { continue };
            if !self.apply(action) {
                return false;
            }
        }
        let readings = self.take_readings();
        let theme = self.check_theme();
        if readings || theme || self.devices.iter().any(view::is_animated) {
            self.redraw();
        }
        true
    }
}

/// Handles pending Win32 messages; `false` once WM_QUIT arrives.
fn pump_messages() -> bool {
    let mut msg = MSG::default();
    unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return false;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    true
}

/// Sleeps until a Win32 message arrives or one tick passes.
fn wait_for_messages() {
    unsafe {
        MsgWaitForMultipleObjects(None, false, TICK.as_millis() as u32, QS_ALLINPUT);
    }
}

pub fn run() {
    let mut app = App::new();
    app.redraw();
    while pump_messages() && app.tick() {
        wait_for_messages();
    }
    app.tray.clear();
}
