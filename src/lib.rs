//! Plume: styled, themed bevy_ui controls for game editors, forked from `bevy_feathers`.
//!
//! Controls self-update their own value and work when simply dropped into a dialog.
//! The parametric theme pipeline (palette → slot → token) styles everything.

extern crate alloc;

use bevy_app::{
    HierarchyPropagatePlugin, Plugin, PluginGroup, PluginGroupBuilder, PostUpdate, PropagateSet,
};
use bevy_asset::embedded_asset;
use bevy_ecs::{query::With, schedule::IntoScheduleConfigs};
use bevy_text::TextFont;
use bevy_ui::UiSystems;

use crate::{
    controls::ControlsPlugin,
    cursor::{CursorIconPlugin, DefaultCursor, EntityCursor},
    theme::{ThemePlugin, ThemedText},
    utils::anim::UiAnimPlugin,
};

pub mod containers;
pub mod controls;
pub mod display;
pub mod imm;
pub mod theme;
pub mod utils;

pub use theme::{dark_theme, light_theme, tokens};
pub use utils::{constants, cursor, focus, font_styles, rounded_corners};

// Marks a tree as a Tab-traversal scope; `PlumeDialog` carries one, app-built
// root panels add their own. Re-exported so apps stay on the plume surface.
pub use bevy_input_focus::tab_navigation::TabGroup;

/// Plugin which installs observers and systems for plume themes, cursors, and all controls.
pub struct PlumeCorePlugin;

impl Plugin for PlumeCorePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        embedded_asset!(app, "assets/fonts/NotoSans-Bold.ttf");
        embedded_asset!(app, "assets/fonts/NotoSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/NotoSansMono-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FontAwesome-Solid.otf");
        embedded_asset!(app, "assets/fonts/FontAwesome-Regular.otf");

        app.add_plugins((
            ControlsPlugin,
            imm::ImmPlugin,
            UiAnimPlugin,
            containers::SectionPlugin,
            CursorIconPlugin,
            ThemePlugin,
            // Click-to-focus plus Tab/Shift-Tab traversal of every control; tabbable
            // entities need a `TabGroup` ancestor, which `PlumeDialog` provides.
            bevy_input_focus::tab_navigation::TabNavigationPlugin,
            focus::FocusPlugin,
            HierarchyPropagatePlugin::<TextFont, With<ThemedText>>::new(PostUpdate),
        ));

        // Fonts must be current before `measure_text_system` and
        // `detect_text_needs_rerender` run in `UiSystems::Content`.
        app.configure_sets(
            PostUpdate,
            PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
        );

        app.insert_resource(DefaultCursor(EntityCursor::System(
            bevy_window::SystemCursorIcon::Default,
        )));

        app.add_observer(font_styles::on_changed_font);
    }
}

/// A plugin group that adds all dependencies for Plume
pub struct PlumePlugins;

impl PluginGroup for PlumePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(PlumeCorePlugin)
    }
}
