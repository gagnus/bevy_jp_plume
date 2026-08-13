//! Menu bar with drop-down menus, checkable items, and submenus.
use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::app::{Plugin, PostUpdate, PreUpdate, Update};
use bevy::camera::visibility::{Visibility, VisibilitySystems};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::{Add, Remove, RemovedComponents};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, ResMut};
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::tab_navigation::{NavAction, TabGroup, TabIndex, TabNavigation};
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusSystems};
use bevy::picking::PickingSystems;
use bevy::picking::events::{Click, Pointer};
use bevy::picking::hover::Hovered;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{LineBreak, TextLayout};
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, BorderRadius, Checkable, Checked, Display, FlexDirection, GlobalZIndex,
    InteractionDisabled, Node, OverrideClip, PositionType, UiRect,
};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};
use bevy::ui_widgets::{
    Activate, ActivateOnPress, Button, MenuAction, MenuButton, MenuEvent, MenuFocusState,
    MenuFocusSystem, MenuItem, MenuLayout, MenuPopup, ValueChange,
};

use crate::constants::{font_awesome, size};
use crate::containers::{PopupSocket, Separator, popup_socket};
use crate::controls::SetValue;
use crate::cursor::EntityCursor;
use crate::display::{caption, fa_icon};
use crate::focus::FocusIndicator;
use crate::font_styles::{InheritableFont, TextStyleRelay};
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, control_box_shadow,
    set_optional_background,
};
use crate::tokens;

/// Horizontal strip of top-level [`PlumeMenuButton`]s, one per menu.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeMenuBar;

impl PlumeMenuBar {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                min_height: size::ROW_HEIGHT,
                padding: UiRect::horizontal(size::SPACE_TIGHT),
            }
            PlumeMenuBar
            AccessibilityNode(accesskit::Node::new(Role::MenuBar))
            TextStyleRelay
        }
    }
}

/// One entry in a menu tree. Its position picks its role: a child of a
/// [`PlumeMenuBar`] (or any non-menu parent) is a top-level button that opens a
/// menu; declared inside another menu button's `Children` it becomes an item in
/// that menu; an item with menu children of its own becomes a submenu.
///
/// Items with [`Checkable`](bevy::ui::Checkable) toggle
/// [`Checked`](bevy::ui::Checked) when picked.
/// # Emitted events
/// * [`Activate`](bevy::ui_widgets::Activate) when an item is picked.
/// * [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) with the new checked
///   state, for checkable items.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeMenuButtonProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeMenuButton;

/// Props used to construct a [`PlumeMenuButton`] scene.
#[derive(Default)]
pub struct PlumeMenuButtonProps {
    /// Label text.
    pub label: String,
    /// Right-aligned shortcut hint (display only; handling the key is the
    /// app's business).
    pub shortcut: Option<String>,
}

impl PlumeMenuButton {
    fn scene(props: PlumeMenuButtonProps) -> impl Scene {
        bsn! {
            menu_button_row(props.label, props.shortcut)
            Children [
                (
                    popup_socket()
                    MenuChrome
                    Children [
                        (
                            // Pre-rendered popup: hidden while closed, so items keep
                            // their observers and state across open/close.
                            menu_frame_chrome()
                            MenuChrome
                            Node { display: Display::None }
                            Visibility::Hidden
                        ),
                    ]
                ),
            ]
        }
    }
}

// The visible row every role shares: check gutter, label, spacer, shortcut hint,
// submenu caret.
pub(crate) fn menu_button_row(label: String, shortcut: Option<String>) -> impl Scene {
    let shortcut = shortcut.unwrap_or_default();
    let shortcut_display = if shortcut.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::SPACE,
            min_height: size::ROW_HEIGHT,
            padding: UiRect::left(size::SPACE),
            border_radius: size::CORNER_RADIUS_SMALL,
        }
        MenuButtonRow
        Hovered
        TabIndex(0)
        FocusIndicator
        EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
        InheritableThemeTextToken(tokens::MENU_ITEM_TEXT)
        TextStyleRelay
        on(menu_root_on_menu_event)
        Children [
            (
                // Check gutter: reserved on every item so labels align.
                fa_icon(font_awesome::solid::CHECK)
                Node { width: size::ICON_WIDTH }
                MenuChrome
                MenuCheckIcon
                Visibility::Hidden
            ),
            (
                // The popup's height is first measured inside the narrow anchor
                // socket; a wrapping label bakes that taller estimate into the
                // frame. Menu labels never wrap.
                caption(label)
                TextLayout { linebreak: LineBreak::NoWrap }
                MenuChrome
            ),
            (
                Node { flex_grow: 1.0 }
                MenuChrome
            ),
            (
                caption(shortcut)
                Node { display: shortcut_display }
                TextLayout { linebreak: LineBreak::NoWrap }
                MenuChrome
                MenuShortcutText
                InheritableThemeTextToken(tokens::TEXT_DIM)
            ),
            (
                // Submenu caret gutter; only submenus show the glyph.
                fa_icon(font_awesome::solid::ANGLE_RIGHT)
                Node { width: size::ICON_WIDTH }
                MenuChrome
                MenuCaretIcon
                Visibility::Hidden
            ),
        ]
    }
}

