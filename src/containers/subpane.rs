//! Collapsible sub-pane container with a header bar.
use bevy_app::{Plugin, PreUpdate};
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_picking::{
    PickingSystems,
    events::{Click, Pointer},
};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on};
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, px, widget::ImageNode,
};

use crate::{
    constants::{fonts, icons, size},
    display::icon,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A sub-pane: a header bar over a body. Collapsible by default — clicking the
/// header folds the body away.
#[derive(SceneComponent, Clone, Reflect)]
#[scene(PlumeSubpaneProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSubpane {
    /// Whether clicking the header collapses/expands the body.
    pub collapsible: bool,
}

impl Default for PlumeSubpane {
    fn default() -> Self {
        Self { collapsible: true }
    }
}

/// Marker for a collapsed [`PlumeSubpane`]; insert it to start collapsed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SubpaneCollapsed;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SubpaneHeader;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SubpaneBody;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SubpaneChevron;

/// Props used to construct a [`PlumeSubpane`] scene.
pub struct PlumeSubpaneProps {
    /// Header bar content (e.g. `bsn! { caption("…") }`).
    pub header: Box<dyn SceneList>,
    /// Body content, folded away when collapsed.
    pub contents: Box<dyn SceneList>,
    /// Whether clicking the header collapses/expands the body.
    pub collapsible: bool,
}

impl Default for PlumeSubpaneProps {
    fn default() -> Self {
        Self {
            header: Box::new(bsn_list!()),
            contents: Box::new(bsn_list!()),
            collapsible: true,
        }
    }
}

impl PlumeSubpane {
    /// Scene function for a sub-pane.
    pub fn scene(props: PlumeSubpaneProps) -> impl Scene {
        let chevron: Box<dyn SceneList> = if props.collapsible {
            Box::new(bsn_list![(icon(icons::CHEVRON_DOWN) SubpaneChevron)])
        } else {
            Box::new(bsn_list!())
        };
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                margin: {UiRect::bottom(size::GAP)},
                align_items: AlignItems::Stretch,
            }
            PlumeSubpane { collapsible: {props.collapsible} }
            Children [
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        border: UiRect {
                            left: size::CONTAINER_BORDER,
                            top: size::CONTAINER_BORDER,
                            right: size::CONTAINER_BORDER,
                        },
                        padding: UiRect::horizontal(size::HEADER_PAD_X),
                        min_height: size::HEADER_HEIGHT,
                        column_gap: size::GAP_TIGHT,
                        border_radius: {RoundedCorners::Top.to_border_radius(size::CORNER_RADIUS)}
                    }
                    SubpaneHeader
                    ThemeBackgroundColor(tokens::SUBPANE_HEADER_BG)
                    ThemeBorderColor(tokens::SUBPANE_HEADER_BORDER)
                    InheritableThemeTextColor(tokens::SUBPANE_HEADER_TEXT)
                    InheritableFont {
                        font: fonts::REGULAR,
                        font_size: size::MEDIUM_FONT,
                        weight: FontWeight::NORMAL,
                    }
                    on(toggle_subpane_collapse)
                    Children [
                        {chevron},
                        {props.header}
                    ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        border: UiRect {
                            left: size::CONTAINER_BORDER,
                            right: size::CONTAINER_BORDER,
                            bottom: size::CONTAINER_BORDER,
                        },
                        row_gap: size::GAP_TIGHT,
                        padding: size::PAD,
                        border_radius: {RoundedCorners::Bottom.to_border_radius(size::CORNER_RADIUS)}
                    }
                    SubpaneBody
                    ThemeBackgroundColor(tokens::SUBPANE_BODY_BG)
                    ThemeBorderColor(tokens::SUBPANE_BODY_BORDER)
                    InheritableFont {
                        font: fonts::REGULAR,
                        font_size: size::MEDIUM_FONT,
                        weight: FontWeight::NORMAL,
                    }
                    Children [
                        {props.contents}
                    ]
                )
            ]
        }
    }
}

fn toggle_subpane_collapse(
    click: On<Pointer<Click>>,
    q_headers: Query<&ChildOf, With<SubpaneHeader>>,
    q_subpanes: Query<(&PlumeSubpane, Has<SubpaneCollapsed>)>,
    mut commands: Commands,
) {
    let Ok(child_of) = q_headers.get(click.event_target()) else {
        return;
    };
    let root = child_of.parent();
    let Ok((subpane, collapsed)) = q_subpanes.get(root) else {
        return;
    };
    if !subpane.collapsible {
        return;
    }
    if collapsed {
        commands.entity(root).remove::<SubpaneCollapsed>();
    } else {
        commands.entity(root).insert(SubpaneCollapsed);
    }
}

type SubpaneParts<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, Has<SubpaneBody>, Has<SubpaneHeader>),
    Or<(With<SubpaneBody>, With<SubpaneHeader>)>,
>;

fn update_subpane_collapse(
    q_collapsed: Query<Entity, (With<PlumeSubpane>, Added<SubpaneCollapsed>)>,
    mut removed: RemovedComponents<SubpaneCollapsed>,
    q_subpanes: Query<(), With<PlumeSubpane>>,
    q_children: Query<&Children>,
    mut q_parts: SubpaneParts,
    mut q_chevrons: Query<&mut ImageNode, With<SubpaneChevron>>,
    assets: Res<AssetServer>,
) {
    let mut apply = |root: Entity, collapsed: bool| {
        for descendant in q_children.iter_descendants(root) {
            if let Ok((mut node, is_body, is_header)) = q_parts.get_mut(descendant) {
                if is_body {
                    node.display = if collapsed {
                        Display::None
                    } else {
                        Display::Flex
                    };
                } else if is_header {
                    let corners = if collapsed {
                        RoundedCorners::All
                    } else {
                        RoundedCorners::Top
                    };
                    node.border_radius = corners.to_border_radius(size::CORNER_RADIUS);
                    node.border.bottom = if collapsed {
                        size::CONTAINER_BORDER
                    } else {
                        px(0)
                    };
                }
            }
            if let Ok(mut chevron) = q_chevrons.get_mut(descendant) {
                chevron.image = assets.load(if collapsed {
                    icons::CHEVRON_RIGHT
                } else {
                    icons::CHEVRON_DOWN
                });
            }
        }
    };
    for root in q_collapsed.iter() {
        apply(root, true);
    }
    for root in removed.read() {
        if q_subpanes.contains(root) {
            apply(root, false);
        }
    }
}

/// Plugin which registers the sub-pane collapse systems.
pub struct SubpanePlugin;

impl Plugin for SubpanePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            update_subpane_collapse.in_set(PickingSystems::Last),
        );
    }
}
