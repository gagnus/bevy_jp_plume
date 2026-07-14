//! Throwaway smoke test: one of each control, spawned bare to audit defaults. Deleted in step 7.
use bevy::{
    prelude::*,
    ui::Checked,
    ui_widgets::{Activate, SliderValue, ValueChange},
};
use bevy_jp_plume::{
    PlumePlugins,
    constants::{font_awesome, icons},
    containers::{PlumeDialog, PlumeGroup, PlumeSubpane, flex_spacer, row},
    controls::{
        ButtonVariant, OptionIndex, PlumeButton, PlumeCheckbox, PlumeColorSwatch, PlumeRadio,
        PlumeRadioGroup, PlumeSelect, PlumeSlider, PlumeTextInput, PlumeToggleSwitch,
        list_rows_from_strings,
    },
    dark_theme::create_dark_theme,
    display::{caption, fa_icon_solid, icon, label_bright, label_dim},
    theme::{ThemeBackgroundColor, ThemeToken, ThemedText, UiTheme},
    tokens,
};
use bevy_ui::InteractionDisabled;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn());
    // SMOKE_SHOT=<path.png>: save a screenshot and exit (for headless verification).
    if std::env::var_os("SMOKE_SHOT").is_some() {
        app.add_systems(Update, screenshot_and_exit);
    }
    // SMOKE_FONT_AUDIT=1: log every text entity's resolved font and exit; entities on
    // the default handle are falling back to Bevy's embedded font.
    if std::env::var_os("SMOKE_FONT_AUDIT").is_some() {
        app.add_systems(Update, font_audit_and_exit);
    }
    app.run();
}

fn font_audit_and_exit(
    mut frames: Local<u32>,
    q_text: Query<(Entity, &TextFont, Option<&Text>, Option<&TextSpan>)>,
    assets: Res<AssetServer>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::text::FontSource;
    *frames += 1;
    if *frames != 60 {
        return;
    }
    for (entity, text_font, text, span) in q_text.iter() {
        let snippet: String = text
            .map(|t| t.0.as_str())
            .or(span.map(|s| s.0.as_str()))
            .unwrap_or("")
            .chars()
            .take(24)
            .collect();
        let font = match &text_font.font {
            FontSource::Handle(handle) => match assets.get_path(handle.id()) {
                Some(path) => format!("{path}"),
                None => "DEFAULT-FALLBACK".into(),
            },
            other => format!("{other:?}"),
        };
        info!("font_audit {entity} [{font}] {snippet:?}");
    }
    exit.write(AppExit::Success);
}

fn screenshot_and_exit(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    let shot_frame = std::env::var("SMOKE_SHOT_FRAME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40u32);
    *frames += 1;
    if *frames == shot_frame
        && let Some(path) = std::env::var_os("SMOKE_SHOT")
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::path::PathBuf::from(path)));
    }
    if *frames == shot_frame + 50 {
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
            // Start, not Stretch: several controls have flex_grow and would fill a
            // full-height column vertically.
            align_items: AlignItems::Start,
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
            label_bright("Plume smoke test"),
            label_dim("Bare controls, minimal wiring"),
            controls_row(None, false),
            controls_row(None, true),
            controls_row(Some(tokens::SUBPANE_BODY_BG), false),
            controls_row(Some(tokens::SUBPANE_BODY_BG), true),
            controls_row(Some(tokens::GROUP_BG), false),
            controls_row(Some(tokens::GROUP_BG), true),
            (
                @PlumeRadioGroup
                Children [
                    (
                        @PlumeRadio {
                            @caption: bsn! { caption("Zero") }
                        }
                    ),
                    (
                        @PlumeRadio {
                            @caption: bsn! { caption("One") }
                        }
                        Checked
                    )
                ]
            ),
            (
                @PlumeRadioGroup
                Children [
                    (
                        @PlumeRadio {
                            @caption: bsn! { caption("Two") }
                        }
                        InteractionDisabled
                    ),
                    (
                        @PlumeRadio {
                            @caption: bsn! { caption("Three") }
                        }
                        Checked
                        InteractionDisabled
                    )
                ]
            ),
            row() Children [
                label_bright("Swatch"),
                flex_spacer(),
                @PlumeColorSwatch,
                icon(icons::CHEVRON_DOWN),
            ],
            @PlumeSubpane {
                @header: bsn! { caption("Subpane") },
                @contents: bsn_list! {
                    label_dim("Subpane body"),
                    @PlumeGroup {
                        @contents: bsn_list! {
                            label_dim("Group content"),
                        },
                    },
                },
            },
        ]
    }
}