// The popup panel chrome, shared by the retained pre-rendered frame and the
// imm-built one.
pub(crate) fn menu_frame_chrome() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            min_width: size::em_from_px(160.0),
            border: size::HAIRLINE,
            padding: UiRect::vertical(size::SPACE_TIGHT),
            border_radius: size::CORNER_RADIUS,
        }
        MenuPopupFrame
        ThemeBackgroundToken(tokens::MENU_BG)
        ThemeBorderToken(tokens::MENU_BORDER)
        template_value(control_box_shadow())
        GlobalZIndex(100)
        OverrideClip
        InheritableThemeTextToken(tokens::TEXT_DIM)
        InheritableFont {}
        TextStyleRelay
    }
}

// An imm-built menu button: the row plus the role and behavior markers the
// classifier would otherwise derive — the imm layer knows the role statically.
pub(crate) fn imm_menu_anchor(
    role: MenuButtonRole,
    label: String,
    shortcut: Option<String>,
    checkable: bool,
) -> impl Scene {
    let bar = role == MenuButtonRole::Bar;
    let item = role == MenuButtonRole::Item;
    let submenu = role == MenuButtonRole::Submenu;
    bsn! {
        menu_button_row(label, shortcut)
        ImmMenuManaged
        template_value(role)
        {bar.then(|| bsn! { MenuButton })}
        {item.then(|| bsn! { MenuItem })}
        {submenu.then(|| bsn! { AccessibilityNode(accesskit::Node::new(Role::MenuItem)) })}
        {checkable.then(|| bsn! { Checkable })}
    }
}

// An imm-built popup frame. It lives in an unrooted socket, outside its anchor's
// entity tree, so [`MenuAnchorLink`] carries events and walks back to the anchor.
pub(crate) fn imm_menu_frame(
    anchor: Entity,
    role: MenuButtonRole,
    nav: Option<NavAction>,
) -> impl Scene {
    let submenu = role == MenuButtonRole::Submenu;
    let bar_focus = (!submenu).then(|| MenuFocusState::Opening(nav.unwrap_or(NavAction::First)));
    let sub_focus = submenu.then_some(nav).flatten().map(SubmenuOpening);
    bsn! {
        menu_frame_chrome()
        template_value(MenuAnchorLink(anchor))
        template_value(popover_for(role))
        on(imm_frame_on_menu_event)
        {(!submenu).then(|| bsn! { MenuPopup })}
        {submenu.then(|| bsn! {
            template_value(TabGroup::modal())
            AccessibilityNode(accesskit::Node::new(Role::MenuListPopup))
        })}
        {bar_focus.map(|state| bsn! { template_value(state) })}
        {sub_focus.map(|state| bsn! { template_value(state) })}
    }
}

// Routes menu events off the top of an unrooted imm popup back into the anchor's
// tree, where the ancestor menus can act on them.
fn imm_frame_on_menu_event(
    mut ev: On<MenuEvent>,
    q_links: Query<&MenuAnchorLink, With<MenuPopupFrame>>,
    mut commands: Commands,
) {
    let Ok(link) = q_links.get(ev.source) else {
        return;
    };
    let action = ev.event().action;
    match action {
        MenuAction::CloseAll | MenuAction::FocusRoot => {
            ev.propagate(false);
            commands.trigger(MenuEvent {
                source: link.0,
                action,
            });
        }
        _ => {}
    }
}

// Marker every menu button row carries, retained or imm-built. The systems key
// on this rather than [`PlumeMenuButton`], which is a scene component and so
// may only appear on entities spawned through its own `@` template — the imm
// anchors are not.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct MenuButtonRow;

