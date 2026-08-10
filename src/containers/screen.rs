//! BSN scene function for the full-screen root surface.
use bevy::ecs::component::Component;
use bevy::ecs::name::Name;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::ui::{
    AlignItems, Display, FlexDirection, LayoutConfig, Node, PositionType, Val, percent,
};

use crate::font_styles::InheritableFont;
use crate::theme::InheritableThemeTextToken;
use crate::tokens;

/// Marker component for screens()
#[derive(Clone, Default, Component)]
pub struct PlumeScreen;

/// Transparent, full-bleed column filling the viewport.
///
/// Alone among the containers it has neither padding nor gap.
///
/// [`Pickable::IGNORE`], so empty areas don't swallow picks.
pub fn screen() -> impl Scene {
    bsn! {
        PlumeScreen
        Name("PlumeScreen")
        TabGroup::new(0)
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            top: Val::ZERO,
            width: percent(100),
            height: percent(100),
        }
        Pickable::IGNORE
        InheritableThemeTextToken(tokens::TEXT_DIM)
        InheritableFont
        LayoutConfig {
            use_rounding: false,
        }
    }
}
