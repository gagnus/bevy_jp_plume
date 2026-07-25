//! Retained (bsn!) twin of the `debug_settings` example: the same debug panel built
//! from Plume's retained components. Nothing here is wired to anything — it audits
//! how the default-styled components read as a realistic composition.
use bevy::{prelude::*, ui::Checked, ui_widgets::SliderValue};
use bevy_jp_plume::{
    PlumePlugins,
    constants::font_awesome,
    containers::{PlumeDialog, PlumeSection, column, flex_spacer, row, separator},
    controls::{
        ButtonVariant, PlumeButton, PlumeCheckbox, PlumeNumberInput, PlumeSelect, PlumeSlider,
        PlumeToggleSwitch, options_from_strings,
    },
    display::{caption, caption_small_caps, fa_icon},
    theme::ThemeBackgroundColor,
    tokens,
};

#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .add_systems(Startup, scene.spawn());
    common::apply_args(&mut app, false);
    app.run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, root()]
}

fn root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            debug_options_dialog(),
        ]
    }
}

fn debug_options_dialog() -> impl Scene {
    bsn! {
        @PlumeDialog {
            @title: bsn! { caption("Debug Options") },
            @width: px(600),
            @left: px(40),
            @top: px(40),
            @contents: bsn_list! {
                // Two columns side by side: the landscape shape comes from the split,
                // not from padding out one tall column.
                (
                    row()
                    Node { align_items: AlignItems::Start }
                    Children [
                        (
                            debug_column()
                            Children [
                                @PlumeSection {
                                    @header: bsn! { caption_small_caps("Rendering") },
                                    @contents: bsn_list! {
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Wireframe") }
                                        },
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Show colliders") }
                                        }
                                        Checked,
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Freeze frustum culling") }
                                        },
                                        select_row(
                                            "View mode",
                                            ["Lit", "Albedo", "Normals", "Depth", "Overdraw"],
                                        ),
                                        slider_row("Gamma", 0.5, 3.0, 2.2, 2, None),
                                    },
                                },
                                @PlumeSection {
                                    @header: bsn! { caption_small_caps("Physics") },
                                    @contents: bsn_list! {
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Pause simulation") }
                                        },
                                        slider_row("Time scale", 0.0, 2.0, 1.0, 2, None),
                                    },
                                },
                            ]
                        ),
                        (
                            debug_column()
                            Children [
                                @PlumeSection {
                                    @header: bsn! { caption_small_caps("Diagnostics") },
                                    @contents: bsn_list! {
                                        toggle_row("FPS overlay", true),
                                        toggle_row("Entity inspector", false),
                                        select_row(
                                            "Overlay",
                                            ["Top left", "Top right", "Bottom left", "Bottom right"],
                                        ),
                                        select_row(
                                            "Log level",
                                            ["Error", "Warn", "Info", "Debug", "Trace"],
                                        ),
                                    },
                                },
                                @PlumeSection {
                                    @header: bsn! { caption_small_caps("Cheats") },
                                    @contents: bsn_list! {
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Noclip") }
                                        },
                                        @PlumeCheckbox {
                                            @caption: bsn! { caption("Infinite health") }
                                        },
                                        slider_row("Move speed", 1.0, 40.0, 6.0, 0, Some("m/s".into())),
                                    },
                                },
                            ]
                        ),
                    ]
                ),
                separator(),
                // Footer: destructive-ish action on the left, confirm on the right.
                (
                    row()
                    Children [
                        (
                            @PlumeButton {
                                @caption: bsn_list! {
                                    fa_icon(font_awesome::solid::ARROW_ROTATE_LEFT),
                                    caption("Reset to defaults")
                                },
                                @variant: ButtonVariant::Outline,
                            }
                        ),
                        flex_spacer(),
                        (
                            @PlumeButton {
                                @caption: bsn! { caption("Cancel") },
                                @variant: ButtonVariant::Outline,
                            }
                        ),
                        (
                            @PlumeButton {
                                @caption: bsn! { caption("Apply") },
                                @variant: ButtonVariant::Primary,
                            }
                        ),
                    ]
                ),
            }
        }
    }
}

/// One half of the debug dialog: an equal-width column of sections.
fn debug_column() -> impl Scene {
    bsn! {
        column()
        // width: 0 + flex_grow so both columns split the dialog evenly regardless
        // of which one holds the wider content.
        Node {
            width: Val::ZERO,
            flex_grow: 1.0,
        }
    }
}

/// A labelled row whose control is a [`PlumeSelect`] over `options`, first one selected.
fn select_row(label: &str, options: impl IntoIterator<Item: AsRef<str>>) -> impl Scene {
    let options = options_from_strings(options, Some(0));
    bsn! {
        row()
        Children [
            (
                caption(label.to_string())
                Node { width: px(84) }
            ),
            (
                @PlumeSelect {
                    @options: {options},
                    @max_visible: 4,
                }
                Node { width: Val::ZERO, flex_grow: 1.0 }
            ),
        ]
    }
}

/// `Checked` as an optional patch, so one scene covers both states.
fn maybe_checked(checked: bool) -> impl Scene {
    checked.then(|| bsn! { Checked })
}

/// A labelled row whose control is a [`PlumeToggleSwitch`], pushed to the right edge.
fn toggle_row(label: &str, on: bool) -> impl Scene {
    bsn! {
        row()
        Children [
            caption(label.to_string()),
            flex_spacer(),
            (
                @PlumeToggleSwitch
                maybe_checked(on)
            ),
        ]
    }
}

/// A labelled row holding a slider and the number input mirroring it. Neither is bound to
/// the other here; this dialog is a look-and-feel sample, not a working panel.
fn slider_row(
    label: &str,
    min: f32,
    max: f32,
    value: f32,
    precision: usize,
    suffix: Option<String>,
) -> impl Scene {
    bsn! {
        row()
        Children [
            (
                caption(label.to_string())
                Node { width: px(84) }
            ),
            (
                @PlumeSlider {
                    @min: {min},
                    @max: {max},
                    @precision: {Some(precision as i32)},
                }
                Node { width: Val::ZERO, flex_grow: 1.0 }
                SliderValue({value})
            ),
            (
                @PlumeNumberInput {
                    @value: {value},
                    @precision: {precision},
                    @min: {min},
                    @max: {max},
                    @suffix: {suffix},
                }
            ),
        ]
    }
}
