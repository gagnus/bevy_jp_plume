//! Throwaway smoke test: one of each control, spawned bare to audit defaults. Deleted in step 7.
use bevy::{
    prelude::*,
    ui::Checked,
    ui_widgets::{
        Activate, RadioGroup, SliderValue, ValueChange, checkbox_self_update,
        listbox_update_selection, radio_self_update, slider_self_update,
    },
};
use bevy_jp_plume::{
    PlumePlugins,
    constants::icons,
    containers::{flex_spacer, group, group_body, group_header},
    controls::{
        PlumeButton, PlumeCheckbox, PlumeColorSwatch, PlumeDialog, PlumeRadio, PlumeSelect,
        PlumeSlider, PlumeTextInput, PlumeTextInputContainer, PlumeToggleSwitch,
        list_rows_from_strings,
    },
    dark_theme::create_dark_theme,
    display::{caption, icon, label, label_dim},
    theme::{ThemeBackgroundColor, ThemedText, UiTheme},
    tokens,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn());
    // SMOKE_SHOT=<path.png>: save a screenshot and exit (for headless verification).
    if std::env::var_os("SMOKE_SHOT").is_some() {
        app.add_systems(Update, screenshot_and_exit);
    }
    app.run();
}

fn screenshot_and_exit(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    *frames += 1;
    if *frames == 40
        && let Some(path) = std::env::var_os("SMOKE_SHOT")
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::path::PathBuf::from(path)));
    }
    if *frames == 90 {
        exit.write(AppExit::Success);
    }
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, root()]
}

fn root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            controls_column(),
            dialog(),
        ]
    }
}

fn controls_column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            padding: px(8),
            row_gap: px(8),
            width: px(260),
        }
        Children [
            label("Plume smoke test"),
            label_dim("Bare controls, minimal wiring"),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Button") }
                }
                on(|_: On<Activate>| info!("button clicked"))
            ),
            (
                @PlumeCheckbox {
                    @caption: bsn! { caption("Checkbox") }
                }
                on(checkbox_self_update)
            ),
            (@PlumeToggleSwitch on(checkbox_self_update)),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                }
                RadioGroup
                on(radio_self_update)
                Children [
                    (
                        @PlumeRadio {
                            @caption: bsn! { caption("One") }
                        }
                        Checked
                    ),
                    @PlumeRadio {
                        @caption: bsn! { caption("Two") }
                    },
                    @PlumeRadio {
                        @caption: bsn! { caption("Three") }
                    },
                ]
            ),
            (
                @PlumeSlider {
                    @max: 100.0,
                }
                SliderValue(20.0)
                on(slider_self_update)
                on(|change: On<ValueChange<f32>>| info!("slider -> {}", change.value))
            ),
            (
                @PlumeSelect {
                    @options: {list_rows_from_strings(["Alpha", "Beta", "Gamma", "Delta"], Some(0))},
                    @max_visible: 4,
                }
                on(listbox_update_selection)
            ),
            (
                @PlumeTextInputContainer
                Children [
                    @PlumeTextInput {
                        @visible_width: {Some(12f32)},
                    }
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                }
                Children [
                    label("Swatch"),
                    flex_spacer(),
                    @PlumeColorSwatch,
                    icon(icons::CHEVRON_DOWN),
                ]
            ),
            group() Children [
                group_header() Children [
                    caption("Group"),
                ],
                group_body() Children [
                    label_dim("Group body"),
                ],
            ],
        ]
    }
}

fn dialog() -> impl Scene {
    bsn! {
        @PlumeDialog {
            @title: {"Dialog".to_string()},
            @width: px(280),
            @left: px(320),
            @top: px(40),
            @contents: bsn_list! {
                (Text("Drag the title bar; close despawns.") ThemedText),
                (
                    @PlumeButton {
                        @caption: bsn! { caption("In-dialog button") }
                    }
                    on(|_: On<Activate>| info!("dialog button clicked"))
                ),
            }
        }
    }
}
