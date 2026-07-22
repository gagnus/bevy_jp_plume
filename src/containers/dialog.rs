//! Movable floating dialog with a draggable title bar and close button.
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    component::Component, entity::Entity, event::EntityEvent, hierarchy::Children, observer::On,
    reflect::ReflectComponent, system::Commands, template::EntityTemplate,
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on};
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, BoxShadow, Display, FlexDirection, JustifyContent, LayoutConfig,
    Node, Overflow, PositionType, UiRect, Val, px,
};
use bevy_ui_widgets::{
    Activate, ControlOrientation, Dialog, DialogDragHandle, RequestClose, ScrollArea,
};

use crate::{
    constants::{font_awesome, fonts, size},
    containers::flex_spacer,
    controls::{ButtonVariant, PlumeScrollbar, PlumeToolButton, ScrollbarGutter},
    display::fa_icon,
    font_styles::InheritableFont,
    theme::{Flat, InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// Props used to construct a [`PlumeDialog`] scene.
pub struct PlumeDialogProps {
    /// Title content shown in the window's drag bar (e.g. `bsn! { caption("…") }`).
    pub title: Box<dyn SceneList>,
    /// Body content of the window.
    pub contents: Box<dyn SceneList>,
    /// How wide the window should be.
    pub width: Val,
    /// Fixed outer height, title bar included. `Val::Auto` hugs the content.
    pub height: Val,
    /// Ceiling on the outer height: the window hugs its content until it would
    /// exceed this, then stops growing. `Val::Auto` for no ceiling. Floored at
    /// [`size::DIALOG_HEADER_HEIGHT`] so the title bar can never overflow.
    pub max_height: Val,
    /// Initial left offset (the window is absolutely positioned).
    pub left: Val,
    /// Initial top offset.
    pub top: Val,
    /// `false` omits the ✕ button, for dialogs dismissed only by an action button.
    pub closable: bool,
    /// `false` omits the drag handle, pinning the dialog in place.
    pub movable: bool,
    /// `false` omits the whole title bar (with it, the title, ✕ and drag), leaving a
    /// bare floating panel — see the imm `panel`. Also drops the header-height floor.
    pub header: bool,
}

impl Default for PlumeDialogProps {
    fn default() -> Self {
        Self {
            title: Box::new(bsn_list!()),
            contents: Box::new(bsn_list!()),
            width: Val::Auto,
            height: Val::Auto,
            max_height: Val::Auto,
            left: px(120),
            top: px(120),
            closable: true,
            movable: true,
            header: true,
        }
    }
}

/// A movable floating dialog with a draggable title bar and a close button.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeDialogProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialog;

impl PlumeDialog {
    /// Scene function for a floating dialog window.
    pub fn scene(props: PlumeDialogProps) -> impl Scene {
        let PlumeDialogProps {
            title,
            contents,
            width,
            height,
            max_height,
            left,
            top,
            closable,
            movable,
            header,
        } = props;
        // A bounded dialog scrolls its body; an unbounded one holds the contents
        // directly and spawns no scroll machinery.
        let body: Box<dyn SceneList> = if height != Val::Auto || max_height != Val::Auto {
            Box::new(bsn_list!((
                dialog_scroll_frame()
                Children [
                    (
                        #inner
                        dialog_scroll_area()
                        Children [
                            {contents}
                        ]
                    ),
                    (
                        @PlumeScrollbar {
                            @target: #inner,
                            @orientation: {ControlOrientation::Vertical},
                        }
                        dialog_scrollbar_node()
                    ),
                ]
            )))
        } else {
            contents
        };
        bsn! {
            dialog_frame(PlumeDialogProps {
                title,
                width,
                height,
                max_height,
                left,
                top,
                closable,
                movable,
                header,
                // Empty for the imm layer, which reconciles the body itself.
                contents: Box::new(bsn_list!((
                    @PlumeDialogBody
                    Children [
                        {body}
                    ]
                ))),
            })
            // Closing despawns the window.
            on(|close: On<RequestClose>, mut commands: Commands| {
                commands.entity(close.event_target()).despawn();
            })
        }
    }
}

/// Set on the dialog root when a close is requested; the imm layer's dialogs
/// carry an observer that inserts this instead of despawning.
#[derive(Component)]
pub(crate) struct DialogCloseRequested;

/// Dialog chrome (frame, title bar, ✕) shared by the public [`PlumeDialog`] and
/// the imm layer, with no close behavior — callers attach their own `RequestClose`
/// observer. `props.contents` is inserted as the body slot verbatim (the public
/// dialog wraps it in a [`PlumeDialogBody`]; the imm layer leaves it empty and
/// reconciles the body itself).
pub(crate) fn dialog_frame(props: PlumeDialogProps) -> impl Scene {
    let PlumeDialogProps {
        title,
        contents,
        width,
        height,
        max_height,
        left,
        top,
        closable,
        movable,
        header,
    } = props;
    // The header-height floor only exists to keep a `max_height` from crushing the
    // title bar; a headerless panel has no such reserve.
    let frame_min_height = if header {
        size::DIALOG_HEADER_HEIGHT
    } else {
        Val::ZERO
    };
    let title_bar = header.then(|| {
        bsn! {
            // Title bar; dragging it moves the window. Same chrome as the section
            // header; the dialog is distinguished by its drop shadow, not a
            // different header.
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    padding: UiRect::horizontal(size::PAD * 2.0),
                    min_height: size::DIALOG_HEADER_HEIGHT,
                    column_gap: size::GAP,
                    border: UiRect::bottom(size::CONTAINER_BORDER),
                    border_radius: BorderRadius::top(size::DIALOG_RADIUS),
                }
                {movable.then(|| bsn!(DialogDragHandle))}
                InheritableThemeTextColor(tokens::DIALOG_HEADER_TEXT)
                ThemeBackgroundColor(tokens::DIALOG_HEADER_BG)
                ThemeBorderColor(tokens::DIALOG_BORDER)
                InheritableFont {
                    font: fonts::REGULAR,
                    font_size: size::MEDIUM_FONT,
                    weight: FontWeight::NORMAL,
                }
                Children [
                    {title},
                    // Spacer, not SpaceBetween: a multi-entity title stays grouped at the start.
                    flex_spacer(),
                    {closable.then(|| bsn_list!(@PlumeDialogClose))}
                ]
            )
        }
    });
    bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                position_type: PositionType::Absolute,
                left: {left},
                top: {top},
                border_radius: size::DIALOG_RADIUS,
                border: UiRect::all(size::CONTAINER_BORDER),
                width: {width},
                height: {height},
                max_height: {max_height},
                // Flexbox resolves `min` after `max`, so this floor survives a
                // `max_height` that would otherwise crush the title bar.
                min_height: {frame_min_height},
            }
            Dialog
            // Tab-traversal scope for the dialog's fields.
            TabGroup::new(0)
            ThemeBackgroundColor(tokens::DIALOG_BG)
            ThemeBorderColor(tokens::DIALOG_BORDER)
            InheritableThemeTextColor(tokens::TEXT_MAIN)
            BoxShadow::new(
                Srgba::BLACK.with_alpha(0.7).into(),
                px(4),
                px(8),
                px(4),
                px(16),
            )
            LayoutConfig {
                use_rounding: false,
            }
            Children [
                {title_bar},
                {contents}
            ]
    }
}

