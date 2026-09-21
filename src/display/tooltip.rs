//! Hover tooltips: insert [`Tooltip`] on any control and, after a hover
//! delay, a small inert panel floats by it until the hover ends.
use std::sync::Arc;

use bevy::app::{Plugin, PostUpdate, Update};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::log::warn;
use bevy::math::Vec2;
use bevy::picking::Pickable;
use bevy::picking::events::{PointerMove, PointerPress};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{FontSourceTemplate, TextLayoutInfo};
use bevy::time::{Real, Time};
use bevy::ui::{
    CalculatedClip, ComputedNode, ComputedUiRenderTargetInfo, FixedNode, FlexDirection,
    GlobalZIndex, JustifyContent, Node, PositionType, UiGlobalTransform, UiSystems, Val, px,
};

use crate::constants::{fonts, size, z_order};
use crate::containers::PopupAnchor;
use crate::display::caption;
use crate::font_styles::{InheritableFont, PlumeFontSize};
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, control_box_shadow,
};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

/// Tooltip text shown after hovering this control (or any descendant).
#[derive(Component, Clone, PartialEq, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Tooltip(pub String);

/// Tooltip shown only while a clipping ancestor cuts this text off: the full
/// text on hover, nothing once it all fits. Not for use beside [`Tooltip`].
#[derive(Component, Clone, PartialEq, Default, Reflect)]
#[reflect(Component, Default)]
pub struct TooltipWhenClipped(pub String);

/// Rich tooltip contents: a scene spawned into the tooltip panel when it
/// shows. Takes precedence over [`Tooltip`] text on the same control.
#[derive(Component, Clone)]
pub struct TooltipContent(Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>);

// The default exists so `bsn!` accepts `TooltipContent` entries (blanket
// `FromTemplate` needs `Default`); an unset one shows nothing and says so.
impl Default for TooltipContent {
    fn default() -> Self {
        Self(Arc::new(|| {
            warn!("TooltipContent contents not specified");
            Box::new(bsn_list! {})
        }))
    }
}

impl TooltipContent {
    /// Wrap a factory producing the tooltip's contents scene.
    pub fn new<S: SceneList + 'static>(contents: impl Fn() -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move || Box::new(contents())))
    }

    fn contents(&self) -> Box<dyn SceneList> {
        (self.0)()
    }
}

// Marks a control whose tooltip contents are built by the imm layer
// (`tooltip_container`); the tick system runs the state machine but spawns nothing.
#[derive(Component, Default, Clone)]
pub(crate) struct TooltipUi;

// Present on a [`TooltipUi`] anchor while the state machine shows for it; the
// imm builder reads it to know when to build the panel.
#[derive(Component, Default, Clone)]
pub(crate) struct TooltipShowing;

/// Timing knobs for tooltips, in seconds.
#[derive(Resource, Clone, Copy)]
pub struct TooltipSettings {
    /// Hover time before the first tooltip shows.
    pub delay: f32,
    /// Hover time while warm - a tooltip showed moments ago.
    pub warm_delay: f32,
    /// Hover-free time after which warmth is lost.
    pub reset_delay: f32,
}

impl Default for TooltipSettings {
    fn default() -> Self {
        Self {
            delay: 0.5,
            warm_delay: 0.1,
            reset_delay: 0.5,
        }
    }
}

#[derive(Clone, Copy, Default)]
enum TooltipState {
    #[default]
    Idle,
    // Cold hover: the full delay pending.
    Waiting {
        anchor: Entity,
        since: f64,
    },
    // Warm hover: a tooltip showed moments ago, the short delay pending.
    WarmWaiting {
        anchor: Entity,
        since: f64,
    },
    Showing {
        anchor: Entity,
    },
    // Nothing hovered; warmth decays until `reset_delay` passes.
    Cooling {
        since: f64,
    },
}

#[derive(Resource, Default)]
struct TooltipController {
    state: TooltipState,
    shown: Option<ShownTooltip>,
}

struct ShownTooltip {
    anchor: Entity,
    // The tick-spawned box; `None` when the imm layer owns the display.
    root: Option<Entity>,
    content: ShownContent,
}

enum ShownContent {
    Text(String),
    Scene,
    Imm,
}

