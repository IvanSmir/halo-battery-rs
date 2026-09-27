//! The tray application: a background thread reads the batteries, the main
//! thread owns one tray icon per device and runs the Win32 message loop.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use tauri_winrt_notification::Toast;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Gaming::Input::RawGameController;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    QS_ALLINPUT, WM_QUIT,
};

use crate::config::{Config, INTERVALS, THRESHOLDS};
use crate::device::{DeviceStatus, Kind, Provider};
use crate::icon::{self, IconState};
use crate::providers::{gamepad::GamepadProvider, logitech::LogitechProvider};
use crate::winutil;

/// Frame time of the loop, and of the charging animation.
const TICK: Duration = Duration::from_millis(100);
/// One "breath" of the charging animation.
const BREATH_PERIOD: f32 = 3.0;
/// How often the taskbar theme is re-read.
const THEME_CHECK: Duration = Duration::from_secs(5);
/// A low-battery alert is re-armed once the level climbs this far above the threshold.
const ALERT_HYSTERESIS: u8 = 5;
/// Key of the placeholder icon shown while no device is found.
const NONE_KEY: &str = "__none__";

enum Cmd {
    PollNow,
    SetInterval(u64),
}

pub fn all_providers() -> Vec<Box<dyn Provider>> {
    vec![Box::new(LogitechProvider::new()), Box::new(GamepadProvider::new())]
}

pub fn init_winrt() {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}