/// Close button for dialog header
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialogClose;

impl PlumeDialogClose {
    /// Scene function for dialog close button.
    pub fn scene() -> impl Scene {
        bsn! {
        @PlumeToolButton {
            @variant: ButtonVariant::Plain,
            @caption: bsn! { fa_icon(font_awesome::solid::XMARK) }
        }
        // Keep the ✕'s hover/press fill flat.
        Flat
        on(|activate: On<Activate>, mut commands: Commands| {
            commands.trigger(RequestClose { source: activate.event_target() });
        })
        }
    }
}

/// Central body section for a dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialogBody;

impl PlumeDialogBody {
    /// Scene function for dialog body.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: size::GAP,
                padding: size::PAD,
                // Shrinking below the content size lets a bounded dialog scroll
                // instead of pushing content out the bottom. Inert while `Auto`.
                flex_grow: 1.0,
                min_height: px(0),
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}

/// Bounded frame inside a height-limited dialog body, holding the scrolling
/// content and the scrollbar that drives it.
///
/// Distinct from [`PlumeDialogBody`] because [`ScrollbarGutter`] *assigns*
/// `padding.right`, which on the body would eat the body's own padding.
pub(crate) fn dialog_scroll_frame() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            min_height: px(0),
        }
        ScrollbarGutter(size::SCROLLBAR_GUTTER)
    }
}

/// The scrolling viewport itself: the dialog's content lands here. Vertical
/// only — a dialog is never allowed to scroll sideways.
pub(crate) fn dialog_scroll_area() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP,
            flex_grow: 1.0,
            min_height: px(0),
            overflow: Overflow::scroll_y(),
        }
        ScrollArea
    }
}

/// Placement shared by both dialog scrollbars: pinned down the trailing edge of
/// [`dialog_scroll_frame`]. The two paths name their viewport differently, so
/// only the placement is shared.
fn dialog_scrollbar_node() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            width: size::SCROLLBAR_WIDTH,
        }
    }
}

/// Vertical scrollbar driving the scroll area at `target`. Hidden, and its
/// gutter reclaimed, whenever the content fits — see
/// `update_scrollbar_visibility`.
pub(crate) fn dialog_scrollbar(target: Entity) -> impl Scene {
    bsn! {
        @PlumeScrollbar {
            @target: {EntityTemplate::from(target)},
            @orientation: {ControlOrientation::Vertical},
        }
        dialog_scrollbar_node()
    }
}
