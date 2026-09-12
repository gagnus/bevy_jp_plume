//! Tab container: a strip of tab buttons over a body, one tab visible at a time.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::ecs::template::{EntityTemplate, FromTemplate};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, on};
use bevy::text::{LineBreak, TextLayout};
use bevy::time::Time;
use bevy::ui::{
    AlignItems, ComputedNode, Display, FlexDirection, InteractionDisabled, JustifyContent, Node,
    Overflow, PositionType, Pressed, Selected, UiRect, UiSystems, UiTransform, Val, ZIndex, px,
};
use bevy::ui_widgets::{Activate, Button, ControlOrientation, ValueChange};

use crate::body::{BodyGap, BodyPadding, apply_body_style};
use crate::constants::{Icon, size};
use crate::containers::{ScrollAxis, scroll_frame, scroll_viewport, scrollbar_node};
use crate::controls::{PlumeScrollbar, ScrollbarHidden, SelectedIndex};
use crate::cursor::EntityCursor;
use crate::display::caption;
use crate::focus::{FocusIndicator, InsetFocusRing};
use crate::set_value::SetValue;
use crate::theme::{InheritableThemeTextToken, ThemeBackgroundToken};
use crate::utils::anim::{UI_ANIM_RATE, approach};
use crate::utils::hierarchy::{descendant_with, nearest_with};
use crate::{display, tokens};

// Width the indicator node is spawned at; it is scaled to the selected tab's
// width from there, so the slide is a transform and never a relayout.
const INDICATOR_BASE_WIDTH: f32 = 100.0;

/// A tab container: a header strip of [`PlumeTab`]s over the bodies they target.
/// Each tab names the body entity it shows, so the two lists are linked by name
/// rather than by position:
///
/// ```text
/// @header: … (@PlumeTab { @target: #general } Selected) …
/// @body:   … (#general tab_body() Children [ … ]) …
/// ```
/// # Emitted events
/// * [`ValueChange<usize>`](bevy::ui_widgets::ValueChange) with the picked tab index.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeTabsProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeTabs;

/// Props used to construct a [`PlumeTabs`] scene.
pub struct PlumeTabsProps {
    /// The strip's tabs, in left-to-right order; each a [`PlumeTab`].
    pub header: Box<dyn SceneList>,
    /// The bodies the tabs target, each named (`#video`) for its tab to point at.
    /// Only the selected tab's body is displayed.
    pub body: Box<dyn SceneList>,
}

impl Default for PlumeTabsProps {
    fn default() -> Self {
        Self {
            header: Box::new(()),
            body: Box::new(()),
        }
    }
}

impl PlumeTabs {
    /// Scene function for a tab container.
    pub fn scene(props: PlumeTabsProps) -> impl Scene {
        bsn! {
            @tabs_frame()
            SelectedIndex
            Children [
                @tab_strip_frame()
                Children [
                    #strip_viewport
                    @scroll_viewport(ScrollAxis::Horizontal)
                    Children [
                        @tab_strip()
                        Children [
                            {props.header}
                        ]
                    ]
                    --
                    @PlumeScrollbar {
                        @target: #strip_viewport,
                        @orientation: ControlOrientation::Horizontal,
                    }
                    @scrollbar_node(ScrollAxis::Horizontal)
                ]
                --
                {props.body}
            ]
        }
    }
}

/// One tab in a [`PlumeTabs`] strip: a flat, square-cornered button that shows
/// the body it targets. Mark the initially selected one with [`Selected`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeTabProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeTab;

/// Props used to construct a [`PlumeTab`] scene.
pub struct PlumeTabProps {
    /// Tab label content (e.g. `bsn! { caption("…") }`).
    pub caption: Box<dyn SceneList>,
    /// The body entity this tab shows, by name (`#video`). Omit it for a strip
    /// that only reports its selection and has no bodies to swap.
    pub target: EntityTemplate,
}

impl Default for PlumeTabProps {
    fn default() -> Self {
        Self {
            caption: Box::new(()),
            target: EntityTemplate::default(),
        }
    }
}

impl PlumeTab {
    /// Scene function for a tab.
    pub fn scene(props: PlumeTabProps) -> impl Scene {
        let target = props.target;
        // `EntityTemplate::None` is what an omitted `@target` leaves behind, and
        // building one is an error - a bodyless tab simply carries no `TabTarget`.
        let has_target = !matches!(target, EntityTemplate::None);
        bsn! {
            @tab_chrome()
            @{has_target.then(|| bsn! { TabTarget(target) })}
            Children [
                {props.caption}
            ]
        }
    }
}

