//! Tab container: a strip of tab buttons over a body, one tab visible at a time.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::{EntityTemplate, FromTemplate},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::{Pickable, PickingSystems, hover::Hovered};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on, template_value};
use bevy::time::Time;
use bevy::ui::{
    AlignItems, BorderRadius, ComputedNode, Display, FlexDirection, InteractionDisabled,
    JustifyContent, Node, Overflow, PositionType, Selected, UiRect, UiSystems, UiTransform, Val,
    ZIndex, px,
};
use bevy::ui_widgets::{Activate, Button};

use crate::{
    constants::{FaIcon, size},
    controls::{SelectedIndex, SetSelectedIndex},
    cursor::EntityCursor,
    display::{caption, fa_icon},
    focus::FocusIndicator,
    font_styles::TextStyleRelay,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor},
    tokens,
    utils::anim::{UI_ANIM_RATE, approach},
};

/// Width the indicator node is spawned at; it is scaled to the selected tab's
/// width from there, so the slide is a transform and never a relayout.
const INDICATOR_BASE_WIDTH: f32 = 100.0;

/// A tab container: a header strip of [`PlumeTab`]s over the bodies they target.
/// Each tab names the body entity it shows, so the two lists are linked by name
/// rather than by position:
///
/// ```text
/// bsn! {
///     @PlumeTabs {
///         @header: {Box::new(bsn_list![
///             (@PlumeTab { @caption: bsn! { caption("General") }, @target: #general } Selected),
///             (@PlumeTab { @caption: bsn! { caption("Video") }, @target: #video }),
///         ])},
///         @body: {Box::new(bsn_list![
///             (#general tab_body() Children [ … ]),
///             (#video tab_body() Children [ … ]),
///         ])}
///     }
/// }
/// ```
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
            header: Box::new(bsn_list!()),
            body: Box::new(bsn_list!()),
        }
    }
}

impl PlumeTabs {
    /// Scene function for a tab container.
    pub fn scene(props: PlumeTabsProps) -> impl Scene {
        bsn! {
            tabs_frame(0)
            Children [
                (
                    tab_strip()
                    Children [
                        {props.header}
                    ]
                ),
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
    /// The body entity this tab shows, by name (`#video`).
    pub target: EntityTemplate,
}

impl Default for PlumeTabProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            target: EntityTemplate::default(),
        }
    }
}

impl PlumeTab {
    /// Scene function for a tab.
    pub fn scene(props: PlumeTabProps) -> impl Scene {
        bsn! {
            tab_chrome()
            TabTarget({props.target})
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

/// Tab container chrome: a clipped column for [`tab_strip`] and the tab bodies,
/// with the initially selected tab seeded as `selected`.
pub(crate) fn tabs_frame(selected: usize) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border_radius: size::CORNER_RADIUS,
            // Clipped so the square-cornered strip doesn't spill past the rounded frame.
            overflow: Overflow::clip(),
            // A clipping frame has to be able to shrink below its content, or a
            // bounded container can never size it and its body just clips away.
            min_height: Val::ZERO,
        }
        TabsRoot
        template_value(SelectedIndex(selected))
        ThemeBackgroundColor(tokens::TABS_BODY_BG)
        TextStyleRelay
    }
}

/// The header strip. Tabs are appended as children; the indicator overlays the
/// bottom edge, so the strip carries no padding for the two to share an origin.
pub(crate) fn tab_strip() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            min_height: size::HEADER_HEIGHT,
            padding: UiRect::top(px(4)),
        }
        TabStrip
        ThemeBackgroundColor(tokens::TABS_STRIP_BG)
        TextStyleRelay
        Children [
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::ZERO,
                    bottom: Val::ZERO,
                    width: {px(INDICATOR_BASE_WIDTH)},
                    height: size::TAB_INDICATOR_HEIGHT,
                }
                TabIndicator
                ZIndex(1)
                UiTransform::default()
                Pickable::IGNORE
                ThemeBackgroundColor(tokens::TAB_INDICATOR)
            )
        ]
    }
}

/// A tab's chrome, shared by the public [`PlumeTab`] and the imm layer; callers
/// append the label content as children.
pub(crate) fn tab_chrome() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: size::GAP,
            padding: UiRect::horizontal(size::GAP),
            border_radius: BorderRadius::top(size::CORNER_RADIUS_SMALL),
        }
        Button
        TabButton
        Hovered
        TabIndex(0)
        FocusIndicator
        EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
        ThemeBackgroundColor(tokens::TAB_BG)
        InheritableThemeTextColor(tokens::TAB_TEXT)
        TextStyleRelay
        on(select_tab_on_activate)
    }
}

