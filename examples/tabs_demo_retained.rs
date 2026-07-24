//! Retained (bsn!) twin of the `tabs_demo` example: tabs and bodies spawned once
//! as one scene and linked by name (`@target: #video`). Every body's widgets stay
//! alive while hidden, so their state survives switching away and back.
use bevy::{
    prelude::*,
    ui::{Checked, InteractionDisabled, Selected},
    ui_widgets::SliderValue,
};
use bevy_jp_plume::{
    PlumePlugins, TabGroup,
    constants::font_awesome,
    containers::{
        PlumeTab, PlumeTabs, column, flex_spacer, row, screen, separator, space, tab_body,
    },
    controls::{
        ButtonVariant, ColorSwatchValue, PlumeButton, PlumeCheckbox, PlumeColorSwatch,
        PlumeDisclosure, PlumeNumberInput, PlumeRadio, PlumeRadioGroup, PlumeSelect, PlumeSlider,
        PlumeTextInput, PlumeToggleSwitch, list_rows_from_strings,
    },
    display::{caption, fa_icon},
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
        // The screen surface establishes the standard font and text color, so the
        // bare caption below renders themed rather than in the engine default font.
        screen()
        Node { align_items: AlignItems::Start }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        // Tabs are tabbable, so they need a traversal scope; a dialog would bring
        // its own, but this screen-level container has to declare one.
        TabGroup::default()
        Children [
            caption("Retained tabs — bodies spawned once, shown by name."),
            (
                column()
                Node { width: px(420) }
                Children [
                    settings_tabs(),
                    separator(),
                    footer(),
                ]
            ),
        ]
    }
}

fn settings_tabs() -> impl Scene {
    bsn! {
        @PlumeTabs {
            @header: bsn_list! {
                (
                    @PlumeTab {
                        @caption: bsn_list! {
                            fa_icon(font_awesome::solid::GEAR),
                            caption("General")
                        },
                        @target: #general,
                    }
                    // The app's initial pick: the container reads it back into its
                    // selected index on the first frame, then owns it from there.
                    Selected
                ),
                @PlumeTab {
                    @caption: bsn! { caption("Video") },
                    @target: #video,
                },
                @PlumeTab {
                    @caption: bsn! { caption("Audio") },
                    @target: #audio,
                },
                (
                    @PlumeTab {
                        @caption: bsn! { caption("Advanced") },
                        @target: #advanced,
                    }
                    InteractionDisabled
                ),
            },
            @body: bsn_list! {
                (
                    #general
                    tab_body()
                    Children [
                        (
                            @PlumeCheckbox { @caption: bsn! { caption("Show tutorial hints") } }
                            Checked
                        ),
                        (
                            row()
                            Children [
                                field_label("Player name"),
                                // Type here, switch tabs, come back: the buffer is
                                // still there, because the entity never died.
                                (
                                    @PlumeTextInput { @value: "Player One" }
                                    Node { width: Val::ZERO, flex_grow: 1.0 }
                                ),
                            ]
                        ),
                        (
                            row()
                            Children [
                                field_label("Player color"),
                                (
                                    @PlumeSelect {
                                        @options: {list_rows_from_strings(
                                            ["Crimson", "Teal", "Amber"],
                                            Some(0),
                                        )},
                                    }
                                    Node { width: Val::ZERO, flex_grow: 1.0 }
                                ),
                                (
                                    @PlumeColorSwatch
                                    ColorSwatchValue({Color::srgb(0.82, 0.24, 0.29)})
                                ),
                            ]
                        ),
                    ]
                ),
                (
                    #video
                    tab_body()
                    Children [
                        caption("Window mode"),
                        (
                            @PlumeRadioGroup
                            Children [
                                (
                                    @PlumeRadio { @caption: bsn! { caption("Windowed") } }
                                    Checked
                                ),
                                @PlumeRadio { @caption: bsn! { caption("Borderless") } },
                                @PlumeRadio { @caption: bsn! { caption("Fullscreen") } },
                            ]
                        ),
                        // In a column the rule draws horizontally; in the footer row
                        // the same call draws vertically.
                        separator(),
                        slider_row("Gamma", 0.0, 2.0, 1.0),
                        (
                            row()
                            Children [
                                field_label("Quality"),
                                (
                                    @PlumeSelect {
                                        @options: {list_rows_from_strings(
                                            ["Low", "Medium", "High"],
                                            Some(1),
                                        )},
                                    }
                                    Node { width: Val::ZERO, flex_grow: 1.0 }
                                ),
                            ]
                        ),
                    ]
                ),
                (
                    #audio
                    tab_body()
                    Children [
                        (
                            row()
                            Children [
                                field_label("Muted"),
                                @PlumeToggleSwitch,
                            ]
                        ),
                        slider_row("Volume", 0.0, 1.0, 0.8),
                        (
                            row()
                            Children [
                                (@PlumeDisclosure Checked),
                                caption("Advanced"),
                            ]
                        ),
                        (
                            row()
                            Children [
                                field_label("Buffer"),
                                @PlumeNumberInput {
                                    @value: 256.0,
                                    @precision: 0,
                                    @min: 64.0,
                                    @max: 2048.0,
                                    @suffix: {Some("ms".to_string())},
                                }
                                Node { width: Val::ZERO, flex_grow: 1.0 },
                            ]
                        ),
                    ]
                ),
                (
                    #advanced
                    tab_body()
                    Children [
                        caption("Unreachable while the tab is disabled."),
                    ]
                ),
            },
        }
        Node {
            height: px(300),
        }

    }
}

/// The dialog-style action row under the tabs, one button per [`ButtonVariant`].
fn footer() -> impl Scene {
    bsn! {
        row()
        Children [
            (
                @PlumeButton {
                    @caption: bsn! { caption("Reset to defaults") },
                    @variant: ButtonVariant::Outline,
                }
            ),
            separator(),
            flex_spacer(),
            (@PlumeButton { @caption: bsn! { caption("Cancel") } }),
            space(px(12)),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Apply") },
                    @variant: ButtonVariant::Primary,
                }
            ),
        ]
    }
}

/// A fixed-width dim label sitting left of a control in a [`row`].
fn field_label(text: &str) -> impl Scene {
    let text = text.to_string();
    bsn! {
        caption(text)
        Node { width: px(84) }
    }
}

/// A labelled row whose control is a [`PlumeSlider`] holding `value`.
fn slider_row(label: &str, min: f32, max: f32, value: f32) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeSlider { @min: {min}, @max: {max} }
                SliderValue({value})
                Node { width: Val::ZERO, flex_grow: 1.0 }
            ),
        ]
    }
}