/// The body a [`PlumeTab`] targets: its `Display` follows the tab's selection.
/// Tabs without one leave their body alone, which is what the imm layer relies on.
#[derive(Component, FromTemplate, Clone, Reflect)]
#[reflect(Component, Clone)]
pub struct TabTarget(pub Entity);

// Plain root marker, inserted by `tabs_frame` in both the retained and imm paths
// (unlike the `PlumeTabs` scene-component, which must not be inserted bare).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct TabsRoot;

// Set once the root's selection has been reconciled against the tab the app
// marked `Selected`; until then `seed_tab_selection` owns the first pick.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct TabsSeeded;

// The header strip holding the tab buttons and the selection indicator.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct TabStrip;

// A body a tab shows, so [`BodyGap`] / [`BodyPadding`] on the frame can find it.
// Both are container-wide, so the relay writes every body, not just the shown one.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct TabBody;

// Plain tab marker, inserted by `tab_chrome` in both paths.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct TabButton;

// The accent underline, easing along the strip toward the selected tab. `pos` and
// `width` are in logical px within the strip; `settled` suppresses the slide-in
// from zero on the first frame the layout is known.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct TabIndicator {
    pos: f32,
    width: f32,
    settled: bool,
}

// Tab container chrome: an invisible clipped column for [`tab_strip`] and the tab bodies.
pub(crate) fn tabs_frame() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            overflow: Overflow::clip(),
            // Flex's `auto` minimum refuses to shrink below content, so the strip
            // only bounds (and so scrolls) once every container above it can give.
            min_height: Val::ZERO,
            min_width: Val::ZERO,
        }
        TabsRoot
    }
}

// The strip's scrolling frame; the scrollbar stays hidden, since a strip has no room
// to give it and the wheel is how a crowded one is moved. The bar's fill sits here
// rather than on the strip, which a crowded strip's tabs overflow.
pub(crate) fn tab_strip_frame() -> impl Scene {
    bsn! {
        @scroll_frame(ScrollAxis::Horizontal)
        ScrollbarHidden
        ThemeBackgroundToken(tokens::TAB_BAR_BG)
    }
}

// The header strip. Tabs are appended as children; the indicator overlays the bottom
// edge, so the strip carries no padding for the two to share an origin. A crowded
// strip's tabs overflow it, so the bar's fill belongs on [`tab_strip_frame`] instead.
pub(crate) fn tab_strip() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            min_height: size::TAB_BAR_HEIGHT,
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
        }
        TabStrip
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: px(INDICATOR_BASE_WIDTH),
                height: size::TAB_INDICATOR_HEIGHT,
            }
            TabIndicator
            ZIndex(1)
            UiTransform::default()
            Pickable::IGNORE
            ThemeBackgroundToken(tokens::TAB_INDICATOR)
        ]
    }
}

// A tab's chrome, shared by the public [`PlumeTab`] and the imm layer; callers
// append the label content as children. The explicit `min_width` is what the strip
// sums to know how far it may squeeze; `auto` would report the full width instead.
pub(crate) fn tab_chrome() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: size::SPACE,
            padding: UiRect::horizontal(size::SPACE),
            min_width: size::TAB_MIN_WIDTH,
            overflow: Overflow::clip(),
        }
        Button
        TabButton
        Hovered
        TabIndex(0)
        FocusIndicator
        // A tab fills its strip's height, so an outset ring is clipped away by the
        // strip's scroll frame on every edge but the ones lapping its neighbours.
        InsetFocusRing
        EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
        ThemeBackgroundToken(tokens::TAB_BG)
        InheritableThemeTextToken(tokens::TAB_TEXT)
        on(select_tab_on_activate)
    }
}

// A tab with a text label and an optional leading icon: the imm layer's tab, and
// the shorthand a retained caller reaches for over a hand-built caption.
pub(crate) fn tab_button(label: String, icon: Option<Icon>) -> impl Scene {
    bsn! {
        @tab_chrome()
        Children [
            {icon.map(|glyph| bsn! {
                @display::icon(glyph)
                Node { flex_shrink: 0.0 }
            })}
            --
            @tab_label(label)
        ]
    }
}

