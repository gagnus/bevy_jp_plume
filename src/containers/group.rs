//! Bordered box for visually grouping related controls.
use bevy_ecs::{hierarchy::Children, reflect::ReflectComponent};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list};
use bevy_text::FontWeight;
use bevy_ui::{Display, FlexDirection, Node, UiRect};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::ThemeBackgroundColor,
    tokens,
};

/// A bordered box for visually grouping related controls.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeGroupProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeGroup;

/// Props used to construct a [`PlumeGroup`] scene.
pub struct PlumeGroupProps {
    /// Grouped content.
    pub contents: Box<dyn SceneList>,
}

impl Default for PlumeGroupProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list!()),
        }
    }
}

impl PlumeGroup {
    /// Scene function for a group.
    pub fn scene(props: PlumeGroupProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: size::GAP_TIGHT,
                padding: size::PAD,
                margin: {UiRect::bottom(size::GAP)},
                border_radius: {RoundedCorners::All.to_border_radius(size::CORNER_RADIUS)}
            }
            PlumeGroup
            ThemeBackgroundColor(tokens::GROUP_BG)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                {props.contents}
            ]
        }
    }
}
