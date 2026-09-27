//! Keeps the system tray in line with a list of [`IconView`]s: adds, updates
//! and removes icons, and only redraws what changed.

use std::collections::HashMap;

use tray_icon::menu::Menu;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use super::view::IconView;
use crate::icon::{self, IconState};

struct Entry {
    tray: TrayIcon,
    state: IconState,
    tooltip: String,
}

pub struct Tray {
    menu: Menu,
    entries: HashMap<String, Entry>,
}

fn build_icon(state: &IconState) -> Option<Icon> {
    Icon::from_rgba(icon::render(state), icon::SIZE, icon::SIZE).ok()
}

impl Tray {
    /// `menu` is attached to every icon.
    pub fn new(menu: Menu) -> Self {
        Self { menu, entries: HashMap::new() }
    }

    pub fn sync(&mut self, views: &[IconView]) {
        self.entries.retain(|key, _| views.iter().any(|v| &v.key == key));
        for v in views {
            match self.entries.get_mut(&v.key) {
                Some(e) => {
                    if e.state != v.state {
                        if let Some(ic) = build_icon(&v.state) {
                            let _ = e.tray.set_icon(Some(ic));
                        }
                        e.state = v.state;
                    }
                    if e.tooltip != v.tooltip {
                        let _ = e.tray.set_tooltip(Some(&v.tooltip));
                        e.tooltip.clone_from(&v.tooltip);
                    }
                }
                None => {
                    let Some(ic) = build_icon(&v.state) else { continue };
                    let tray = TrayIconBuilder::new()
                        .with_icon(ic)
                        .with_tooltip(&v.tooltip)
                        .with_menu(Box::new(self.menu.clone()))
                        .build();
                    if let Ok(tray) = tray {
                        let entry = Entry { tray, state: v.state, tooltip: v.tooltip.clone() };
                        self.entries.insert(v.key.clone(), entry);
                    }
                }
            }
        }
    }

    /// Removes every icon from the tray.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