// Marker for a menu button's own scene entities, so adoption can tell chrome
// from declared items.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct MenuChrome;

// Marker for the hidden popup panel; the reparent target for declared items.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct MenuPopupFrame;

// Marker for the check gutter glyph.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct MenuCheckIcon;

// Marker for the submenu caret glyph.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct MenuCaretIcon;

// Marker for the shortcut hint caption.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct MenuShortcutText;

// What a menu button is, derived from where it sits.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MenuButtonRole {
    // Opens a menu below itself: a bar child or a standalone dropdown button.
    Bar,
    // A pickable row inside a menu.
    Item,
    // A row that opens a nested menu beside itself.
    Submenu,
}

// Present while this button's menu is open. `focus` is where keyboard focus
// should land in the popup; `None` (hover-opened submenus) leaves focus alone.
#[derive(Component, Clone, Copy)]
pub(crate) struct MenuOpen {
    pub(crate) focus: Option<NavAction>,
}

// Marks menu buttons whose popups the imm layer builds and tears down itself:
// the classifier and the child-adoption step leave them alone.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct ImmMenuManaged;

// On an imm popup frame: the menu button it belongs to. The frame is unrooted
// (not a descendant of its anchor), so event routing and ancestor walks jump
// through this instead of `ChildOf`.
#[derive(Component, Clone, Copy)]
pub(crate) struct MenuAnchorLink(pub(crate) Entity);

// A just-opened submenu popup waiting for its first-item focus; the headless
// `MenuFocusState` equivalent for the popups plume manages itself.
#[derive(Component, Clone, Copy)]
struct SubmenuOpening(NavAction);

// The popup frame in this menu button's own chrome (not one adopted with an item).
fn own_frame(
    root: Entity,
    q_children: &Query<&Children>,
    q_socket: &Query<(), With<PopupSocket>>,
    q_frame: &Query<(), With<MenuPopupFrame>>,
) -> Option<Entity> {
    let socket = q_children
        .get(root)
        .ok()?
        .iter()
        .copied()
        .find(|&child| q_socket.contains(child))?;
    q_children
        .get(socket)
        .ok()?
        .iter()
        .copied()
        .find(|&child| q_frame.contains(child))
}

// Move declared children (items, separators) into the hidden popup frame, so
// apps write `Children [ … ]` while observers stay on the entities they declared.
fn adopt_menu_children(
    q_roots: Query<(Entity, &Children), (With<MenuButtonRow>, Without<ImmMenuManaged>)>,
    q_chrome: Query<(), With<MenuChrome>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_frame: Query<(), With<MenuPopupFrame>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for (root, children) in q_roots.iter() {
        let mut strays = children
            .iter()
            .copied()
            .filter(|&child| !q_chrome.contains(child))
            .peekable();
        if strays.peek().is_none() {
            continue;
        }
        let Some(frame) = own_frame(root, &q_children, &q_socket, &q_frame) else {
            continue;
        };
        for stray in strays {
            commands.entity(stray).insert(ChildOf(frame));
        }
    }
}

// Popover placements: a bar menu drops below its button, edges flush; a submenu
// opens beside its row, top edges flush.
fn popover_for(role: MenuButtonRole) -> Popover {
    let sides: &[PopoverSide] = match role {
        MenuButtonRole::Submenu => &[PopoverSide::Right, PopoverSide::Left],
        _ => &[PopoverSide::Bottom, PopoverSide::Top],
    };
    Popover {
        positions: sides
            .iter()
            .map(|&side| PopoverPlacement {
                side,
                align: PopoverAlign::Start,
                gap: 2.0,
            })
            .collect(),
        window_margin: 10.0,
    }
}

