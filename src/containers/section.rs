//! Collapsible section container with a header bar.
use core::f32::consts::FRAC_PI_2;

use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy::picking::{
    PickingSystems,
    events::{Click, Pointer},
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on, template_value};
use bevy::text::FontWeight;
use bevy::ui::{AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, UiTransform};

use crate::{
    constants::{font_awesome, fonts, size},
    cursor::EntityCursor,
    display::fa_icon,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
    utils::anim::AnimState,
};

/// A section: a header bar over a body. Collapsible by default — clicking the
/// header folds the body away.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSectionProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSection;

/// Plain root marker, inserted by [`section_frame`]
/// in both the retained and imm paths (unlike the [`PlumeSection`] scene-component,
/// which must not be inserted as a bare component).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SectionRoot;

/// Marker for a collapsed [`PlumeSection`]; insert it to start collapsed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SectionCollapsed;

/// Whether a section's header responds to clicks by folding its body. App-owned
/// config (unlike [`SectionCollapsed`], which is user state), so the imm layer
/// reconciles it every frame from the `.collapsible(_)` builder. Absent means
/// collapsible, matching the [`Default`].
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SectionCollapsible(pub(crate) bool);

impl Default for SectionCollapsible {
    fn default() -> Self {
        Self(true)
    }
}

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
            section_frame(
                header,
                collapsible,
                // Empty for the imm layer, which reconciles the body itself.
                bsn_list!((
                    section_body()
                    Children [
                        {contents}
                    ]
                )),
            )
        }
    }
}

/// Section chrome (root, header bar, chevron, collapse behavior) shared by the
/// public [`PlumeSection`] and the imm layer. `props.contents` is inserted as the
/// body slot verbatim (the public section wraps it in a [`section_body`]; the imm
/// layer leaves it empty and reconciles the body itself).
pub(crate) fn section_frame(
    header: impl SceneList,
    collapsible: bool,
    body: impl SceneList,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border_radius: size::CORNER_RADIUS,
        }
        SectionRoot
        template_value(SectionCollapsible(collapsible))
        ThemeBackgroundColor(tokens::SECTION_BODY_BG)
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    padding: UiRect::horizontal(size::PAD),
                    min_height: size::HEADER_HEIGHT,
                    column_gap: size::GAP,
                    border_radius: size::CORNER_RADIUS,
                }
                SectionHeader
                ThemeBackgroundColor(tokens::SECTION_HEADER_BG)
                ThemeBorderColor(tokens::SEPARATOR)
                EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
                InheritableThemeTextColor(tokens::SECTION_HEADER_TEXT)
                InheritableFont {
                    font: fonts::REGULAR,
                    font_size: size::MEDIUM_FONT,
                    weight: FontWeight::NORMAL,
                }
                on(toggle_section_collapse)
                Children [
                    {collapsible.then(|| bsn! { (fa_icon(font_awesome::solid::ANGLE_DOWN) Node { width: size::ICON_WIDTH } SectionChevron template_value(AnimState::rotation(0.0, -FRAC_PI_2)) UiTransform::default()) })},
                    {header}
                ]
            ),
            {body}
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
        // Both, not just the font: `section_frame` carries neither, so it has no
        // `ThemedText` to relay an inherited color through, and bare text in the
        // body would fall back to bevy's default white.
        InheritableThemeTextColor(tokens::TEXT_DIM)
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
    q_sections: Query<(Option<&SectionCollapsible>, Has<SectionCollapsed>), With<SectionRoot>>,
    mut commands: Commands,
) {
    let Ok(child_of) = q_headers.get(click.event_target()) else {
        return;
    };
    let root = child_of.parent();
    let Ok((collapsible, collapsed)) = q_sections.get(root) else {
        return;
    };
    // Absent means collapsible (the component's Default); only an explicit false locks it.
    if matches!(collapsible, Some(SectionCollapsible(false))) {
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
    mut q_chevrons: Query<&mut AnimState, With<SectionChevron>>,
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
                chevron.set_target(if collapsed { 1.0 } else { 0.0 });
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

/// The chevron is spawned once at build time, but the imm layer sets
/// [`SectionCollapsible`] after the fact via `.collapsible(_)`, so its visibility
/// tracks the component here rather than the spawn-time prop.
fn update_section_collapsible(
    q_changed: Query<
        (Entity, &SectionCollapsible, Has<SectionCollapsed>),
        (With<SectionRoot>, Changed<SectionCollapsible>),
    >,
    q_children: Query<&Children>,
    mut q_chevrons: Query<(&mut Node, &mut AnimState), With<SectionChevron>>,
) {
    for (root, collapsible, collapsed) in q_changed.iter() {
        for descendant in q_children.iter_descendants(root) {
            if let Ok((mut node, mut anim)) = q_chevrons.get_mut(descendant) {
                node.display = if collapsible.0 {
                    Display::Flex
                } else {
                    Display::None
                };
                // Settle the chevron without a spin.
                anim.set_target(if collapsed { 1.0 } else { 0.0 });
            }
        }
    }
}

/// Header with `collapsible` false (`SectionCollapsible(false)`) is non-filled.
fn update_section_header_style(
    q_changed: Query<Entity, (With<SectionRoot>, Changed<SectionCollapsible>)>,
    q_sections: Query<Option<&SectionCollapsible>, With<SectionRoot>>,
    q_children: Query<&Children>,
    mut q_headers: Query<&mut Node, With<SectionHeader>>,
    mut commands: Commands,
) {
    for root in q_changed.iter() {
        let Ok(collapsible) = q_sections.get(root) else {
            continue;
        };
        // Absent means collapsible (the component's Default); only explicit false is flat.
        let collapsible = !matches!(collapsible, Some(SectionCollapsible(false)));

        for descendant in q_children.iter_descendants(root) {
            let Ok(mut node) = q_headers.get_mut(descendant) else {
                continue;
            };
            let (bg_token, text_token, border, cursor) = if !collapsible {
                (
                    tokens::SECTION_BODY_BG,
                    tokens::SECTION_HEADER_MUTED_TEXT,
                    UiRect::bottom(size::CONTAINER_BORDER),
                    bevy::window::SystemCursorIcon::Default,
                )
            } else {
                (
                    tokens::SECTION_HEADER_BG,
                    tokens::SECTION_HEADER_TEXT,
                    UiRect::ZERO,
                    bevy::window::SystemCursorIcon::Pointer,
                )
            };
            node.border = border;
            commands.entity(descendant).insert((
                ThemeBackgroundColor(bg_token),
                InheritableThemeTextColor(text_token),
                EntityCursor::System(cursor),
            ));
        }
    }
}

/// Plugin which registers the section collapse systems.
pub struct SectionPlugin;

impl Plugin for SectionPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_section_collapse,
                update_section_collapsible,
                update_section_header_style,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
