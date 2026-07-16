//! Collapsible sub-pane container with a header bar.
use bevy_app::{Plugin, PreUpdate};
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
    system::{Commands, Query},
};
use bevy_picking::{
    PickingSystems,
    events::{Click, Pointer},
};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on, template_value};
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, px, widget::Text};

use crate::{
    constants::{font_awesome, fonts, size},
    display::fa_icon,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{
        GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundColor, ThemeBackgroundGradient,
        control_box_shadow,
    },
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
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                border_radius: px(size::CORNER_RADIUS),
            }
            PlumeSubpane { collapsible: {props.collapsible} }
            ThemeBackgroundColor(tokens::SUBPANE_BODY_BG)
            Children [
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        padding: UiRect::horizontal(size::HEADER_PAD_X),
                        min_height: size::HEADER_HEIGHT,
                        column_gap: size::GAP_TIGHT,
                        border_radius: px(size::CORNER_RADIUS),
                    }
                    template_value(control_box_shadow())
                    SubpaneHeader
                    ThemeBackgroundGradient(tokens::SUBPANE_HEADER_BG, GRADIENT_AMOUNT)
                    InheritableThemeTextColor(tokens::SUBPANE_HEADER_TEXT)
                    InheritableFont {
                        font: fonts::REGULAR,
                        font_size: size::MEDIUM_FONT,
                        weight: FontWeight::NORMAL,
                    }
                    on(toggle_subpane_collapse)
                    Children [
                        {props.collapsible.then(|| bsn! { (fa_icon(font_awesome::solid::ANGLE_DOWN) Node { width: px(16) } SubpaneChevron) })},
                        {props.header}
                    ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: size::GAP_TIGHT,
                        padding: size::PAD,
                    }
                    SubpaneBody
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
    mut q_chevrons: Query<&mut Text, With<SubpaneChevron>>,
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
                chevron.0 = if collapsed {
                    font_awesome::solid::ANGLE_RIGHT.glyph().into()
                } else {
                    font_awesome::solid::ANGLE_DOWN.glyph().into()
                };
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