// Assign roles from position and keep the headless behavior markers in step:
// bar buttons are `MenuButton`s over a focus-tracked `MenuPopup`; items are
// `MenuItem`s; submenus are neither — plume drives their popups itself.
fn classify_menu_buttons(
    q_buttons: Query<
        (Entity, Option<&MenuButtonRole>, Option<&ChildOf>),
        (With<MenuButtonRow>, Without<ImmMenuManaged>),
    >,
    q_is_button: Query<(), With<MenuButtonRow>>,
    q_frame: Query<(), With<MenuPopupFrame>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for (root, role_now, child_of) in q_buttons.iter() {
        let parent = child_of.map(ChildOf::parent);
        // A direct child of another menu button is awaiting adoption.
        if parent.is_some_and(|parent| q_is_button.contains(parent)) {
            continue;
        }
        let in_menu = parent.is_some_and(|parent| q_frame.contains(parent));
        let has_menu = own_frame(root, &q_children, &q_socket, &q_frame)
            .and_then(|frame| q_children.get(frame).ok())
            .is_some_and(|children| !children.is_empty());
        let role = match (in_menu, has_menu) {
            (false, _) => MenuButtonRole::Bar,
            (true, false) => MenuButtonRole::Item,
            (true, true) => MenuButtonRole::Submenu,
        };
        if role_now == Some(&role) {
            continue;
        }
        commands.entity(root).insert(role);
        let Some(frame) = own_frame(root, &q_children, &q_socket, &q_frame) else {
            continue;
        };
        match role {
            MenuButtonRole::Bar => {
                commands
                    .entity(root)
                    .insert(MenuButton)
                    .remove::<MenuItem>();
                commands.entity(frame).insert((
                    MenuPopup {
                        layout: MenuLayout::Column,
                    },
                    popover_for(role),
                ));
            }
            MenuButtonRole::Item => {
                commands
                    .entity(root)
                    .insert(MenuItem)
                    .remove::<(MenuButton, Button, ActivateOnPress)>();
            }
            MenuButtonRole::Submenu => {
                commands
                    .entity(root)
                    .insert(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))
                    .remove::<(MenuButton, Button, ActivateOnPress, MenuItem)>();
                commands.entity(frame).insert((
                    TabGroup::modal(),
                    AccessibilityNode(accesskit::Node::new(Role::MenuListPopup)),
                    popover_for(role),
                ));
            }
        }
    }
}

fn on_menu_opened(
    ev: On<Add, MenuOpen>,
    q_open: Query<(&MenuOpen, Option<&MenuButtonRole>)>,
    q_children: Query<&Children>,
    q_socket: Query<(), With<PopupSocket>>,
    q_frame: Query<(), With<MenuPopupFrame>>,
    mut q_frames: Query<
        (&mut Node, &mut Visibility, Option<&mut MenuFocusState>),
        With<MenuPopupFrame>,
    >,
    mut commands: Commands,
) {
    let Ok((open, role)) = q_open.get(ev.entity) else {
        return;
    };
    let Some(frame) = own_frame(ev.entity, &q_children, &q_socket, &q_frame) else {
        return;
    };
    let Ok((mut node, mut visibility, focus_state)) = q_frames.get_mut(frame) else {
        return;
    };
    node.display = Display::Flex;
    *visibility = Visibility::Inherited;
    match role {
        Some(MenuButtonRole::Submenu) => {
            if let Some(nav) = open.focus {
                commands.entity(frame).insert(SubmenuOpening(nav));
            }
        }
        _ => {
            let opening = MenuFocusState::Opening(open.focus.unwrap_or(NavAction::First));
            match focus_state {
                Some(mut state) => *state = opening,
                None => {
                    commands.entity(frame).insert(opening);
                }
            }
        }
    }
}

fn on_menu_closed(
    ev: On<Remove, MenuOpen>,
    q_children: Query<&Children>,
    q_socket: Query<(), With<PopupSocket>>,
    q_frame: Query<(), With<MenuPopupFrame>>,
    q_open: Query<(), (With<MenuOpen>, With<MenuButtonRow>)>,
    mut q_frames: Query<
        (&mut Node, &mut Visibility, Option<&mut MenuFocusState>),
        With<MenuPopupFrame>,
    >,
    mut commands: Commands,
) {
    let Some(frame) = own_frame(ev.entity, &q_children, &q_socket, &q_frame) else {
        return;
    };
    let Ok((mut node, mut visibility, focus_state)) = q_frames.get_mut(frame) else {
        return;
    };
    node.display = Display::None;
    *visibility = Visibility::Hidden;
    if let Some(mut state) = focus_state {
        *state = MenuFocusState::Closed;
    }
    commands.entity(frame).remove::<SubmenuOpening>();
    // Open submenus below this menu close with it.
    for descendant in q_children.iter_descendants(frame) {
        if q_open.contains(descendant) {
            commands.entity(descendant).remove::<MenuOpen>();
        }
    }
}