/// A tab label for a hand-built header: the part that gives when the strip is
/// crowded, in a box of its own so it cuts there rather than over its siblings.
pub fn tab_label(label: impl Into<String>) -> impl Scene {
    let label = label.into();
    bsn! {
        Node {
            display: Display::Flex,
            align_items: AlignItems::Center,
            min_width: Val::ZERO,
            overflow: Overflow::clip(),
        }
        Children [
            @caption(label)
            Node { min_width: Val::ZERO }
            TextLayout { linebreak: LineBreak::NoWrap }
        ]
    }
}

/// A tab body: a padded column that children stretch to. Name it in `bsn!`
/// (`#video`) for a [`PlumeTab`] to target.
pub fn tab_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::SPACE,
            padding: size::SPACE,
            // Fills the height the strip leaves when the container is bounded
            // (no-op when it hugs its content), and `min_height` lets a
            // `scroll_area` inside shrink below its content instead of clipping.
            flex_grow: 1.0,
            min_height: Val::ZERO,
        }
        TabBody
        ThemeBackgroundToken(tokens::TAB_BODY_BG)
    }
}

// The strip's tabs in left-to-right order - the order every tab index counts in.
fn strip_tabs(
    root: Entity,
    q_children: &Query<&Children>,
    q_strips: &Query<(), With<TabStrip>>,
    is_tab: impl Fn(Entity) -> bool,
) -> Vec<Entity> {
    let Some(strip) = descendant_with(root, q_children, q_strips) else {
        return Vec::new();
    };
    let Ok(children) = q_children.get(strip) else {
        return Vec::new();
    };
    children
        .iter()
        .copied()
        .filter(|&child| is_tab(child))
        .collect()
}

fn select_tab_on_activate(
    activate: On<Activate>,
    q_tabs: Query<Has<InteractionDisabled>, With<TabButton>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_roots: Query<(), With<TabsRoot>>,
    q_strips: Query<(), With<TabStrip>>,
    mut commands: Commands,
) {
    let tab_ent = activate.event_target();
    let Ok(disabled) = q_tabs.get(tab_ent) else {
        return;
    };
    if disabled {
        return;
    }
    let Some(root) = nearest_with(tab_ent, &q_parents, &q_roots) else {
        return;
    };
    let tabs = strip_tabs(root, &q_children, &q_strips, |tab| q_tabs.contains(tab));
    if let Some(index) = tabs.iter().position(|&tab| tab == tab_ent) {
        commands.entity(root).insert(SelectedIndex(index));
        commands.trigger(ValueChange {
            source: root,
            value: index,
            is_final: true,
        });
    }
}

// Programmatic selection, the counterpart of the [`PlumeSelect`](crate::controls::PlumeSelect)
// observer: same event, matched against tab position instead of list rows.
fn tabs_on_set_selected_index(
    ev: On<SetValue<usize>>,
    q_roots: Query<(), With<TabsRoot>>,
    q_children: Query<&Children>,
    q_strips: Query<(), With<TabStrip>>,
    q_tabs: Query<(), With<TabButton>>,
    mut commands: Commands,
) {
    if !q_roots.contains(ev.entity) {
        return;
    }
    // Only tabs that exist: an out-of-range push would strand the indicator.
    let tabs = strip_tabs(ev.entity, &q_children, &q_strips, |tab| {
        q_tabs.contains(tab)
    });
    if ev.value < tabs.len() {
        commands.entity(ev.entity).insert(SelectedIndex(ev.value));
    }
}

// Adopt the app's initial pick: a retained container spawns its tabs with
// [`Selected`] on one of them, which this reads back into the root's index
// before [`apply_tab_selection`] starts driving the other direction.
fn seed_tab_selection(
    q_roots: Query<Entity, (With<TabsRoot>, Without<TabsSeeded>)>,
    q_children: Query<&Children>,
    q_strips: Query<(), With<TabStrip>>,
    q_tabs: Query<Has<Selected>, With<TabButton>>,
    mut commands: Commands,
) {
    for root in q_roots.iter() {
        let tabs = strip_tabs(root, &q_children, &q_strips, |tab| q_tabs.contains(tab));
        // No tabs yet: the strip's children have not spawned, so there is nothing to
        // read back. Stay unseeded and look again next frame.
        if tabs.is_empty() {
            continue;
        }
        if let Some(index) = tabs
            .iter()
            .position(|&tab| q_tabs.get(tab).is_ok_and(|selected| selected))
        {
            commands.entity(root).insert(SelectedIndex(index));
        }
        commands.entity(root).insert(TabsSeeded);
    }
}