/// One of each control on a single row. `bg` tints the row (`None` keeps the window
/// background); `disabled` puts `InteractionDisabled` on every control.
fn controls_row(bg: Option<ThemeToken>, disabled: bool) -> impl Scene {
    let bg = bg.map(|token| bsn! { ThemeBackgroundColor({token}) });
    bsn! {
        row()
        {bg}
        Node { height: px(80.), width: px(1200.) }
        Children [
            (
                @PlumeButton {
                    @caption: bsn_list! { fa_icon_solid(font_awesome::FA_BUILDING_CIRCLE_ARROW_RIGHT), Node { width: px(10), }, caption("Button") }
                }
                on(|_: On<Activate>| info!("button clicked"))
                maybe_disabled(disabled)
            ),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Primary") },
                    @variant: ButtonVariant::Primary,
                }
                maybe_disabled(disabled)
            ),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Primary") },
                    @variant: ButtonVariant::Plain,
                }
                maybe_disabled(disabled)
            ),
            @PlumeCheckbox {
                @caption: bsn! { caption("Checkbox") }
            }
            maybe_disabled(disabled),
            @PlumeCheckbox {
                @caption: bsn! { caption("Checkbox") }
            }
            Checked
            maybe_disabled(disabled),
            @PlumeToggleSwitch maybe_disabled(disabled),
            @PlumeToggleSwitch Checked maybe_disabled(disabled),
            (
                // Group-less: checks itself when clicked, nothing unchecks it.
                @PlumeRadio
                maybe_disabled(disabled)
            ),
            (
                @PlumeRadio
                Checked
                maybe_disabled(disabled)
            ),
            (
                @PlumeSlider {
                    @max: 100.0,
                }
                SliderValue(20.0)
                on(|change: On<ValueChange<f32>>| info!("slider -> {}", change.value))
                maybe_disabled(disabled)
            ),
            (
                // 8 options / 4 visible: popup scrolls, scrollbar shown.
                @PlumeSelect {
                    @options: {list_rows_from_strings(
                        ["Alpha", "Beta", "Gamma", "Delta", "Echo", "Foxtrot", "Golf", "Hotel"],
                        Some(0),
                    )},
                    @max_visible: 4,
                }
                on(|change: On<ValueChange<Entity>>, q_options: Query<&OptionIndex>| {
                    if let Ok(option) = q_options.get(change.value) {
                        info!("select -> option {}", option.0);
                    }
                })
                maybe_disabled(disabled)
            ),
            // 3 options / 4 visible: fits, scrollbar hidden.
            @PlumeSelect {
                @options: {list_rows_from_strings(["Red", "Green", "Blue"], Some(0))},
                @max_visible: 4,
            }
            maybe_disabled(disabled),
            @PlumeTextInput
            maybe_disabled(disabled),
        ]
    }
}

/// `InteractionDisabled` as an optional patch, so the same row scene covers both variants.
fn maybe_disabled(disabled: bool) -> impl Scene {
    disabled.then(|| bsn! { InteractionDisabled })
}

fn dialog() -> impl Scene {
    bsn! {
        @PlumeDialog {
            @title: bsn! { caption("Dialog") },
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