// Routes the headless menu events that bubble up from this button and its popup.
fn menu_root_on_menu_event(
    mut ev: On<MenuEvent>,
    q_state: Query<
        (
            Option<&MenuButtonRole>,
            Has<MenuOpen>,
            Has<InteractionDisabled>,
        ),
        With<MenuButtonRow>,
    >,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let root = ev.source;
    let Ok((role, open, disabled)) = q_state.get(root) else {
        return;
    };
    let bar = role == Some(&MenuButtonRole::Bar);
    match ev.event().action {
        MenuAction::Toggle if bar => {
            ev.propagate(false);
            if open {
                commands.entity(root).remove::<MenuOpen>();
            } else if !disabled {
                commands.entity(root).insert(MenuOpen {
                    focus: Some(NavAction::First),
                });
            }
        }
        MenuAction::Open(nav) if bar => {
            ev.propagate(false);
            if !open && !disabled {
                commands.entity(root).insert(MenuOpen { focus: Some(nav) });
            }
        }
        MenuAction::CloseAll => {
            // Keep propagating: every open ancestor closes too.
            if open {
                commands.entity(root).remove::<MenuOpen>();
            }
        }
        MenuAction::FocusRoot if bar => {
            ev.propagate(false);
            focus.set(root, FocusCause::Navigated);
        }
        _ => {}
    }
}

// While a menu in the bar is open, hovering a sibling button switches to it.
fn bar_hover_switch(
    q_changed: Query<
        (
            Entity,
            &Hovered,
            &MenuButtonRole,
            Has<MenuOpen>,
            Has<InteractionDisabled>,
            &ChildOf,
        ),
        (Changed<Hovered>, With<MenuButtonRow>),
    >,
    q_bar: Query<(), With<PlumeMenuBar>>,
    q_children: Query<&Children>,
    q_open: Query<(), (With<MenuOpen>, With<MenuButtonRow>)>,
    mut commands: Commands,
) {
    for (root, hovered, role, open, disabled, child_of) in q_changed.iter() {
        if *role != MenuButtonRole::Bar
            || !hovered.0
            || open
            || disabled
            || !q_bar.contains(child_of.parent())
        {
            continue;
        }
        let Ok(siblings) = q_children.get(child_of.parent()) else {
            continue;
        };
        let mut any_open = false;
        for sibling in siblings.iter().copied().filter(|&sibling| sibling != root) {
            if q_open.contains(sibling) {
                any_open = true;
                commands.entity(sibling).remove::<MenuOpen>();
            }
        }
        if any_open {
            commands.entity(root).insert(MenuOpen {
                focus: Some(NavAction::First),
            });
        }
    }
}

// Hovering a row opens its submenu and closes any open submenu of a sibling.
fn submenu_hover(
    q_changed: Query<
        (
            Entity,
            &Hovered,
            &MenuButtonRole,
            Has<MenuOpen>,
            Has<InteractionDisabled>,
            &ChildOf,
        ),
        (Changed<Hovered>, With<MenuButtonRow>),
    >,
    q_frame: Query<(), With<MenuPopupFrame>>,
    q_children: Query<&Children>,
    q_open: Query<(), (With<MenuOpen>, With<MenuButtonRow>)>,
    mut commands: Commands,
) {
    for (root, hovered, role, open, disabled, child_of) in q_changed.iter() {
        if !hovered.0 || !q_frame.contains(child_of.parent()) {
            continue;
        }
        if let Ok(siblings) = q_children.get(child_of.parent()) {
            for sibling in siblings.iter().copied().filter(|&sibling| sibling != root) {
                if q_open.contains(sibling) {
                    commands.entity(sibling).remove::<MenuOpen>();
                }
            }
        }
        if *role == MenuButtonRole::Submenu && !open && !disabled {
            commands.entity(root).insert(MenuOpen { focus: None });
        }
    }
}

// A click on a submenu row opens it (hover normally has already).
fn submenu_on_click(
    mut ev: On<Pointer<Click>>,
    q_subs: Query<(&MenuButtonRole, Has<MenuOpen>, Has<InteractionDisabled>), With<MenuButtonRow>>,
    mut commands: Commands,
) {
    let Ok((role, open, disabled)) = q_subs.get(ev.entity) else {
        return;
    };
    if *role != MenuButtonRole::Submenu {
        return;
    }
    ev.propagate(false);
    if !open && !disabled {
        commands.entity(ev.entity).insert(MenuOpen { focus: None });
    }
}

