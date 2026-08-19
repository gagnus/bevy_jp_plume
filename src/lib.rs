//! Plume: styled, themed bevy::ui controls for game editors, forked from `bevy_feathers`.
//!
//! Controls self-update their own value and work when simply dropped into a dialog.
//! The parametric theme pipeline (palette → slot → token) styles everything.

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![warn(missing_docs)]

extern crate alloc;

use bevy::app::{Plugin, PluginGroup, PluginGroupBuilder, PostUpdate, PropagateSet};
use bevy::asset::embedded_asset;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::text::TextFont;
// Short crate-internal paths for the modules the public surface re-exports
// piecemeal through `style`, `retained` and `theme`.
pub(crate) use theme::tokens;
pub use utils::cursor::{CursorLock, DefaultCursor, EntityCursor, OverrideCursor};
pub(crate) use utils::{constants, cursor, focus, font_styles, rounded_corners};

use crate::controls::ControlsPlugin;
use crate::theme::ThemePlugin;
use crate::utils::anim::UiAnimPlugin;
use crate::utils::cursor::CursorIconPlugin;

mod containers;
mod controls;
mod display;
mod utils;

pub mod imm;
pub mod prelude;
pub mod retained;
pub mod style;
pub mod theme;

// Plugin which installs observers and systems for plume themes, cursors, and all controls.
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
            containers::DialogPlugin,
            containers::PopupPlugin,
            display::TooltipPlugin,
            containers::ScrollAreaPlugin,
            containers::SectionPlugin,
            containers::SeparatorPlugin,
            containers::SplitterPlugin,
            containers::TabsPlugin,
            CursorIconPlugin,
            ThemePlugin,
            // Click-to-focus plus Tab/Shift-Tab traversal of every control; tabbable
            // entities need a `TabGroup` ancestor, which `PlumeDialog` provides.
            bevy::input_focus::tab_navigation::TabNavigationPlugin,
            focus::FocusPlugin,
        ));

        app.insert_resource(DefaultCursor(EntityCursor::System(
            bevy::window::SystemCursorIcon::Default,
        )));

        // Before the set so it reads the `Inherited<TextFont>` of the previous run.
        app.add_systems(
            PostUpdate,
            font_styles::resolve_inheritable_font
                .in_set(font_styles::FontStyleSystems)
                .before(PropagateSet::<TextFont>::default()),
        );
        // `Val::Em` chrome: propagation lands a `TextFont` on every node under a
        // surface, and bevy derives `EmSize` from it; nodes above any surface
        // fall back to `RemSize`, so it must hold the standard size.
        app.insert_resource(bevy::text::RemSize(constants::size::MEDIUM_FONT_PX));
    }
}

/// A plugin group that adds all dependencies for Plume.
pub struct PlumePlugins;

impl PluginGroup for PlumePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(PlumeCorePlugin)
    }
}
