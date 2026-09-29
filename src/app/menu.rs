//! The context menu shared by every tray icon. It only builds the items and
//! translates clicks into [`Action`]s; applying them is up to the app. Every
//! setting lives in the settings window.

use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    OpenSettings,
    Refresh,
    Exit,
}

pub struct TrayMenu {
    menu: Menu,
    settings: MenuItem,
    refresh: MenuItem,
    exit: MenuItem,
}

impl TrayMenu {
    pub fn new() -> Self {
        let settings = MenuItem::new("Configuración…", true, None);
        let refresh = MenuItem::new("Actualizar ahora", true, None);
        let exit = MenuItem::new("Salir", true, None);
        let menu = Menu::new();
        let _ = menu.append_items(&[&settings, &refresh, &PredefinedMenuItem::separator(), &exit]);
        Self { menu, settings, refresh, exit }
    }

    /// A handle to the menu for one more tray icon; all handles share the items.
    pub fn handle(&self) -> Menu {
        self.menu.clone()
    }

    /// What a click on the item `id` asks for, if it is one of ours.
    pub fn action(&self, id: &MenuId) -> Option<Action> {
        [(&self.settings, Action::OpenSettings), (&self.refresh, Action::Refresh), (&self.exit, Action::Exit)]
            .into_iter()
            .find(|(item, _)| item.id() == id)
            .map(|(_, action)| action)
    }
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self::new()
    }
}