// The keyboard reach the headless layer doesn't give us: ArrowRight/Enter opens
// a submenu, ArrowLeft closes back out of one, and Left/Right otherwise switch
// between the bar's menus while one is open.
fn menu_on_key(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_state: Query<(&MenuButtonRole, Has<MenuOpen>, Has<InteractionDisabled>), With<MenuButtonRow>>,
    q_bar: Query<(), With<PlumeMenuBar>>,
    q_links: Query<&MenuAnchorLink>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if ev.focused_entity != ev.original_event_target() {
        return;
    }
    let input = &ev.event().input;
    if input.repeat || input.state != ButtonState::Pressed {
        return;
    }
    let focused = ev.focused_entity;

    // One hop toward the menu bar; an unrooted imm frame jumps to its anchor.
    let step = |entity: Entity| -> Option<Entity> {
        match q_links.get(entity) {
            Ok(link) => Some(link.0),
            Err(_) => q_parents.get(entity).ok().map(ChildOf::parent),
        }
    };
    let ancestors =
        |start: Entity| core::iter::successors(step(start), move |&entity| step(entity));

    let enabled_submenu = |entity: Entity| {
        q_state
            .get(entity)
            .is_ok_and(|(role, _, disabled)| *role == MenuButtonRole::Submenu && !disabled)
    };
    // The open submenu the focus is inside, if any.
    let enclosing_submenu = || {
        ancestors(focused).find(|&ancestor| {
            q_state
                .get(ancestor)
                .is_ok_and(|(role, open, _)| *role == MenuButtonRole::Submenu && open)
        })
    };
    // The open bar menu the focus is inside: (bar, its open button).
    let enclosing_bar_menu = || {
        ancestors(focused)
            .find(|&ancestor| {
                q_state
                    .get(ancestor)
                    .is_ok_and(|(role, open, _)| *role == MenuButtonRole::Bar && open)
            })
            .and_then(|open_root| {
                let bar = q_parents.get(open_root).ok()?.parent();
                q_bar.contains(bar).then_some((bar, open_root))
            })
    };
    let mut switch_bar_menu = |bar: Entity, current: Entity, forward: bool| {
        let Ok(children) = q_children.get(bar) else {
            return;
        };
        let buttons: Vec<Entity> = children
            .iter()
            .copied()
            .filter(|&child| {
                q_state
                    .get(child)
                    .is_ok_and(|(role, _, disabled)| *role == MenuButtonRole::Bar && !disabled)
            })
            .collect();
        let Some(index) = buttons.iter().position(|&button| button == current) else {
            return;
        };
        let step = if forward { 1 } else { buttons.len() - 1 };
        let next = buttons[(index + step) % buttons.len()];
        if next != current {
            commands.entity(current).remove::<MenuOpen>();
            commands.entity(next).insert(MenuOpen {
                focus: Some(NavAction::First),
            });
        }
    };

    match input.key_code {
        KeyCode::Enter | KeyCode::Space if enabled_submenu(focused) => {
            ev.propagate(false);
            commands.entity(focused).insert(MenuOpen {
                focus: Some(NavAction::First),
            });
        }
        KeyCode::ArrowRight => {
            if enabled_submenu(focused) {
                ev.propagate(false);
                commands.entity(focused).insert(MenuOpen {
                    focus: Some(NavAction::First),
                });
            } else if let Some((bar, current)) = enclosing_bar_menu() {
                ev.propagate(false);
                switch_bar_menu(bar, current, true);
            }
        }
        KeyCode::ArrowLeft => {
            if let Some(submenu) = enclosing_submenu() {
                ev.propagate(false);
                commands.entity(submenu).remove::<MenuOpen>();
                focus.set(submenu, FocusCause::Navigated);
            } else if let Some((bar, current)) = enclosing_bar_menu() {
                ev.propagate(false);
                switch_bar_menu(bar, current, false);
            }
        }
        _ => {}
    }
}

// Focus a just-opened submenu's first item. Runs after visibility propagation
// (the popup was hidden until this frame) and before focus-change events, the
// same window the headless `MenuFocusState` machinery uses.
fn submenu_acquire_focus(
    q_opening: Query<(Entity, &SubmenuOpening), With<MenuPopupFrame>>,
    tab_navigation: TabNavigation,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for (frame, opening) in q_opening.iter() {
        if let Ok(next) = tab_navigation.initialize(frame, opening.0) {
            focus.set(next, FocusCause::Navigated);
        }
        commands.entity(frame).remove::<SubmenuOpening>();
    }
}