type TooltipSources = Or<(With<Tooltip>, With<TooltipContent>, With<TooltipUi>)>;

fn on_pointer_move(
    pointer: On<PointerMove>,
    q_sources: Query<(), TooltipSources>,
    q_childof: Query<&ChildOf>,
    mut controller: ResMut<TooltipController>,
    time: Res<Time<Real>>,
) {
    // Bubbling refires this at every ancestor hop; evaluate the topmost hit once.
    if pointer.entity != pointer.original_event_target() {
        return;
    }
    let hovered = nearest_with(pointer.entity, &q_childof, &q_sources);
    let now = time.elapsed_secs_f64();
    use TooltipState::*;
    controller.state = match (controller.state, hovered) {
        (
            state @ (Waiting { anchor, .. } | WarmWaiting { anchor, .. } | Showing { anchor }),
            Some(hovered_anchor),
        ) if anchor == hovered_anchor => state,
        (Idle | Waiting { .. }, Some(anchor)) => Waiting { anchor, since: now },
        (WarmWaiting { .. } | Showing { .. } | Cooling { .. }, Some(anchor)) => {
            WarmWaiting { anchor, since: now }
        }
        (Idle | Waiting { .. }, None) => Idle,
        (WarmWaiting { .. } | Showing { .. }, None) => Cooling { since: now },
        (state @ Cooling { .. }, None) => state,
    };
}

fn on_pointer_press(_press: On<PointerPress>, mut controller: ResMut<TooltipController>) {
    controller.state = TooltipState::Idle;
}

fn update_tooltips(
    mut controller: ResMut<TooltipController>,
    settings: Res<TooltipSettings>,
    time: Res<Time<Real>>,
    q_sources: Query<(), TooltipSources>,
    q_text: Query<&Tooltip>,
    q_content: Query<&TooltipContent>,
    q_imm: Query<(), With<TooltipUi>>,
    mut commands: Commands,
) {
    use TooltipState::*;
    let now = time.elapsed_secs_f64();
    controller.state = match controller.state {
        Waiting { anchor, since } if now - since > settings.delay as f64 => Showing { anchor },
        WarmWaiting { anchor, since } if now - since > settings.warm_delay as f64 => {
            Showing { anchor }
        }
        Cooling { since } if now - since > settings.reset_delay as f64 => Idle,
        state => state,
    };
    // The imm reconciler tears controls down freely; a dead anchor resets the cycle.
    if let Waiting { anchor, .. } | WarmWaiting { anchor, .. } | Showing { anchor } =
        controller.state
        && !q_sources.contains(anchor)
    {
        controller.state = Idle;
    }
    enum Desired<'w> {
        Text(&'w str),
        Scene(&'w TooltipContent),
        Imm,
    }
    let desired = match controller.state {
        Showing { anchor } => {
            if q_imm.contains(anchor) {
                Some((anchor, Desired::Imm))
            } else if let Ok(content) = q_content.get(anchor) {
                Some((anchor, Desired::Scene(content)))
            } else {
                q_text
                    .get(anchor)
                    .ok()
                    .map(|tooltip| (anchor, Desired::Text(tooltip.0.as_str())))
            }
        }
        _ => None,
    };
    let in_sync = match (&controller.shown, &desired) {
        (None, None) => true,
        (Some(shown), Some((anchor, desired))) if shown.anchor == *anchor => {
            match (&shown.content, desired) {
                (ShownContent::Text(text), Desired::Text(new_text)) => text == new_text,
                (ShownContent::Scene, Desired::Scene(_)) | (ShownContent::Imm, Desired::Imm) => {
                    true
                }
                _ => false,
            }
        }
        _ => false,
    };
    if in_sync {
        return;
    }
    if let Some(shown) = controller.shown.take() {
        if let Some(root) = shown.root {
            commands.entity(root).despawn();
        }
        if matches!(shown.content, ShownContent::Imm)
            && let Ok(mut anchor_commands) = commands.get_entity(shown.anchor)
        {
            anchor_commands.remove::<TooltipShowing>();
        }
    }
    let Some((anchor, desired)) = desired else {
        return;
    };
    controller.shown = Some(match desired {
        Desired::Text(text) => {
            let root = commands
                .spawn_scene(tooltip_panel(text.to_owned()))
                .insert(PopupAnchor(anchor))
                .id();
            ShownTooltip {
                anchor,
                root: Some(root),
                content: ShownContent::Text(text.to_owned()),
            }
        }
        Desired::Scene(content) => {
            let root = commands
                .spawn_scene(rich_tooltip_panel(content.contents()))
                .insert(PopupAnchor(anchor))
                .id();
            ShownTooltip {
                anchor,
                root: Some(root),
                content: ShownContent::Scene,
            }
        }
        Desired::Imm => {
            commands.entity(anchor).insert(TooltipShowing);
            ShownTooltip {
                anchor,
                root: None,
                content: ShownContent::Imm,
            }
        }
    });
}

// Marks the panel chrome so `shield_tooltip_content` can find content under it.
#[derive(Component, Default, Clone)]
pub(crate) struct TooltipPanel;

// Marks the positioning box [`place_tooltip_box`] drives.
#[derive(Component, Default, Clone)]
pub(crate) struct TooltipBox;

// Tooltips pin the root font on purpose (see `tooltip_chrome`), so the wrap
// width is rem to match it: em would track the scaled subtree the tooltip
// happens to hover over, px would stay put while the app's `RemSize` moved
// the text it has to wrap.
const TOOLTIP_WIDTH: Val = Val::Rem(25.0);

// Far enough off-screen that an unmeasured tooltip never flashes into view.
const PARKED_LEFT_PX: f32 = -4000.0;

// Invisible fixed-width positioning box; the visual panel centers inside it
// and hugs its content. A `FixedNode` layout root, so the panel's text wraps
// at the box width rather than the anchor's, and its inset is viewport-
// relative for [`place_tooltip_box`]. Spawns parked off-screen until measured.
pub(crate) fn tooltip_box() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(PARKED_LEFT_PX),
            width: TOOLTIP_WIDTH,
            justify_content: JustifyContent::Center,
        }
        FixedNode
        TooltipBox
        GlobalZIndex(z_order::TOOLTIP)
        Pickable::IGNORE
    }
}

