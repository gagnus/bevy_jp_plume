//! Debug dialog registry plus a toolbar that toggles each entry: a feature plugin
//! registers a titled dialog with [`AddDebugDialog::add_debug_dialog`].
use std::collections::BTreeMap;

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

/// Open/closed state for every registered dialog, keyed by title.
#[derive(Resource, Default)]
pub struct DebugDialogRegistry {
    set_all_open: bool,
    dialogs: BTreeMap<&'static str, (FaIcon, bool)>,
}

impl DebugDialogRegistry {
    pub fn set_all_open(&mut self) {
        self.set_all_open = true;
    }

    /// Register a dialog with its initial open state. First registration wins.
    pub fn register(&mut self, title: &'static str, icon: FaIcon, open: bool) {
        self.dialogs
            .entry(title)
            .or_insert((icon, open || self.set_all_open));
    }

    /// Whether the named dialog is open (false if unregistered).
    pub fn is_open(&self, title: &'static str) -> bool {
        self.dialogs
            .get(title)
            .copied()
            .map(|(_, open)| open)
            .unwrap_or(false)
    }

    /// Set the named dialog's open state.
    pub fn set_open(&mut self, title: &'static str, open: bool) {
        if let Some(state) = self.dialogs.get_mut(title) {
            state.1 = open;
        }
    }

    fn entries(&self) -> Vec<(&'static str, (FaIcon, bool))> {
        self.dialogs.iter().map(|(&k, &v)| (k, v)).collect()
    }
}

/// Registers the [`DebugDialogRegistry`] and the toolbar that toggles its entries.
pub struct DebugDialogHubPlugin;

impl Plugin for DebugDialogHubPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugDialogRegistry>()
            .add_systems(Update, debug_hub_ui);
    }
}

/// Register a feature's dialog on the [`App`]: a [`DebugDialogRegistry`] entry (so the hub
/// shows a toggle for it) plus its `system` in `Update`.
pub trait AddDebugDialog {
    fn add_debug_dialog<M>(
        &mut self,
        title: &'static str,
        icon: FaIcon,
        open: bool,
        system: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl AddDebugDialog for App {
    fn add_debug_dialog<M>(
        &mut self,
        title: &'static str,
        icon: FaIcon,
        open: bool,
        system: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<DebugDialogRegistry>()
            .register(title, icon, open);
        self.add_systems(Update, system);
        self
    }
}

fn debug_hub_ui(mut root: PlumeRoot, mut registry: ResMut<DebugDialogRegistry>) {
    let entries = registry.entries();
    if entries.is_empty() {
        return;
    }

    let mut toggled: Vec<&'static str> = vec![];
    root.panel().at(px(16), px(16)).show(|ui| {
        ui.horizontal(|ui| {
            for &(title, (icon, mut is_open)) in &entries {
                if ui
                    .tool_button(icon)
                    .flat()
                    .checkable(&mut is_open)
                    .variant(ButtonToggleVariant::Plain)
                    .tooltip(title)
                    .clicked
                {
                    toggled.push(title);
                }
            }
        });
    });
    for title in toggled {
        let now = registry.is_open(title);
        registry.set_open(title, !now);
    }
}
