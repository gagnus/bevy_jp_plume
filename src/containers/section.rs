//! Collapsible section container with a header bar.
use core::f32::consts::FRAC_PI_2;

use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::picking::PickingSystems;
use bevy::picking::events::PointerClick;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, on};
use bevy::ui::{
    AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, UiSystems, UiTransform, Val,
};

use crate::body::{BodyGap, BodyPadding, apply_body_style};
use crate::constants::{lucide, size};
use crate::cursor::EntityCursor;
use crate::display::icon;
use crate::theme::{InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken};
use crate::tokens;
use crate::utils::anim::AnimState;
use crate::utils::hierarchy::descendant_get_mut;

/// A section: a header bar over a body. Collapsible by default - clicking the
/// header folds the body away.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSectionProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSection;

// Plain root marker, inserted by [`section_frame`] on both the retained and imm paths.
// The systems key on this, not [`PlumeSection`], which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SectionRoot;

/// Marker for a collapsed [`PlumeSection`]; insert it to start collapsed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SectionCollapsed;

// Whether a section's header responds to clicks by folding its body. App-owned config,
// unlike [`SectionCollapsed`], which is user state. Absent means collapsible.
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
            header: Box::new(()),
            contents: Box::new(()),
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
            @section_frame(
                header,
                collapsible,
                // Empty for the imm layer, which reconciles the body itself.
                bsn! {
                    @section_body()
                    Children [
                        {contents}
                    ]
                },
            )
        }
    }
}

// Section chrome (root, header bar, chevron, collapse behavior) shared by the public
// [`PlumeSection`] and the imm layer. `props.contents` is inserted as the body slot
// verbatim; the imm layer leaves it empty and reconciles the body itself.
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
        SectionCollapsible(collapsible)
        ThemeBackgroundToken(tokens::SECTION_BODY_BG)
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Start,
                padding: UiRect::horizontal(size::SPACE),
                min_height: size::HEADER_HEIGHT,
                column_gap: size::SPACE,
                border_radius: size::CORNER_RADIUS,
            }
            SectionHeader
            ThemeBackgroundToken(tokens::SECTION_HEADER_BG)
            ThemeBorderToken(tokens::SEPARATOR)
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            InheritableThemeTextToken(tokens::SECTION_HEADER_TEXT)
            on(toggle_section_collapse)
            Children [
                {collapsible.then(|| bsn! {
                    @icon(lucide::CHEVRON_DOWN)
                    Node { width: size::ICON_WIDTH }
                    SectionChevron
                    AnimState::rotation(0.0, -FRAC_PI_2)
                    UiTransform::default()
                })}
                --
                {header}
            ]
            --
            {body}
        ]
    }
}

// The section body node: a padded, tight-gapped column folded away on collapse.
// Callers append the body content as children; children stretch to the body width.
pub(crate) fn section_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::SPACE_TIGHT,
            padding: size::SPACE,
        }
        SectionBody
    }
}

fn toggle_section_collapse(
    click: On<PointerClick>,
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
        if let Some(mut node) = descendant_get_mut(root, &q_children, &mut q_body) {
            node.display = if collapsed {
                Display::None
            } else {
                Display::Flex
            };
        }
        if let Some(mut chevron) = descendant_get_mut(root, &q_children, &mut q_chevrons) {
            chevron.set_target(if collapsed { 1.0 } else { 0.0 });
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

// The chevron is spawned once at build time, but the imm layer sets
// [`SectionCollapsible`] after the fact via `.collapsible(_)`, so its visibility
// tracks the component here rather than the spawn-time prop.
fn update_section_collapsible(
    q_changed: Query<
        (Entity, &SectionCollapsible, Has<SectionCollapsed>),
        (With<SectionRoot>, Changed<SectionCollapsible>),
    >,
    q_children: Query<&Children>,
    mut q_chevrons: Query<(&mut Node, &mut AnimState), With<SectionChevron>>,
) {
    for (root, collapsible, collapsed) in q_changed.iter() {
        if let Some((mut node, mut anim)) = descendant_get_mut(root, &q_children, &mut q_chevrons) {
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

// Header with `collapsible` false (`SectionCollapsible(false)`) is non-filled.
fn update_section_header_style(
    q_changed: Query<Entity, (With<SectionRoot>, Changed<SectionCollapsible>)>,
    q_sections: Query<Option<&SectionCollapsible>, With<SectionRoot>>,
    q_children: Query<&Children>,
    mut q_headers: Query<(Entity, &mut Node), With<SectionHeader>>,
    mut commands: Commands,
) {
    for root in q_changed.iter() {
        let Ok(collapsible) = q_sections.get(root) else {
            continue;
        };
        // Absent means collapsible (the component's Default); only explicit false is flat.
        let collapsible = !matches!(collapsible, Some(SectionCollapsible(false)));

        if let Some((header, mut node)) = descendant_get_mut(root, &q_children, &mut q_headers) {
            let (bg_token, text_token, border, border_radius, cursor) = if !collapsible {
                (
                    tokens::SECTION_BODY_BG,
                    tokens::SECTION_HEADER_MUTED_TEXT,
                    UiRect::bottom(size::HAIRLINE),
                    Val::ZERO,
                    bevy::window::SystemCursorIcon::Default,
                )
            } else {
                (
                    tokens::SECTION_HEADER_BG,
                    tokens::SECTION_HEADER_TEXT,
                    UiRect::ZERO,
                    size::CORNER_RADIUS,
                    bevy::window::SystemCursorIcon::Pointer,
                )
            };
            node.border = border;
            node.border_radius = border_radius.into();
            commands.entity(header).insert((
                ThemeBackgroundToken(bg_token),
                InheritableThemeTextToken(text_token),
                EntityCursor::System(cursor),
            ));
        }
    }
}

// `BodyGap` / `BodyPadding` sit on the frame, which is what a caller holds, but the
// body child is what lays the content out.
fn relay_section_body_style(
    q_frames: Query<
        (Option<&BodyGap>, Option<&BodyPadding>, &Children),
        (With<SectionRoot>, Or<(With<BodyGap>, With<BodyPadding>)>),
    >,
    q_bodies: Query<(), With<SectionBody>>,
    mut q_nodes: Query<&mut Node>,
) {
    for (gap, padding, children) in q_frames.iter() {
        let Some(&body) = children.iter().find(|entity| q_bodies.contains(**entity)) else {
            continue;
        };
        if let Ok(mut node) = q_nodes.get_mut(body) {
            apply_body_style(&mut node, gap, padding);
        }
    }
}

// Plugin which registers the section collapse systems.
pub(crate) struct SectionPlugin;

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
        app.add_systems(
            PostUpdate,
            relay_section_body_style.before(UiSystems::Layout),
        );
    }
}
