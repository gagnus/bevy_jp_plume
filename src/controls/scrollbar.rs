//! Themed scrollbar control.
use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Changed, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    template::EntityTemplate,
};
use bevy_math::Vec2;
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{BorderRadius, ComputedNode, Node, UiSystems, Val, px};
use bevy_ui_widgets::{ControlOrientation, Scrollbar, ScrollbarDragState, ScrollbarThumb};

use crate::{cursor::EntityCursor, theme::ThemeBackgroundColor, tokens};

/// A scrollbar. The `target` property should point to an entity whose
/// [`ScrollPosition`](bevy_ui::ScrollPosition) will be synchronized with the scrollbar.
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

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeScrollbarThumb;

/// Put this on a scrollbar's parent: `padding.right` reserved for the scrollbar
/// while it is visible, reclaimed when the content fits.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ScrollbarGutter(pub Val);

impl PlumeScrollbar {
    /// Scene function for scrollbar.
    pub fn scene(props: PlumeScrollbarProps) -> impl Scene {
        bsn! {
            PlumeScrollbar
            Scrollbar {
                target: {props.target},
                orientation: {props.orientation},
                min_thumb_length: 8.0
            }
            Node {
                border_radius: BorderRadius::all(px(3))
            }
            ThemeBackgroundColor(tokens::SCROLLBAR_BG)
            Children [(
                Hovered
                ThemeBackgroundColor(tokens::SCROLLBAR_THUMB)
                ScrollbarThumb {
                    border_radius: BorderRadius::all(px(3))
                }
                PlumeScrollbarThumb
                EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            )]
        }
    }
}

fn update_scrollbar_thumb_styles(
    q_thumbs: Query<
        (Entity, &Hovered, &ThemeBackgroundColor, &ScrollbarDragState),
        (
            With<PlumeScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<ScrollbarDragState>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (scrollbar_ent, hovered, bg_color, drag_state) in q_thumbs.iter() {
        let bg_token = if drag_state.dragging {
            tokens::SCROLLBAR_THUMB_PRESSED
        } else if hovered.0 {
            tokens::SCROLLBAR_THUMB_HOVER
        } else {
            tokens::SCROLLBAR_THUMB
        };

        if bg_token != bg_color.0 {
            commands
                .entity(scrollbar_ent)
                .insert(ThemeBackgroundColor(bg_token));
        }
    }
}

/// Hide scrollbars whose target content fits its viewport (same overflow math as
/// the headless widget, which otherwise renders a full-length thumb), and
/// reclaim the parent's [`ScrollbarGutter`] while hidden.
fn update_scrollbar_visibility(
    mut q_scrollbars: Query<(Entity, &Scrollbar, &mut Visibility), With<PlumeScrollbar>>,
    q_scroll_area: Query<&ComputedNode>,
    q_parents: Query<&ChildOf>,
    mut q_gutters: Query<(&ScrollbarGutter, &mut Node)>,
) {
    for (scrollbar_ent, scrollbar, mut visibility) in q_scrollbars.iter_mut() {
        let Ok(area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };
        let visible = (area.size() - area.scrollbar_size).max(Vec2::ZERO);
        let overflows = match scrollbar.orientation {
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
        if let Ok(child_of) = q_parents.get(scrollbar_ent)
            && let Ok((gutter, mut node)) = q_gutters.get_mut(child_of.parent())
        {
            let padding = if overflows { gutter.0 } else { px(0) };
            if node.padding.right != padding {
                node.padding.right = padding;
            }
        }
    }
}

/// Plugin which registers the systems for updating the scrollbar styles.
pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
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
