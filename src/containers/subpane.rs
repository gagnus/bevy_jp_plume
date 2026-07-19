//! Collapsible sub-pane container with a header bar.
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Has, With},
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
use bevy_ui::{AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, widget::Text};

use crate::{
    constants::{font_awesome, fonts, size},
    display::fa_icon,
    font_styles::InheritableFont,
    theme::{
        Flat, GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundColor,
        ThemeBackgroundGradient, control_box_shadow,
    },
    tokens,
};

/// A sub-pane: a header bar over a body. Collapsible by default — clicking the
/// header folds the body away.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSubpaneProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSubpane;

/// Plain root marker carrying the collapse behavior, inserted by [`subpane_frame`]
/// in both the retained and imm paths (unlike the [`PlumeSubpane`] scene-component,
/// which must not be inserted as a bare component).
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SubpaneRoot {
    pub collapsible: bool,
}

impl Default for SubpaneRoot {
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
pub(crate) struct SubpaneBody;

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
        let PlumeSubpaneProps {
            header,
            contents,
            collapsible,
        } = props;
        bsn! {
            subpane_frame(PlumeSubpaneProps {
                header,
                collapsible,
                // The public sub-pane owns its body; the imm layer passes empty
                // contents and reconciles the body itself.
                contents: Box::new(bsn_list!((
                    subpane_body()
                    Children [
                        {contents}
                    ]
                ))),
            })
        }
    }
}

/// Sub-pane chrome (root, header bar, chevron, collapse behavior) shared by the
/// public [`PlumeSubpane`] and the imm layer. `props.contents` is inserted as the
/// body slot verbatim (the public sub-pane wraps it in a [`subpane_body`]; the imm
/// layer leaves it empty and reconciles the body itself).
pub(crate) fn subpane_frame(props: PlumeSubpaneProps) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border_radius: size::CORNER_RADIUS,
        }
        SubpaneRoot { collapsible: {props.collapsible} }
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
                    column_gap: size::GAP,
                    border_radius: size::CORNER_RADIUS,
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
                    {props.collapsible.then(|| bsn! { (fa_icon(font_awesome::solid::ANGLE_DOWN) Node { width: size::ICON_WIDTH } SubpaneChevron) })},
                    {props.header}
                ]
            ),
            {props.contents}
        ]
    }
}

/// The sub-pane body node: a padded, tight-gapped column folded away on collapse.
/// Callers append the body content as children; children stretch to the body width.
pub(crate) fn subpane_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP_TIGHT,
            padding: size::PAD,
        }
        SubpaneBody
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}

fn toggle_subpane_collapse(
    click: On<Pointer<Click>>,
    q_headers: Query<&ChildOf, With<SubpaneHeader>>,
    q_subpanes: Query<(&SubpaneRoot, Has<SubpaneCollapsed>)>,
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

fn update_subpane_collapse(
    q_collapsed: Query<Entity, (With<SubpaneRoot>, Added<SubpaneCollapsed>)>,
    mut removed: RemovedComponents<SubpaneCollapsed>,
    q_subpanes: Query<(), With<SubpaneRoot>>,
    q_children: Query<&Children>,
    mut q_body: Query<&mut Node, With<SubpaneBody>>,
    mut q_chevrons: Query<&mut Text, With<SubpaneChevron>>,
) {
    let mut apply = |root: Entity, collapsed: bool| {
        for descendant in q_children.iter_descendants(root) {
            if let Ok(mut node) = q_body.get_mut(descendant) {
                node.display = if collapsed {
                    Display::None
                } else {
                    Display::Flex
                };
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

/// The header gradient is set once at scene build, so [`Flat`] on the subpane root
/// needs its own pass (unlike the controls, whose state resolvers read it).
fn update_subpane_header_flat(
    q_flagged: Query<(Entity, Has<Flat>), (With<SubpaneRoot>, Added<Flat>)>,
    mut removed_flat: RemovedComponents<Flat>,
    q_subpanes: Query<Has<Flat>, With<SubpaneRoot>>,
    q_children: Query<&Children>,
    q_headers: Query<&ThemeBackgroundGradient, With<SubpaneHeader>>,
    mut commands: Commands,
) {
    let apply = |root: Entity, flat: bool, commands: &mut Commands| {
        let amount = if flat { 0.0 } else { GRADIENT_AMOUNT };
        for descendant in q_children.iter_descendants(root) {
            if let Ok(header_bg) = q_headers.get(descendant)
                && header_bg.1 != amount
            {
                commands
                    .entity(descendant)
                    .insert(ThemeBackgroundGradient(tokens::SUBPANE_HEADER_BG, amount));
            }
        }
    };
    for (root, flat) in q_flagged.iter() {
        apply(root, flat, &mut commands);
    }
    for root in removed_flat.read() {
        if let Ok(flat) = q_subpanes.get(root) {
            apply(root, flat, &mut commands);
        }
    }
}

/// Plugin which registers the sub-pane collapse systems.
pub struct SubpanePlugin;

impl Plugin for SubpanePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_subpane_collapse, update_subpane_header_flat).in_set(PickingSystems::Last),
        );
    }
}