/// A tab with a text label and an optional leading icon: the imm layer's tab, and
/// the shorthand a retained caller reaches for over a hand-built caption.
pub(crate) fn tab_button(label: String, icon: Option<FaIcon>) -> impl Scene {
    bsn! {
        tab_chrome()
        Children [
            {icon.map(|icon| bsn! { fa_icon(icon) })},
            caption(label)
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
            row_gap: size::GAP_TIGHT,
            padding: size::PAD,
            // Fills the height the strip leaves when the container is bounded
            // (no-op when it hugs its content), and `min_height` lets a
            // `scroll_area` inside shrink below its content instead of clipping.
            flex_grow: 1.0,
            min_height: Val::ZERO,
        }
        InheritableThemeTextColor(tokens::TEXT_DIM)
        TextStyleRelay
    }
}

// The strip's tabs in left-to-right order — the order every tab index counts in.
fn strip_tabs(
    root: Entity,
    q_children: &Query<&Children>,
    q_strips: &Query<(), With<TabStrip>>,
    is_tab: impl Fn(Entity) -> bool,
) -> Vec<Entity> {
    let Some(strip) = q_children
        .iter_descendants(root)
        .find(|descendant| q_strips.contains(*descendant))
    else {
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
    let Some(root) = q_parents
        .iter_ancestors(tab_ent)
        .find(|ancestor| q_roots.contains(*ancestor))
    else {
        return;
    };
    let tabs = strip_tabs(root, &q_children, &q_strips, |tab| q_tabs.contains(tab));
    if let Some(index) = tabs.iter().position(|&tab| tab == tab_ent) {
        commands.entity(root).insert(SelectedIndex(index));
    }
}

/// Programmatic selection, the counterpart of the [`PlumeSelect`](crate::controls::PlumeSelect)
/// observer: same event, matched against tab position instead of list rows.
fn tabs_on_set_selected_index(
    ev: On<SetSelectedIndex>,
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
    if ev.index < tabs.len() {
        commands.entity(ev.entity).insert(SelectedIndex(ev.index));
    }
}

/// Adopt the app's initial pick: a retained container spawns its tabs with
/// [`Selected`] on one of them, which this reads back into the root's index
/// before [`apply_tab_selection`] starts driving the other direction.
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

/// Push the root's index onto the tabs: exactly one carries [`Selected`], and each
/// tab's [`TabTarget`] body is displayed only while its tab is the selected one.
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

// Selected and hovered are both cheap to recompute, and a strip holds a handful of
// tabs, so every tab is re-derived each frame and only differences are written back.
fn update_tab_styles(
    q_tabs: Query<
        (
            Entity,
            Has<Selected>,
            &Hovered,
            Has<InteractionDisabled>,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
        ),
        With<TabButton>,
    >,
    mut commands: Commands,
) {
    for (tab, selected, hovered, disabled, background, text_color) in q_tabs.iter() {
        let background_token = match (disabled, selected, hovered.0) {
            (_, true, _) => tokens::TAB_BG_SELECTED,
            (false, false, true) => tokens::TAB_BG_HOVER,
            _ => tokens::TAB_BG,
        };
        let text_token = match (disabled, selected) {
            (true, _) => tokens::TAB_TEXT_DISABLED,
            (false, true) => tokens::TAB_TEXT_SELECTED,
            (false, false) => tokens::TAB_TEXT,
        };
        let cursor = match disabled {
            true => bevy::window::SystemCursorIcon::NotAllowed,
            false => bevy::window::SystemCursorIcon::Pointer,
        };
        if background.0 != background_token {
            commands
                .entity(tab)
                .insert(ThemeBackgroundColor(background_token));
        }
        if text_color.0 != text_token {
            commands
                .entity(tab)
                .insert(InheritableThemeTextColor(text_token));
        }
        commands.entity(tab).insert(EntityCursor::System(cursor));
    }
}

/// Ease the underline toward the selected tab. Runs after layout and writes only
/// [`UiTransform`], so tracking the tab's width costs no relayout.
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
        // Zero width means the strip has not been laid out yet; leave the indicator
        // unsettled rather than easing away from a placeholder.
        let Some((target_pos, target_width)) = target.filter(|(_, width)| *width > 0.0) else {
            continue;
        };

        let Some(strip) = q_children
            .iter_descendants(root)
            .find(|descendant| q_strips.contains(*descendant))
        else {
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

        if indicator.settled {
            indicator.pos = approach(indicator.pos, target_pos, UI_ANIM_RATE, dt);
            indicator.width = approach(indicator.width, target_width, UI_ANIM_RATE, dt);
        } else {
            indicator.pos = target_pos;
            indicator.width = target_width;
            indicator.settled = true;
        }

        // Scale is about the node's centre, so the translation targets centres too.
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

/// Plugin which registers the tab selection, styling and indicator systems.
pub(crate) struct TabsPlugin;

impl Plugin for TabsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(tabs_on_set_selected_index)
            .add_systems(
                PreUpdate,
                (seed_tab_selection, apply_tab_selection, update_tab_styles)
                    .chain()
                    .in_set(PickingSystems::Last),
            )
            .add_systems(PostUpdate, update_tab_indicator.after(UiSystems::Layout));
    }
}
