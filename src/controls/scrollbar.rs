//! Themed scrollbar control.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::query::{Changed, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::ecs::template::EntityTemplate;
use bevy::math::Vec2;
use bevy::picking::PickingSystems;
use bevy::picking::hover::Hovered;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{ComputedNode, Node, UiSystems, Val};
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarDragState, ScrollbarThumb};

use crate::constants::size;
use crate::cursor::EntityCursor;
use crate::theme::ThemeBackgroundToken;
use crate::tokens;

/// A scrollbar. The `target` property should point to an entity whose
/// [`ScrollPosition`](bevy::ui::ScrollPosition) will be synchronized with the scrollbar.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeScrollbarProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeScrollbar;

/// Props used to construct a [`PlumeScrollbar`] scene.
#[derive(Default, Clone)]
pub struct PlumeScrollbarProps {
    /// The entity whose scroll position will be synchronized with this scrollbar.
    pub target: EntityTemplate,
    /// Whether this is a vertical or horizontal scrollbar.
    pub orientation: ControlOrientation,
}

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not [`PlumeScrollbar`], which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ScrollbarFrame;

// The themed fill inside the bar. Not named `ScrollbarThumb` — that is
// `bevy_ui_widgets`' own component, which sits on the same entity.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ThemedThumb;

/// Put this on a scrollbar's parent: padding on the scrollbar's own edge reserved
/// for it while it is visible, reclaimed when the content fits.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ScrollbarGutter(pub Val);

/// Put this on a scrollbar's parent to keep the scrollbar out of sight: the region
/// still scrolls, but shows nothing and reserves no gutter.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ScrollbarHidden;

impl PlumeScrollbar {
    /// Scene function for scrollbar.
    pub fn scene(props: PlumeScrollbarProps) -> impl Scene {
        bsn! {
            ScrollbarFrame
            Scrollbar {
                target: {props.target},
                orientation: {props.orientation},
                // Logical px in the headless widget — not a `Val`, so it can't
                // be em; it only bites on very long scroll extents.
                min_thumb_length: 8.0,
            }
            Node {
                border_radius: {size::SCROLLBAR_WIDTH / 2.0},
            }
            ThemeBackgroundToken(tokens::SCROLLBAR_BG)
            Children [
                Hovered
                ThemeBackgroundToken(tokens::SCROLLBAR_THUMB)
                ScrollbarThumb {
                    border_radius: {size::SCROLLBAR_WIDTH / 2.0},
                }
                ThemedThumb
                EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            ]
        }
    }
}

fn update_scrollbar_thumb_styles(
    q_thumbs: Query<
        (Entity, &Hovered, &ThemeBackgroundToken, &ScrollbarDragState),
        (
            With<ThemedThumb>,
            Or<(Changed<Hovered>, Changed<ScrollbarDragState>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (scrollbar_ent, hovered, bg_color, drag_state) in q_thumbs.iter() {
        let bg_token = tokens::sets::SCROLLBAR_THUMB.pick(false, drag_state.dragging, hovered.0);

        if bg_token != bg_color.0 {
            commands
                .entity(scrollbar_ent)
                .insert(ThemeBackgroundToken(bg_token));
        }
    }
}

// Hide scrollbars whose target content fits its viewport (same overflow math as
// the headless widget, which otherwise renders a full-length thumb), and
// reclaim the parent's [`ScrollbarGutter`] while hidden.
fn update_scrollbar_visibility(
    mut q_scrollbars: Query<(Entity, &Scrollbar, &mut Visibility), With<ScrollbarFrame>>,
    q_scroll_area: Query<&ComputedNode>,
    q_parents: Query<&ChildOf>,
    q_hidden: Query<(), With<ScrollbarHidden>>,
    mut q_gutters: Query<(&ScrollbarGutter, &mut Node)>,
) {
    for (scrollbar_ent, scrollbar, mut visibility) in q_scrollbars.iter_mut() {
        let Ok(area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };
        let parent = q_parents.get(scrollbar_ent).map(ChildOf::parent).ok();
        let visible = (area.size() - area.scrollbar_size).max(Vec2::ZERO);
        // A hidden scrollbar reads as never overflowing, so its gutter stays shut too.
        let overflows = !parent.is_some_and(|parent| q_hidden.contains(parent))
            && match scrollbar.orientation {
                ControlOrientation::Horizontal => area.content_size().x > visible.x,
                ControlOrientation::Vertical => area.content_size().y > visible.y,
            };
        let target = if overflows {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != target {
            *visibility = target;
        }
        if let Some(parent) = parent
            && let Ok((gutter, mut node)) = q_gutters.get_mut(parent)
        {
            let reserved = if overflows { gutter.0 } else { Val::ZERO };
            let mut padding = node.padding;
            match scrollbar.orientation {
                ControlOrientation::Horizontal => padding.bottom = reserved,
                ControlOrientation::Vertical => padding.right = reserved,
            }
            // Compared first: writing unconditionally would dirty layout every frame.
            if node.padding != padding {
                node.padding = padding;
            }
        }
    }
}

// Plugin which registers the systems for updating the scrollbar styles.
pub(crate) struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            update_scrollbar_thumb_styles.in_set(PickingSystems::Last),
        );
        app.add_systems(
            PostUpdate,
            update_scrollbar_visibility.after(UiSystems::Layout),
        );
    }
}