// Picking a checkable item toggles its check besides the headless `Activate`.
fn menu_item_toggle_check(
    ev: On<Activate>,
    q_items: Query<
        (Has<Checked>, Has<InteractionDisabled>, &MenuButtonRole),
        (With<MenuButtonRow>, With<Checkable>),
    >,
    mut commands: Commands,
) {
    let Ok((checked, disabled, role)) = q_items.get(ev.entity) else {
        return;
    };
    if disabled || *role != MenuButtonRole::Item {
        return;
    }
    if checked {
        commands.entity(ev.entity).remove::<Checked>();
    } else {
        commands.entity(ev.entity).insert(Checked);
    }
    commands.trigger(ValueChange {
        source: ev.entity,
        value: !checked,
        is_final: true,
    });
}

// Programmatic checked state, the counterpart of the emitted `ValueChange<bool>`.
fn menu_on_set_checked(
    ev: On<SetValue<bool>>,
    q_items: Query<Has<Checked>, (With<MenuButtonRow>, With<Checkable>)>,
    mut commands: Commands,
) {
    let Ok(checked) = q_items.get(ev.entity) else {
        return;
    };
    if ev.value == checked {
        return;
    }
    if ev.value {
        commands.entity(ev.entity).insert(Checked);
    } else {
        commands.entity(ev.entity).remove::<Checked>();
    }
}