/// Reads every provider now, then again every `interval` seconds or on request.
fn spawn_poller(interval: u64, out: Sender<Vec<DeviceStatus>>) -> Sender<Cmd> {
    let (tx, rx) = mpsc::channel::<Cmd>();
    let events_tx = tx.clone();
    thread::spawn(move || {
        init_winrt();
        let mut providers = all_providers();
        // a controller switched on or off: read at once instead of waiting for the next poll
        let added = events_tx.clone();
        let _ = RawGameController::RawGameControllerAdded(&windows::Foundation::EventHandler::new(
            move |_, _| {
                let _ = added.send(Cmd::PollNow);
                Ok(())
            },
        ));
        let removed = events_tx;
        let _ = RawGameController::RawGameControllerRemoved(&windows::Foundation::EventHandler::new(
            move |_, _| {
                let _ = removed.send(Cmd::PollNow);
                Ok(())
            },
        ));
        // Windows.Gaming.Input fills its controller list asynchronously
        thread::sleep(Duration::from_millis(1500));
        let mut interval = Duration::from_secs(interval);
        loop {
            let devices: Vec<DeviceStatus> = providers.iter_mut().flat_map(|p| p.poll()).collect();
            if out.send(devices).is_err() {
                return;
            }
            let deadline = Instant::now() + interval;
            loop {
                match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(Cmd::PollNow) | Err(RecvTimeoutError::Timeout) => break,
                    Ok(Cmd::SetInterval(s)) => {
                        interval = Duration::from_secs(s);
                        break;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    });
    tx
}

fn tooltip(d: &DeviceStatus) -> String {
    match (d.level, d.online, d.charging) {
        (None, _, _) => format!("{}: nivel desconocido", d.name),
        (Some(l), false, _) => format!("{}: dormido (último {l}%)", d.name),
        (Some(l), true, true) => format!("{}: {l}% (cargando)", d.name),
        (Some(l), true, false) => format!("{}: {l}%", d.name),
    }
}

struct Entry {
    tray: TrayIcon,
    state: IconState,
    tooltip: String,
}

struct MenuIds {
    refresh: MenuId,
    exit: MenuId,
    autostart: CheckMenuItem,
    intervals: Vec<(u64, CheckMenuItem)>,
    thresholds: Vec<(u8, CheckMenuItem)>,
}

fn build_menu(cfg: &Config) -> (Menu, MenuIds) {
    let menu = Menu::new();
    let refresh = MenuItem::new("Actualizar ahora", true, None);
    let interval_menu = Submenu::new("Intervalo de lectura", true);
    let intervals: Vec<(u64, CheckMenuItem)> = INTERVALS
        .iter()
        .map(|&s| {
            let label = if s < 60 { format!("{s} s") } else { format!("{} min", s / 60) };
            (s, CheckMenuItem::new(label, true, s == cfg.interval_secs, None))
        })
        .collect();
    for (_, item) in &intervals {
        let _ = interval_menu.append(item);
    }
    let threshold_menu = Submenu::new("Aviso de batería baja", true);
    let thresholds: Vec<(u8, CheckMenuItem)> = THRESHOLDS
        .iter()
        .map(|&t| (t, CheckMenuItem::new(format!("{t}%"), true, t == cfg.low_threshold, None)))
        .collect();
    for (_, item) in &thresholds {
        let _ = threshold_menu.append(item);
    }
    let autostart = CheckMenuItem::new("Iniciar con Windows", true, winutil::autostart_enabled(), None);
    let exit = MenuItem::new("Salir", true, None);
    let _ = menu.append_items(&[
        &refresh,
        &PredefinedMenuItem::separator(),
        &interval_menu,
        &threshold_menu,
        &autostart,
        &PredefinedMenuItem::separator(),
        &exit,
    ]);
    let ids = MenuIds { refresh: refresh.id().clone(), exit: exit.id().clone(), autostart, intervals, thresholds };
    (menu, ids)
}

struct App {
    cfg: Config,
    menu: Menu,
    ids: MenuIds,
    poller: Sender<Cmd>,
    devices: Vec<DeviceStatus>,
    entries: HashMap<String, Entry>,
    /// Devices whose low-battery alert already fired in this discharge cycle.
    alerted: HashSet<String>,
    light: bool,
    started: Instant,
}

impl App {
    fn icon_state(&self, d: &DeviceStatus, now: f32) -> IconState {
        let pulse = if d.charging && d.online {
            icon::breath_level((now / BREATH_PERIOD).fract())
        } else {
            1.0
        };
        IconState {
            level: d.level,
            charging: d.charging,
            online: d.online,
            kind: d.kind,
            low: self.cfg.low_threshold,
            light_taskbar: self.light,
            pulse,
        }
    }

    /// Brings the tray icons in line with `self.devices`.
    fn refresh_icons(&mut self) {
        let now = self.started.elapsed().as_secs_f32();
        let mut shown: Vec<DeviceStatus> = self.devices.clone();
        if shown.is_empty() {
            shown.push(DeviceStatus {
                key: NONE_KEY.into(),
                name: "Halo Battery".into(),
                level: None,
                charging: false,
                online: false,
                kind: Kind::Mouse,
            });
        }
        let keys: HashSet<&str> = shown.iter().map(|d| d.key.as_str()).collect();
        self.entries.retain(|k, _| keys.contains(k.as_str()));

        for d in &shown {
            let state = self.icon_state(d, now);
            let tip = if d.key == NONE_KEY {
                "Halo Battery: ningún dispositivo encontrado".to_string()
            } else {
                tooltip(d)
            };
            match self.entries.get_mut(&d.key) {
                Some(e) => {
                    if e.state != state {
                        if let Ok(ic) = Icon::from_rgba(icon::render(&state), icon::SIZE, icon::SIZE) {
                            let _ = e.tray.set_icon(Some(ic));
                        }
                        e.state = state;
                    }
                    if e.tooltip != tip {
                        let _ = e.tray.set_tooltip(Some(&tip));
                        e.tooltip = tip;
                    }
                }
                None => {
                    let Ok(ic) = Icon::from_rgba(icon::render(&state), icon::SIZE, icon::SIZE) else {
                        continue;
                    };
                    let tray = TrayIconBuilder::new()
                        .with_icon(ic)
                        .with_tooltip(&tip)
                        .with_menu(Box::new(self.menu.clone()))
                        .build();
                    if let Ok(tray) = tray {
                        self.entries.insert(d.key.clone(), Entry { tray, state, tooltip: tip });
                    }
                }
            }
        }
    }

    fn check_alerts(&mut self) {
        let low = self.cfg.low_threshold;
        for d in &self.devices {
            let Some(level) = d.level else { continue };
            if d.charging || level > low + ALERT_HYSTERESIS {
                self.alerted.remove(&d.key);
            } else if d.online && level <= low && self.alerted.insert(d.key.clone()) {
                let _ = Toast::new(Toast::POWERSHELL_APP_ID)
                    .title("Batería baja")
                    .text1(&format!("{} está al {level}%", d.name))
                    .show();
            }
        }
    }

    /// Returns `false` when the app should quit.
    fn on_menu(&mut self, ev: MenuEvent) -> bool {
        let id = ev.id;
        if id == self.ids.exit {
            return false;
        }
        if id == self.ids.refresh {
            let _ = self.poller.send(Cmd::PollNow);
        } else if &id == self.ids.autostart.id() {
            winutil::set_autostart(self.ids.autostart.is_checked());
            self.ids.autostart.set_checked(winutil::autostart_enabled());
        } else if let Some(&(secs, _)) = self.ids.intervals.iter().find(|(_, i)| i.id() == &id) {
            self.cfg.interval_secs = secs;
            for (s, item) in &self.ids.intervals {
                item.set_checked(*s == secs);
            }
            self.cfg.save();
            let _ = self.poller.send(Cmd::SetInterval(secs));
        } else if let Some(&(thr, _)) = self.ids.thresholds.iter().find(|(_, i)| i.id() == &id) {
            self.cfg.low_threshold = thr;
            for (t, item) in &self.ids.thresholds {
                item.set_checked(*t == thr);
            }
            self.cfg.save();
            self.alerted.clear();
            self.check_alerts();
            self.refresh_icons();
        }
        true
    }
}

/// Pumps pending Win32 messages; `false` once WM_QUIT arrives.
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

pub fn run() {
    let cfg = Config::load();
    let (dev_tx, dev_rx): (Sender<Vec<DeviceStatus>>, Receiver<Vec<DeviceStatus>>) = mpsc::channel();
    let poller = spawn_poller(cfg.interval_secs, dev_tx);
    let (menu, ids) = build_menu(&cfg);
    let mut app = App {
        cfg,
        menu,
        ids,
        poller,
        devices: Vec::new(),
        entries: HashMap::new(),
        alerted: HashSet::new(),
        light: winutil::taskbar_is_light(),
        started: Instant::now(),
    };
    app.refresh_icons();
    let mut theme_checked = Instant::now();

    loop {
        if !pump_messages() {
            break;
        }
        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if !app.on_menu(ev) {
                break;
            }
        }
        let mut changed = false;
        while let Ok(devices) = dev_rx.try_recv() {
            app.devices = devices;
            changed = true;
        }
        if changed {
            app.check_alerts();
        }
        if theme_checked.elapsed() >= THEME_CHECK {
            theme_checked = Instant::now();
            let light = winutil::taskbar_is_light();
            if light != app.light {
                app.light = light;
                changed = true;
            }
        }
        let animating = app.devices.iter().any(|d| d.charging && d.online);
        if changed || animating {
            app.refresh_icons();
        }
        unsafe {
            MsgWaitForMultipleObjects(None, false, TICK.as_millis() as u32, QS_ALLINPUT);
        }
    }
    // dropping the icons removes them from the tray
    app.entries.clear();
}