// Inert themed chrome shared by every content kind: `Pickable::IGNORE`, so an
// overlapping tooltip never steals the hover that produced it.
pub(crate) fn tooltip_chrome() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: size::SPACE_TIGHT,
            padding: size::SPACE_TIGHT,
            border: size::HAIRLINE,
            border_radius: size::CORNER_RADIUS_SMALL,
        }
        TooltipPanel
        ThemeBackgroundToken(tokens::TOOLTIP_BG)
        ThemeBorderToken(tokens::TOOLTIP_BORDER)
        control_box_shadow()
        Pickable::IGNORE
        InheritableThemeTextToken(tokens::TOOLTIP_TEXT)
        // Fully specified: tooltips stay standard-sized inside scaled subtrees.
        InheritableFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: PlumeFontSize::Rem(1.0),
        }
    }
}

fn tooltip_panel(text: String) -> impl Scene {
    bsn! {
        @tooltip_box()
        Children [
            @tooltip_chrome()
            Children [
                @caption(text)
                Pickable::IGNORE
            ]
        ]
    }
}

fn rich_tooltip_panel(contents: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        @tooltip_box()
        Children [
            @tooltip_chrome()
            Children [
                {contents}
            ]
        ]
    }
}

// Positions each tooltip box: the panel centered under its anchor, slid
// horizontally to stay inside the window, flipped above when out of room
// below. Placement math uses the panel's rect (the box is wider); rects are
// the last layout's, so it trails the anchor by a frame like
// `track_popup_anchors`.
fn place_tooltip_box(
    mut q_boxes: Query<
        (
            &mut Node,
            &ComputedNode,
            &PopupAnchor,
            &Children,
            &ComputedUiRenderTargetInfo,
        ),
        With<TooltipBox>,
    >,
    q_rects: Query<(&ComputedNode, &UiGlobalTransform)>,
    q_panels: Query<&ComputedNode, With<TooltipPanel>>,
) {
    const GAP: f32 = 4.0;
    const WINDOW_MARGIN: f32 = 10.0;
    for (mut node, box_node, anchor, children, target) in q_boxes.iter_mut() {
        let Ok((anchor_node, anchor_transform)) = q_rects.get(anchor.0) else {
            continue;
        };
        let Some(panel) = children.iter().find_map(|child| q_panels.get(*child).ok()) else {
            continue;
        };
        let box_size = box_node.size() * box_node.inverse_scale_factor;
        let panel_size = panel.size() * panel.inverse_scale_factor;
        if panel_size.x <= 0.0 || box_size.x <= 0.0 {
            continue;
        }
        let window = target.logical_size();
        let anchor_size = anchor_node.size() * anchor_node.inverse_scale_factor;
        let anchor_top_left =
            anchor_transform.translation * anchor_node.inverse_scale_factor - 0.5 * anchor_size;
        let panel_left = (anchor_top_left.x + 0.5 * (anchor_size.x - panel_size.x)).clamp(
            WINDOW_MARGIN,
            (window.x - WINDOW_MARGIN - panel_size.x).max(WINDOW_MARGIN),
        );
        let below = anchor_top_left.y + anchor_size.y + GAP;
        let top = if below + box_size.y + WINDOW_MARGIN > window.y {
            anchor_top_left.y - GAP - box_size.y
        } else {
            below
        };
        let (left, top) = (px(panel_left - 0.5 * (box_size.x - panel_size.x)), px(top));
        // Read through the immutable deref first: writing unconditionally would
        // dirty layout every frame.
        if (node.left, node.top) != (left, top) {
            (node.left, node.top) = (left, top);
        }
    }
}

