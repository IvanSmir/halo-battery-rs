//! The tray application: a background thread reads the batteries, the main
//! thread owns the tray icons and runs the Win32 message loop.
//!
//! It shares two files with the settings window: it publishes every reading
//! to devices.json, and applies config.json whenever the window rewrites it.
//! - [`poller`]: the battery-reading thread
//! - [`view`]: devices and settings to icon states and tooltips (pure)
//! - [`alerts`]: when to notify (pure)
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
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT, TranslateMessage, WM_QUIT,
};

use self::alerts::{AlertKind, AlertTracker};
use self::menu::{Action, TrayMenu};
use self::poller::Poller;
use self::tray::Tray;
use self::view::Look;
use crate::config::Config;
use crate::device::DeviceStatus;
use crate::platform::{launch, notify, theme};
use crate::snapshot::Snapshot;
use crate::storage::FileWatch;

/// Frame time of the loop, and of the charging animation.
const TICK: Duration = Duration::from_millis(100);
/// How often the taskbar theme is re-read.
const THEME_CHECK: Duration = Duration::from_secs(5);
/// How often config.json is checked for changes from the settings window.
const CONFIG_CHECK: Duration = Duration::from_millis(500);

struct App {
    cfg: Config,
    config_watch: Option<FileWatch>,
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
        let menu = TrayMenu::new();
        let tray = Tray::new(menu.handle());
        let now = Instant::now();
        Self {
            cfg,
            config_watch: Config::default_path().map(|p| FileWatch::new(p, CONFIG_CHECK)),
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
        Look::new(&self.cfg, self.light_taskbar, self.started.elapsed().as_secs_f32())
    }

    fn redraw(&mut self) {
        self.tray.sync(&view::icons(&self.devices, &self.cfg, self.look()));
    }

    fn raise_alerts(&mut self) {
        for a in self.alerts.update(&self.devices, &self.cfg) {
            let (title, text) = match a.kind {
                AlertKind::Low => ("Batería baja", format!("{} está al {}%", a.name, a.level)),
                AlertKind::Full => ("Carga completa", format!("{} está al {}%", a.name, a.level)),
            };
            notify::toast(title, &text, self.cfg.notifications.sound);
        }
    }

    /// Takes in the newest readings and publishes them; `true` when there were any.
    fn take_readings(&mut self) -> bool {
        let mut received = false;
        while let Ok(devices) = self.readings.try_recv() {
            self.devices = devices;
            received = true;
        }
        if received {
            self.raise_alerts();
            if let Some(path) = Snapshot::default_path() {
                let _ = Snapshot { devices: self.devices.clone() }.save_to(&path);
            }
        }
        received
    }

    /// Applies config.json when the settings window changed it; `true` when it did.
    fn check_config(&mut self) -> bool {
        let Some(watch) = self.config_watch.as_mut() else { return false };
        if !watch.changed() {
            return false;
        }
        let new = Config::load_from(watch.path());
        let old = std::mem::replace(&mut self.cfg, new);
        if old.interval_secs != self.cfg.interval_secs {
            self.poller.set_interval(Duration::from_secs(self.cfg.interval_secs));
        }
        if old.low_threshold != self.cfg.low_threshold {
            self.alerts.reset();
            self.raise_alerts();
        }
        true
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
            Action::OpenSettings => {
                let _ = launch::open_settings();
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
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                self.apply(Action::OpenSettings);
            }
        }
        let readings = self.take_readings();
        let config = self.check_config();
        let theme = self.check_theme();
        let animating = self.cfg.appearance.animate && self.devices.iter().any(view::is_animated);
        if readings || config || theme || animating {
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
