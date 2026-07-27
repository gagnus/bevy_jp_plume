//! Movable floating dialog with a draggable title bar and close button.
use bevy::color::{Alpha, Srgba};
use bevy::ecs::name::Name;
use bevy::ecs::{
    component::Component, event::EntityEvent, hierarchy::Children, observer::On,
    reflect::ReflectComponent, system::Commands,
};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on, template_value};
use bevy::text::FontWeight;
use bevy::ui::{
    AlignItems, BorderRadius, BoxShadow, Display, FlexDirection, JustifyContent, LayoutConfig,
    Node, PositionType, UiRect, Val,
};
use bevy::ui_widgets::{Activate, ControlOrientation, Dialog, DialogDragHandle, RequestClose};

use crate::{
    constants::{font_awesome, fonts, size},
    containers::{flex_spacer, scroll_content, scroll_frame, scroll_viewport, scrollbar_node},
    controls::{ButtonVariant, PlumeScrollbar, PlumeToolButton},
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
    /// Body padding
    pub body_padding: UiRect,
}

impl Default for PlumeDialogProps {
    fn default() -> Self {
        Self {
            title: Box::new(bsn_list!()),
            contents: Box::new(bsn_list!()),
            width: Val::Auto,
            height: Val::Auto,
            max_height: Val::Auto,
            left: size::DEFAULT_DIALOG_POS.x,
            top: size::DEFAULT_DIALOG_POS.y,
            closable: true,
            movable: true,
            header: true,
            body_padding: UiRect::all(size::PAD),
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
            body_padding,
        } = props;
        // A bounded dialog scrolls its body; an unbounded one holds the contents
        // directly and spawns no scroll machinery.
        let body: Box<dyn SceneList> = if height != Val::Auto || max_height != Val::Auto {
            Box::new(bsn_list!((
                scroll_frame()
                Children [
                    (
                        #inner
                        scroll_viewport()
                        Children [
                            (
                                scroll_content()
                                Children [
                                    {contents}
                                ]
                            )
                        ]
                    ),
                    (
                        @PlumeScrollbar {
                            @target: #inner,
                            @orientation: {ControlOrientation::Vertical},
                        }
                        scrollbar_node()
                    ),
                ]
            )))
        } else {
            contents
        };
        bsn! {
            dialog_frame(DialogChrome {
                name: "PlumeDialog".into(),
                // The public dialog builds its whole body eagerly and hands the
                // frame a padded `PlumeDialogBody` wrapping it.
                body: Box::new(bsn_list!((
                    @PlumeDialogBody { @padding: {body_padding} }
                    Children [
                        {body}
                    ]
                ))),
                header: header.then(|| DialogHeader {
                    title,
                    closable,
                    movable,
                }),
                width,
                height,
                max_height,
                inset: UiRect {
                    left,
                    top,
                    ..UiRect::AUTO
                },
            })
            // Closing despawns the window.
            on(|close: On<RequestClose>, mut commands: Commands| {
                commands.entity(close.event_target()).despawn();
            })
        }
    }
}

// Set on a dialog or popup root when a close is requested; the surface's owner
// (the imm layer, or a control's observer) closes it rather than the requester.
#[derive(Component)]
pub(crate) struct CloseRequested;

/// Chrome-level input for [`dialog_frame`], kept distinct from the public
/// [`PlumeDialogProps`] so `body` has exactly one meaning — the finished body,
/// inserted verbatim — and body padding never reaches the frame (it lives on the
/// body's [`PlumeDialogBody`]).
pub(crate) struct DialogChrome {
    /// Debug name to give the entity
    pub name: Name,
    /// Finished body slot, inserted into the frame verbatim. The public dialog hands
    /// over a padded [`PlumeDialogBody`]; the imm layer hands over an empty slot and
    /// reconciles the body itself.
    pub body: Box<dyn SceneList>,
    /// The title bar, or `None` for a bare floating panel — no ✕, no drag, and no
    /// header-height floor.
    pub header: Option<DialogHeader>,
    /// How wide the frame should be.
    pub width: Val,
    /// Fixed outer height, title bar included. `Val::Auto` hugs the content.
    pub height: Val,
    /// Ceiling on the outer height. `Val::Auto` for no ceiling.
    pub max_height: Val,
    /// Initial offsets from each viewport edge (the frame is absolutely
    /// positioned); an `Auto` side is unanchored, so the opposite one places it.
    pub inset: UiRect,
}

/// Title-bar configuration for a [`DialogChrome`] that has one.
pub(crate) struct DialogHeader {
    /// Title content shown in the drag bar (e.g. `bsn! { caption("…") }`).
    pub title: Box<dyn SceneList>,
    /// `false` omits the ✕ button, for dialogs dismissed only by an action button.
    pub closable: bool,
    /// `false` omits the drag handle, pinning the dialog in place.
    pub movable: bool,
}

/// Dialog chrome (frame, optional title bar, ✕) shared by the public [`PlumeDialog`]
/// and the imm layer, with no close behavior — callers attach their own
/// `RequestClose` observer.
pub(crate) fn dialog_frame(chrome: DialogChrome) -> impl Scene {
    let DialogChrome {
        name,
        body,
        header,
        width,
        height,
        max_height,
        inset,
    } = chrome;
    // The header-height floor only exists to keep a `max_height` from crushing the
    // title bar; a headerless panel has no such reserve.
    let frame_min_height = if header.is_some() {
        size::DIALOG_HEADER_HEIGHT
    } else {
        Val::ZERO
    };
    let title_bar = header.map(
        |DialogHeader {
             title,
             closable,
             movable,
         }| {
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
        },
    );
    bsn! {
            template_value(name)
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                position_type: PositionType::Absolute,
                left: {inset.left},
                top: {inset.top},
                right: {inset.right},
                bottom: {inset.bottom},
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
            InheritableThemeTextColor(tokens::TEXT_DIM)
            BoxShadow::new(
                Srgba::BLACK.with_alpha(0.7).into(),
                size::GAP / 2.0,
                size::GAP,
                size::GAP / 2.0,
                size::GAP * 2.0,
            )
            LayoutConfig {
                use_rounding: false,
            }
            Children [
                {title_bar},
                {body}
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

/// Props used to construct a [`PlumeDialogBody`] scene.
pub struct PlumeDialogBodyProps {
    /// Padding inside the body, around the content.
    pub padding: UiRect,
}

impl Default for PlumeDialogBodyProps {
    fn default() -> Self {
        Self {
            padding: UiRect::all(size::PAD),
        }
    }
}

/// Central body section for a dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeDialogBodyProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialogBody;

impl PlumeDialogBody {
    /// Scene function for dialog body.
    pub fn scene(props: PlumeDialogBodyProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: size::GAP,
                padding: {props.padding},
                // Shrinking below the content size lets a bounded dialog scroll
                // instead of pushing content out the bottom. Inert while `Auto`.
                flex_grow: 1.0,
                min_height: Val::ZERO,
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}
