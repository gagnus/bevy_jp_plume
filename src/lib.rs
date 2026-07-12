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
use bevy_text::{TextColor, TextFont};
use bevy_ui::UiSystems;
use bevy_ui_render::UiMaterialPlugin;

use crate::{
    alpha_pattern::{AlphaPatternMaterial, AlphaPatternResource},
    controls::ControlsPlugin,
    cursor::{CursorIconPlugin, DefaultCursor, EntityCursor},
    theme::{ThemedText, UiTheme},
};

mod alpha_pattern;
pub mod containers;
pub mod controls;
pub mod display;
pub mod theme;
pub mod utils;

pub use theme::{dark_theme, light_theme, tokens};
pub use utils::{constants, cursor, font_styles, rounded_corners};

/// Plugin which installs observers and systems for plume themes, cursors, and all controls.
pub struct PlumeCorePlugin;

impl Plugin for PlumeCorePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<UiTheme>();

        // Embedded font
        embedded_asset!(app, "assets/fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-BoldItalic.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Italic.ttf");
        embedded_asset!(app, "assets/fonts/FiraMono-Medium.ttf");

        // Embedded icons
        embedded_asset!(app, "assets/icons/chevron-down.png");
        embedded_asset!(app, "assets/icons/chevron-right.png");
        embedded_asset!(app, "assets/icons/x.png");

        // Embedded shader
        embedded_asset!(app, "assets/shaders/alpha_pattern.wgsl");

        app.add_plugins((
            ControlsPlugin,
            CursorIconPlugin,
            HierarchyPropagatePlugin::<TextColor, With<ThemedText>>::new(PostUpdate),
            HierarchyPropagatePlugin::<TextFont, With<ThemedText>>::new(PostUpdate),
            UiMaterialPlugin::<AlphaPatternMaterial>::default(),
        ));

        // This needs to run in UiSystems::Propagate so the fonts are up-to-date for `measure_text_system`
        // and `detect_text_needs_rerender` in UiSystems::Content
        app.configure_sets(
            PostUpdate,
            PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
        );

        app.insert_resource(DefaultCursor(EntityCursor::System(
            bevy_window::SystemCursorIcon::Default,
        )));

        app.add_systems(
            PostUpdate,
            (
                theme::update_theme,
                display::update_themed_icons.after(PropagateSet::<TextColor>::default()),
            ),
        )
        .add_observer(theme::on_changed_background)
        .add_observer(theme::on_changed_border)
        .add_observer(theme::on_changed_font_color)
        .add_observer(theme::on_changed_text_color)
        .add_observer(font_styles::on_changed_font)
        // Click-to-focus resolver for `TabIndex` targets. Deliberately not
        // `TabNavigationPlugin`, which would also install Tab-key navigation.
        .add_observer(bevy_input_focus::tab_navigation::acquire_focus_tab_index);

        app.init_resource::<AlphaPatternResource>();
    }
}

/// A plugin group that adds all dependencies for Plume
pub struct PlumePlugins;

impl PluginGroup for PlumePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(PlumeCorePlugin)
    }
}