// Push the root's index onto the tabs: exactly one carries [`Selected`], and each
// tab's [`TabTarget`] body is displayed only while its tab is the selected one.
fn apply_tab_selection(
    q_roots: Query<(Entity, &SelectedIndex), (With<TabsRoot>, With<TabsSeeded>)>,
    q_children: Query<&Children>,
    q_strips: Query<(), With<TabStrip>>,
    q_tabs: Query<(Has<Selected>, Option<&TabTarget>), With<TabButton>>,
    mut q_nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (root, selected) in q_roots.iter() {
        let tabs = strip_tabs(root, &q_children, &q_strips, |tab| q_tabs.contains(tab));
        for (index, &tab) in tabs.iter().enumerate() {
            let Ok((has_selected, target)) = q_tabs.get(tab) else {
                continue;
            };
            let is_selected = index == selected.0;
            if is_selected != has_selected {
                if is_selected {
                    commands.entity(tab).insert(Selected);
                } else {
                    commands.entity(tab).remove::<Selected>();
                }
            }
            let Some(target) = target else {
                continue;
            };
            let Ok(mut node) = q_nodes.get_mut(target.0) else {
                continue;
            };
            let display = if is_selected {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

// What a tab's colors are derived from, and the components they are written to.
// The strip and the container paint fixed tokens from their own scenes.
type TabStyle<'w> = (
    Entity,
    &'w ChildOf,
    Has<Selected>,
    &'w Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'w ThemeBackgroundToken,
    &'w InheritableThemeTextToken,
    &'w EntityCursor,
);

fn update_tab_styles(
    q_tabs: Query<
        TabStyle,
        (
            With<TabButton>,
            // Added<TabButton> guarantees the initial style pass on spawn.
            Or<(
                Added<TabButton>,
                Changed<Hovered>,
                Added<Selected>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_indicators: Query<&ThemeBackgroundToken, With<TabIndicator>>,
    mut commands: Commands,
) {
    for tab in q_tabs.iter() {
        set_tab_styles(tab, &q_children, &q_indicators, &mut commands);
    }
}

fn update_tab_styles_remove(
    q_tabs: Query<TabStyle, With<TabButton>>,
    q_children: Query<&Children>,
    q_indicators: Query<&ThemeBackgroundToken, With<TabIndicator>>,
    mut removed_selected: RemovedComponents<Selected>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_selected
        .read()
        .chain(removed_pressed.read())
        .chain(removed_disabled.read())
        .for_each(|ent| {
            if let Ok(tab) = q_tabs.get(ent) {
                set_tab_styles(tab, &q_children, &q_indicators, &mut commands);
            }
        });
}

fn set_tab_styles(
    (tab, child_of, selected, hovered, pressed, disabled, background, text_color, cursor_now): (
        Entity,
        &ChildOf,
        bool,
        &Hovered,
        bool,
        bool,
        &ThemeBackgroundToken,
        &InheritableThemeTextToken,
        &EntityCursor,
    ),
    q_children: &Query<&Children>,
    q_indicators: &Query<&ThemeBackgroundToken, With<TabIndicator>>,
    commands: &mut Commands,
) {
    // The selected tab keeps its surface under hover and press: it reads as the
    // body's continuation rather than as a button waiting to be pushed.
    let background_token = match selected {
        true => tokens::TAB_BG_SELECTED,
        false => tokens::sets::TAB_BG.pick(disabled, pressed, hovered.0),
    };
    if background.0 != background_token {
        commands
            .entity(tab)
            .insert(ThemeBackgroundToken(background_token));
    }

    let text_token = tokens::sets::TAB_TEXT.pick(selected, disabled);
    if text_color.0 != text_token {
        commands
            .entity(tab)
            .insert(InheritableThemeTextToken(text_token));
    }

    let cursor = EntityCursor::System(match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    });
    if *cursor_now != cursor {
        commands.entity(tab).insert(cursor);
    }

    // The accent underline mutes with the tab it points at, so only the selected
    // one speaks for it - the tab losing `Selected` leaves it to its replacement.
    if !selected {
        return;
    }
    let indicator_token = match disabled {
        true => tokens::TAB_INDICATOR_DISABLED,
        false => tokens::TAB_INDICATOR,
    };
    if let Some(indicator) = q_children.get(child_of.parent()).ok().and_then(|children| {
        children
            .iter()
            .copied()
            .find(|&child| q_indicators.contains(child))
    }) && q_indicators
        .get(indicator)
        .is_ok_and(|current| current.0 != indicator_token)
    {
        commands
            .entity(indicator)
            .insert(ThemeBackgroundToken(indicator_token));
    }
}

// Ease the underline toward the selected tab. Runs after layout and writes only
// [`UiTransform`], so tracking the tab's width costs no relayout.
fn update_tab_indicator(
    time: Res<Time>,
    q_roots: Query<Entity, With<TabsRoot>>,
    q_children: Query<&Children>,
    q_strips: Query<(), With<TabStrip>>,
    q_tabs: Query<(Has<Selected>, &ComputedNode), With<TabButton>>,
    mut q_indicators: Query<(&mut TabIndicator, &mut UiTransform)>,
) {
    let dt = time.delta_secs();
    for root in q_roots.iter() {
        let tabs = strip_tabs(root, &q_children, &q_strips, |tab| q_tabs.contains(tab));

        // Tabs are laid out left to right in strip order with no gap, so the
        // selected tab's offset is the sum of the widths before it.
        let mut offset = 0.0;
        let mut target = None;
        for &tab in tabs.iter() {
            let Ok((selected, computed)) = q_tabs.get(tab) else {
                continue;
            };
            let width = computed.size().x * computed.inverse_scale_factor();
            if selected {
                target = Some((offset, width));
                break;
            }
            offset += width;
        }
        let Some(strip) = descendant_with(root, &q_children, &q_strips) else {
            continue;
        };
        let Some(indicator_ent) = q_children.get(strip).ok().and_then(|children| {
            children
                .iter()
                .copied()
                .find(|&child| q_indicators.contains(child))
        }) else {
            continue;
        };
        let Ok((mut indicator, mut transform)) = q_indicators.get_mut(indicator_ent) else {
            continue;
        };

        // Zero width means the strip is not laid out yet; no target means it is empty.
        // Either way the underline collapses and stays unsettled, so a tab arriving
        // snaps it into place rather than sliding it out of the corner.
        let Some((target_pos, target_width)) = target.filter(|(_, width)| *width > 0.0) else {
            indicator.settled = false;
            if transform.scale.x != 0.0 {
                transform.scale.x = 0.0;
            }
            continue;
        };

        if indicator.settled {
            indicator.pos = approach(indicator.pos, target_pos, UI_ANIM_RATE, dt);
            indicator.width = approach(indicator.width, target_width, UI_ANIM_RATE, dt);
        } else {
            indicator.pos = target_pos;
            indicator.width = target_width;
            indicator.settled = true;
        }

        // Scale is about the node's center, so the translation targets centers too.
        let scale_x = indicator.width / INDICATOR_BASE_WIDTH;
        let translation_x = indicator.pos + indicator.width / 2.0 - INDICATOR_BASE_WIDTH / 2.0;
        if transform.scale.x != scale_x {
            transform.scale.x = scale_x;
        }
        let translation = px(translation_x);
        if transform.translation.x != translation {
            transform.translation.x = translation;
        }
    }
}

// `BodyGap` / `BodyPadding` sit on the frame, which is what a caller holds, but the
// bodies are what lay the content out.
fn relay_tab_body_style(
    q_frames: Query<
        (Option<&BodyGap>, Option<&BodyPadding>, &Children),
        (With<TabsRoot>, Or<(With<BodyGap>, With<BodyPadding>)>),
    >,
    q_bodies: Query<(), With<TabBody>>,
    mut q_nodes: Query<&mut Node>,
) {
    for (gap, padding, children) in q_frames.iter() {
        for body in children
            .iter()
            .copied()
            .filter(|&child| q_bodies.contains(child))
        {
            if let Ok(mut node) = q_nodes.get_mut(body) {
                apply_body_style(&mut node, gap, padding);
            }
        }
    }
}

// Plugin which registers the tab selection, styling and indicator systems.
pub(crate) struct TabsPlugin;

impl Plugin for TabsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(tabs_on_set_selected_index)
            .add_systems(
                PreUpdate,
                (
                    seed_tab_selection,
                    apply_tab_selection,
                    (update_tab_styles, update_tab_styles_remove),
                )
                    .chain()
                    .in_set(PickingSystems::Last),
            )
            .add_systems(
                PostUpdate,
                (
                    relay_tab_body_style.before(UiSystems::Layout),
                    update_tab_indicator.after(UiSystems::Layout),
                ),
            );
    }
}