// Text anchors at its node's top-left, so the visible text rect is that corner
// plus the layout size - whatever the node itself was squeezed to.
fn promote_clipped_tooltips(
    q_sources: Query<(
        Entity,
        &TooltipWhenClipped,
        &ComputedNode,
        &UiGlobalTransform,
        &TextLayoutInfo,
        Option<&CalculatedClip>,
        Option<&Tooltip>,
    )>,
    mut commands: Commands,
) {
    // Sub-pixel overhang from rounding is not a cut-off.
    const TOLERANCE: f32 = 0.5;
    for (entity, when_clipped, node, transform, layout, clip, tooltip) in q_sources.iter() {
        let clipped = clip.is_some_and(|clip| {
            let top_left = transform.translation - 0.5 * node.size() + Vec2::splat(TOLERANCE);
            let bottom_right = top_left + layout.size - Vec2::splat(2.0 * TOLERANCE);
            !clip.is_fully_clipped()
                && !(clip.contains_point(top_left) && clip.contains_point(bottom_right))
        });
        match (clipped, tooltip) {
            (true, Some(tooltip)) if tooltip.0 == when_clipped.0 => {}
            (true, _) => {
                commands
                    .entity(entity)
                    .insert(Tooltip(when_clipped.0.clone()));
            }
            (false, Some(_)) => {
                commands.entity(entity).remove::<Tooltip>();
            }
            (false, None) => {}
        }
    }
}

// Rich content is app-authored, so its nodes spawn pickable by default; every
// node appearing under a panel gets `Pickable::IGNORE` to keep the panel inert.
fn shield_tooltip_content(
    q_added: Query<Entity, Added<Node>>,
    q_childof: Query<&ChildOf>,
    q_panels: Query<(), With<TooltipPanel>>,
    mut commands: Commands,
) {
    for entity in q_added.iter() {
        if q_childof
            .iter_ancestors(entity)
            .any(|ancestor| q_panels.contains(ancestor))
        {
            commands.entity(entity).insert(Pickable::IGNORE);
        }
    }
}

pub(crate) struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<TooltipSettings>()
            .init_resource::<TooltipController>()
            .add_observer(on_pointer_move)
            .add_observer(on_pointer_press)
            .add_systems(Update, (update_tooltips, shield_tooltip_content))
            // Ahead of layout, like `track_popup_anchors`: the box lands where
            // this frame's layout puts everything else.
            .add_systems(PostUpdate, place_tooltip_box.in_set(UiSystems::Prepare))
            // Clip rects are this frame's, so a resize flips the tooltip the same frame.
            .add_systems(
                PostUpdate,
                promote_clipped_tooltips.after(UiSystems::PostLayout),
            );
    }
}
