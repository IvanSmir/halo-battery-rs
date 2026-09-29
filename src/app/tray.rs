//! Keeps the system tray in line with a list of [`IconView`]s: adds, updates
//! and removes icons, and only redraws what changed.

use std::collections::HashMap;

use tray_icon::menu::Menu;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use super::view::{IconView, icon_guid};
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
                    if let Some(tray) = self.add(&v.key, ic, &v.tooltip) {
                        let entry = Entry { tray, state: v.state, tooltip: v.tooltip.clone() };
                        self.entries.insert(v.key.clone(), entry);
                    }
                }
            }
        }
    }

    /// Adds one icon under a GUID derived from `key`, so Windows sees the same
    /// icon each time it comes back and keeps the user's pin. If Windows
    /// refuses the GUID, the icon is added without it: an unpinned icon beats
    /// no icon.
    fn add(&self, key: &str, icon: Icon, tooltip: &str) -> Option<TrayIcon> {
        let builder = |icon: Icon| {
            TrayIconBuilder::new().with_icon(icon).with_tooltip(tooltip).with_menu(Box::new(self.menu.clone()))
        };
        builder(icon.clone()).with_guid(icon_guid(key)).build().or_else(|_| builder(icon).build()).ok()
    }

    /// Removes every icon from the tray.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