fn update_menu_button_styles(
    q_changed: Query<
        (
            Entity,
            &MenuButtonRole,
            &Hovered,
            Has<MenuOpen>,
            Has<Checked>,
            Has<InteractionDisabled>,
            Option<&ThemeBackgroundToken>,
            &InheritableThemeTextToken,
        ),
        (
            With<MenuButtonRow>,
            Or<(
                Changed<MenuButtonRole>,
                Changed<Hovered>,
                Changed<MenuOpen>,
                Changed<Checked>,
                Changed<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<MenuCheckIcon>>,
    q_caret: Query<(), With<MenuCaretIcon>>,
    q_shortcut: Query<(&Text, &InheritableThemeTextToken), With<MenuShortcutText>>,
    mut q_nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (root, role, hovered, open, checked, disabled, bg_now, text_now) in q_changed.iter() {
        set_menu_button_styles(
            root,
            *role,
            hovered.0,
            open,
            checked,
            disabled,
            bg_now,
            text_now,
            &q_children,
            &q_check,
            &q_caret,
            &q_shortcut,
            &mut q_nodes,
            &mut commands,
        );
    }
}

fn update_menu_button_styles_remove(
    q_buttons: Query<
        (
            Entity,
            &MenuButtonRole,
            &Hovered,
            Has<MenuOpen>,
            Has<Checked>,
            Has<InteractionDisabled>,
            Option<&ThemeBackgroundToken>,
            &InheritableThemeTextToken,
        ),
        With<MenuButtonRow>,
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<MenuCheckIcon>>,
    q_caret: Query<(), With<MenuCaretIcon>>,
    q_shortcut: Query<(&Text, &InheritableThemeTextToken), With<MenuShortcutText>>,
    mut q_nodes: Query<&mut Node>,
    mut removed_open: RemovedComponents<MenuOpen>,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_open
        .read()
        .chain(removed_checked.read())
        .chain(removed_disabled.read())
        .for_each(|ent| {
            if let Ok((root, role, hovered, open, checked, disabled, bg_now, text_now)) =
                q_buttons.get(ent)
            {
                set_menu_button_styles(
                    root,
                    *role,
                    hovered.0,
                    open,
                    checked,
                    disabled,
                    bg_now,
                    text_now,
                    &q_children,
                    &q_check,
                    &q_caret,
                    &q_shortcut,
                    &mut q_nodes,
                    &mut commands,
                );
            }
        });
}

fn set_menu_button_styles(
    root: Entity,
    role: MenuButtonRole,
    hovered: bool,
    open: bool,
    checked: bool,
    disabled: bool,
    bg_now: Option<&ThemeBackgroundToken>,
    text_now: &InheritableThemeTextToken,
    q_children: &Query<&Children>,
    q_check: &Query<(), With<MenuCheckIcon>>,
    q_caret: &Query<(), With<MenuCaretIcon>>,
    q_shortcut: &Query<(&Text, &InheritableThemeTextToken), With<MenuShortcutText>>,
    q_nodes: &mut Query<&mut Node>,
    commands: &mut Commands,
) {
    let bar = role == MenuButtonRole::Bar;
    let (bg_token, text_token) = if bar {
        (
            if disabled {
                None
            } else if open || hovered {
                Some(tokens::MENU_BUTTON_BG_HOVER)
            } else {
                None
            },
            if disabled {
                tokens::MENU_BUTTON_TEXT_DISABLED
            } else {
                tokens::MENU_BUTTON_TEXT
            },
        )
    } else {
        (
            (!disabled && (hovered || open)).then_some(tokens::MENU_ITEM_BG_HOVER),
            if disabled {
                tokens::MENU_ITEM_TEXT_DISABLED
            } else {
                tokens::MENU_ITEM_TEXT
            },
        )
    };

    set_optional_background(commands, root, bg_now, bg_token);
    if text_now.0 != text_token {
        commands
            .entity(root)
            .insert(InheritableThemeTextToken(text_token));
    }
    commands
        .entity(root)
        .insert(EntityCursor::System(match disabled {
            true => bevy::window::SystemCursorIcon::NotAllowed,
            false => bevy::window::SystemCursorIcon::Pointer,
        }));

    // Bar buttons round like buttons; rows in a menu run square, edge to edge.
    let radius = if bar {
        BorderRadius::all(size::CORNER_RADIUS_SMALL)
    } else {
        BorderRadius::ZERO
    };
    if let Ok(mut node) = q_nodes.get_mut(root)
        && node.border_radius != radius
    {
        node.border_radius = radius;
    }

    let Ok(children) = q_children.get(root) else {
        return;
    };
    let set_display = |node: &mut Node, wanted: Display| {
        if node.display != wanted {
            node.display = wanted;
        }
    };
    for child in children.iter().copied() {
        if q_check.contains(child) {
            if let Ok(mut node) = q_nodes.get_mut(child) {
                set_display(&mut node, if bar { Display::None } else { Display::Flex });
            }
            commands.entity(child).insert(match !bar && checked {
                true => Visibility::Inherited,
                false => Visibility::Hidden,
            });
        } else if q_caret.contains(child) {
            if let Ok(mut node) = q_nodes.get_mut(child) {
                set_display(&mut node, if bar { Display::None } else { Display::Flex });
            }
            commands.entity(child).insert(match role {
                MenuButtonRole::Submenu => Visibility::Inherited,
                _ => Visibility::Hidden,
            });
        } else if let Ok((text, text_now)) = q_shortcut.get(child) {
            let shown = !bar && !text.0.is_empty();
            if let Ok(mut node) = q_nodes.get_mut(child) {
                set_display(&mut node, if shown { Display::Flex } else { Display::None });
            }
            // Dim while usable; grayed like the label only when actually disabled.
            let shortcut_token = match disabled {
                true => tokens::MENU_ITEM_TEXT_DISABLED,
                false => tokens::TEXT_DIM,
            };
            if text_now.0 != shortcut_token {
                commands
                    .entity(child)
                    .insert(InheritableThemeTextToken(shortcut_token));
            }
        }
    }
}

// Breathing room around a menu's dividers. Keyed on the parent change so both
// paths are covered once: retained separators when adopted into the frame, imm
// ones when built inside it.
fn space_menu_separators(
    mut q_separators: Query<(Entity, &ChildOf, &mut Node), (With<Separator>, Changed<ChildOf>)>,
    q_frames: Query<(), With<MenuPopupFrame>>,
    mut commands: Commands,
) {
    for (separator, child_of, mut node) in q_separators.iter_mut() {
        if !q_frames.contains(child_of.parent()) {
            continue;
        }
        node.margin = UiRect::vertical(size::SPACE_TIGHT);
        // Em margins need the chain's `EmSize`.
        commands.entity(separator).insert(TextStyleRelay);
    }
}

// Plugin which runs the [`PlumeMenuBar`] and [`PlumeMenuButton`] controls.
pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_menu_button_styles, update_menu_button_styles_remove)
                .in_set(PickingSystems::Last),
        )
        .add_systems(
            Update,
            (
                adopt_menu_children,
                classify_menu_buttons,
                bar_hover_switch,
                submenu_hover,
                space_menu_separators,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            submenu_acquire_focus
                .after(VisibilitySystems::VisibilityPropagate)
                .before(MenuFocusSystem)
                .before(InputFocusSystems::FocusChangeEvents),
        )
        .add_observer(on_menu_opened)
        .add_observer(on_menu_closed)
        .add_observer(submenu_on_click)
        .add_observer(menu_on_key)
        .add_observer(menu_item_toggle_check)
        .add_observer(menu_on_set_checked);
    }
}
