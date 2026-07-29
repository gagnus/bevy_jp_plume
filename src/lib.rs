//! Plume: styled, themed bevy::ui controls for game editors, forked from `bevy_feathers`.
//!
//! Controls self-update their own value and work when simply dropped into a dialog.
//! The parametric theme pipeline (palette → slot → token) styles everything.

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![warn(missing_docs)]

extern crate alloc;

use bevy::app::{
    HierarchyPropagatePlugin, Plugin, PluginGroup, PluginGroupBuilder, PostUpdate, PropagateSet,
};
use bevy::asset::embedded_asset;
use bevy::ecs::{query::With, schedule::IntoScheduleConfigs};
use bevy::text::TextFont;
use bevy::ui::UiSystems;

use crate::{
    controls::ControlsPlugin,
    theme::{ThemePlugin, ThemedText, on_themed_text_inserted},
    utils::{anim::UiAnimPlugin, cursor::CursorIconPlugin},
};

pub use utils::cursor::{DefaultCursor, EntityCursor, OverrideCursor};

// Short crate-internal paths for the modules the public surface re-exports
// piecemeal through `style`, `retained` and `theme`.
pub(crate) use theme::tokens;
pub(crate) use utils::{constants, cursor, focus, font_styles, rounded_corners};

mod containers;
mod controls;
mod display;
mod utils;

pub mod imm;
pub mod prelude;
pub mod retained;
pub mod style;
pub mod theme;

/// Plugin which installs observers and systems for plume themes, cursors, and all controls.
pub(crate) struct PlumeCorePlugin;

impl Plugin for PlumeCorePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        embedded_asset!(app, "assets/fonts/NotoSans-Bold.ttf");
        embedded_asset!(app, "assets/fonts/NotoSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/NotoSansMono-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FontAwesome-Solid.otf");
        embedded_asset!(app, "assets/fonts/FontAwesome-Regular.otf");

        app.add_plugins((
            ControlsPlugin,
            imm::ImmPlugin,
            UiAnimPlugin,
            containers::PopupPlugin,
            containers::ScrollAreaPlugin,
            containers::SectionPlugin,
            containers::TabsPlugin,
            CursorIconPlugin,
            ThemePlugin,
            // Click-to-focus plus Tab/Shift-Tab traversal of every control; tabbable
            // entities need a `TabGroup` ancestor, which `PlumeDialog` provides.
            bevy::input_focus::tab_navigation::TabNavigationPlugin,
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
            bevy::window::SystemCursorIcon::Default,
        )));

        app.add_observer(font_styles::on_changed_font);
        // Companion to the `TextColor` registration in `ThemePlugin`.
        app.add_observer(on_themed_text_inserted::<TextFont>);
    }
}

/// A plugin group that adds all dependencies for Plume
pub struct PlumePlugins;

impl PluginGroup for PlumePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(PlumeCorePlugin)
    }
}
