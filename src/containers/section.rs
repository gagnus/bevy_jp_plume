//! Collapsible section container with a header bar.
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

/// A section: a header bar over a body. Collapsible by default — clicking the
/// header folds the body away.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSectionProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSection;

/// Plain root marker carrying the collapse behavior, inserted by [`section_frame`]
/// in both the retained and imm paths (unlike the [`PlumeSection`] scene-component,
/// which must not be inserted as a bare component).
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SectionRoot {
    pub collapsible: bool,
}

impl Default for SectionRoot {
    fn default() -> Self {
        Self { collapsible: true }
    }
}

/// Marker for a collapsed [`PlumeSection`]; insert it to start collapsed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SectionCollapsed;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SectionHeader;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SectionBody;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SectionChevron;

/// Props used to construct a [`PlumeSection`] scene.
pub struct PlumeSectionProps {
    /// Header bar content (e.g. `bsn! { caption("…") }`).
    pub header: Box<dyn SceneList>,
    /// Body content, folded away when collapsed.
    pub contents: Box<dyn SceneList>,
    /// Whether clicking the header collapses/expands the body.
    pub collapsible: bool,
}

impl Default for PlumeSectionProps {
    fn default() -> Self {
        Self {
            header: Box::new(bsn_list!()),
            contents: Box::new(bsn_list!()),
            collapsible: true,
        }
    }
}

impl PlumeSection {
    /// Scene function for a section.
    pub fn scene(props: PlumeSectionProps) -> impl Scene {
        let PlumeSectionProps {
            header,
            contents,
            collapsible,
        } = props;
        bsn! {
            section_frame(PlumeSectionProps {
                header,
                collapsible,
                // The public section owns its body; the imm layer passes empty
                // contents and reconciles the body itself.
                contents: Box::new(bsn_list!((
                    section_body()
                    Children [
                        {contents}
                    ]
                ))),
            })
        }
    }
}

/// Section chrome (root, header bar, chevron, collapse behavior) shared by the
/// public [`PlumeSection`] and the imm layer. `props.contents` is inserted as the
/// body slot verbatim (the public section wraps it in a [`section_body`]; the imm
/// layer leaves it empty and reconciles the body itself).
pub(crate) fn section_frame(props: PlumeSectionProps) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border_radius: size::CORNER_RADIUS,
        }
        SectionRoot { collapsible: {props.collapsible} }
        ThemeBackgroundColor(tokens::SECTION_BODY_BG)
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
                SectionHeader
                ThemeBackgroundGradient(tokens::SECTION_HEADER_BG, GRADIENT_AMOUNT)
                InheritableThemeTextColor(tokens::SECTION_HEADER_TEXT)
                InheritableFont {
                    font: fonts::REGULAR,
                    font_size: size::MEDIUM_FONT,
                    weight: FontWeight::NORMAL,
                }
                on(toggle_section_collapse)
                Children [
                    {props.collapsible.then(|| bsn! { (fa_icon(font_awesome::solid::ANGLE_DOWN) Node { width: size::ICON_WIDTH } SectionChevron) })},
                    {props.header}
                ]
            ),
            {props.contents}
        ]
    }
}

/// The section body node: a padded, tight-gapped column folded away on collapse.
/// Callers append the body content as children; children stretch to the body width.
pub(crate) fn section_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP_TIGHT,
            padding: size::PAD,
        }
        SectionBody
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}

fn toggle_section_collapse(
    click: On<Pointer<Click>>,
    q_headers: Query<&ChildOf, With<SectionHeader>>,
    q_sections: Query<(&SectionRoot, Has<SectionCollapsed>)>,
    mut commands: Commands,
) {
    let Ok(child_of) = q_headers.get(click.event_target()) else {
        return;
    };
    let root = child_of.parent();
    let Ok((section, collapsed)) = q_sections.get(root) else {
        return;
    };
    if !section.collapsible {
        return;
    }
    if collapsed {
        commands.entity(root).remove::<SectionCollapsed>();
    } else {
        commands.entity(root).insert(SectionCollapsed);
    }
}

fn update_section_collapse(
    q_collapsed: Query<Entity, (With<SectionRoot>, Added<SectionCollapsed>)>,
    mut removed: RemovedComponents<SectionCollapsed>,
    q_sections: Query<(), With<SectionRoot>>,
    q_children: Query<&Children>,
    mut q_body: Query<&mut Node, With<SectionBody>>,
    mut q_chevrons: Query<&mut Text, With<SectionChevron>>,
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
        if q_sections.contains(root) {
            apply(root, false);
        }
    }
}

/// The header gradient is set once at scene build, so [`Flat`] on the section root
/// needs its own pass (unlike the controls, whose state resolvers read it).
fn update_section_header_flat(
    q_flagged: Query<(Entity, Has<Flat>), (With<SectionRoot>, Added<Flat>)>,
    mut removed_flat: RemovedComponents<Flat>,
    q_sections: Query<Has<Flat>, With<SectionRoot>>,
    q_children: Query<&Children>,
    q_headers: Query<&ThemeBackgroundGradient, With<SectionHeader>>,
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
                    .insert(ThemeBackgroundGradient(tokens::SECTION_HEADER_BG, amount));
            }
        }
    };
    for (root, flat) in q_flagged.iter() {
        apply(root, flat, &mut commands);
    }
    for root in removed_flat.read() {
        if let Ok(flat) = q_sections.get(root) {
            apply(root, flat, &mut commands);
        }
    }
}

/// Plugin which registers the section collapse systems.
pub struct SectionPlugin;

impl Plugin for SectionPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_section_collapse, update_section_header_flat).in_set(PickingSystems::Last),
        );
    }
}
