//! Plume: debug and editor UI for bevy games. Themed `bevy_ui` controls with an
//! immediate-mode front end, in the spirit of egui and Dear ImGui. The controls
//! began as a fork of `bevy_feathers`, and the immediate mode is built on
//! [bevy_immediate](https://github.com/PPakalns/bevy_immediate) by Pēteris Pakalns.
//!
//! To get going, add [`PlumePlugins`], bring in the [`prelude`], and write a system
//! that takes a [`PlumeRoot`](imm::PlumeRoot). The [`imm`] page walks through a
//! first dialog.
//!
//! - [`imm`] is the everyday API: call widgets from a system, every frame.
//! - [`retained`] is what `imm` is built from - the widgets themselves, as `bsn!`
//!   scenes. Reach for it to build something `imm` doesn't offer.
//! - [`theme`] is color. One small palette drives every control, so a whole new
//!   look is a few numbers.
//! - [`style`] is shape: sizes, fonts, icons and the button variants.
//!
//! Every control looks after its own value and works as soon as it is dropped into
//! a dialog, with nothing extra to wire up.

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
pub(crate) use utils::{
    body, constants, cursor, default_width, focus, font_styles, rounded_corners, set_value,
};

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
        embedded_asset!(app, "assets/fonts/Lucide.ttf");

        app.add_plugins((
            ControlsPlugin,
            imm::ImmPlugin,
            UiAnimPlugin,
            containers::DialogPlugin,
            containers::ModalPlugin,
            containers::PopupPlugin,
            display::TooltipPlugin,
            containers::ReorderablePlugin,
            containers::ScrollAreaPlugin,
            containers::SectionPlugin,
            display::SeparatorPlugin,
            containers::SplitterPlugin,
            containers::TabsPlugin,
        ));
        app.add_plugins((
            CursorIconPlugin,
            default_width::DefaultWidthPlugin,
            set_value::SetValuePlugin,
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

// Compiles the README's code blocks under `cargo test --doc`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
