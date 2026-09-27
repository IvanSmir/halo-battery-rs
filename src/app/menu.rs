//! The context menu shared by every tray icon. It only builds the items and
//! translates clicks into [`Action`]s; applying them is up to the app.

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use crate::config::{Config, INTERVALS, THRESHOLDS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Refresh,
    SetInterval(u64),
    SetThreshold(u8),
    /// The autostart item was clicked; its new checked state.
    SetAutostart(bool),
    Exit,
}

pub struct TrayMenu {
    menu: Menu,
    refresh: MenuItem,
    exit: MenuItem,
    autostart: CheckMenuItem,
    intervals: Vec<(u64, CheckMenuItem)>,
    thresholds: Vec<(u8, CheckMenuItem)>,
}

fn interval_label(secs: u64) -> String {
    if secs < 60 { format!("{secs} s") } else { format!("{} min", secs / 60) }
}

impl TrayMenu {
    pub fn new(cfg: &Config, autostart_enabled: bool) -> Self {
        let refresh = MenuItem::new("Actualizar ahora", true, None);
        let intervals: Vec<(u64, CheckMenuItem)> = INTERVALS
            .iter()
            .map(|&s| (s, CheckMenuItem::new(interval_label(s), true, s == cfg.interval_secs, None)))
            .collect();
        let thresholds: Vec<(u8, CheckMenuItem)> = THRESHOLDS
            .iter()
            .map(|&t| (t, CheckMenuItem::new(format!("{t}%"), true, t == cfg.low_threshold, None)))
            .collect();
        let autostart = CheckMenuItem::new("Iniciar con Windows", true, autostart_enabled, None);
        let exit = MenuItem::new("Salir", true, None);

        let interval_menu = Submenu::new("Intervalo de lectura", true);
        for (_, item) in &intervals {
            let _ = interval_menu.append(item);
        }
        let threshold_menu = Submenu::new("Aviso de batería baja", true);
        for (_, item) in &thresholds {
            let _ = threshold_menu.append(item);
        }
        let menu = Menu::new();
        let _ = menu.append_items(&[
            &refresh,
            &PredefinedMenuItem::separator(),
            &interval_menu,
            &threshold_menu,
            &autostart,
            &PredefinedMenuItem::separator(),
            &exit,
        ]);
        Self { menu, refresh, exit, autostart, intervals, thresholds }
    }

    /// A handle to the menu for one more tray icon; all handles share the items.
    pub fn handle(&self) -> Menu {
        self.menu.clone()
    }

    /// What a click on the item `id` asks for, if it is one of ours.
    pub fn action(&self, id: &MenuId) -> Option<Action> {
        if id == self.refresh.id() {
            return Some(Action::Refresh);
        }
        if id == self.exit.id() {
            return Some(Action::Exit);
        }
        if id == self.autostart.id() {
            return Some(Action::SetAutostart(self.autostart.is_checked()));
        }
        if let Some((secs, _)) = self.intervals.iter().find(|(_, i)| i.id() == id) {
            return Some(Action::SetInterval(*secs));
        }
        self.thresholds.iter().find(|(_, i)| i.id() == id).map(|(t, _)| Action::SetThreshold(*t))
    }

    /// Check marks follow the settings (a check item toggles itself when clicked).
    pub fn show_interval(&self, secs: u64) {
        for (s, item) in &self.intervals {
            item.set_checked(*s == secs);
        }
    }

    pub fn show_threshold(&self, low: u8) {
        for (t, item) in &self.thresholds {
            item.set_checked(*t == low);
        }
    }

    pub fn show_autostart(&self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }
}
