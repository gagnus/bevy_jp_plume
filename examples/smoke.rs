//! Throwaway smoke test: one of each control, spawned bare to audit defaults. Deleted in step 7.
use bevy::{
    prelude::*,
    ui::Checked,
    ui_widgets::{Activate, ControlOrientation, ScrollArea, SliderValue, ValueChange},
};
use bevy_jp_plume::{
    PlumePlugins,
    constants::{font_awesome, icons, size},
    containers::{PlumeDialog, PlumeGroup, PlumeSubpane, flex_spacer, row},
    controls::{
        ButtonVariant, OptionIndex, PlumeButton, PlumeCheckbox, PlumeColorSwatch, PlumeRadio,
        PlumeRadioGroup, PlumeScrollbar, PlumeSelect, PlumeSlider, PlumeTextInput,
        PlumeToggleSwitch, ScrollbarGutter, list_rows_from_strings,
    },
    dark_theme::default_dark_palette,
    display::{caption, fa_icon_solid, icon, label_bright, label_dim},
    theme::{
        EditablePalette, ThemeBackgroundColor, ThemeToken, UiTheme, build_theme,
        default_token_slots,
    },
    tokens,
};
use bevy_ui::InteractionDisabled;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<PaletteEditor>()
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, rebuild_theme_on_edit);
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
                    @caption: bsn_list! { fa_icon_solid(font_awesome::FA_WAND_MAGIC_SPARKLES), Node { width: px(10), }, caption("Button") }
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

/// Holds the parametric palette the dialog edits. Mutated by the slider observers;
/// [`rebuild_theme_on_edit`] bakes it into the live [`UiTheme`] whenever it changes.
#[derive(Resource)]
struct PaletteEditor(EditablePalette);

impl Default for PaletteEditor {
    fn default() -> Self {
        Self(default_dark_palette())
    }
}

/// The three parametric ramps ([`OklchaArray`]s) an [`EditablePalette`] exposes.
#[derive(Clone, Copy)]
enum Ramp {
    Neutral,
    Accent,
    Text,
}

/// One editable scalar of the [`EditablePalette`]. Each slider owns a variant and
/// reads/writes exactly that field. `L(_, i)` is the i-th lightness stop of a ramp.
#[derive(Clone, Copy)]
enum PaletteParam {
    Hue(Ramp),
    Chroma(Ramp),
    L(Ramp, usize),
    DisabledTextAlpha,
}

impl Ramp {
    fn array<'a>(self, p: &'a EditablePalette) -> RampView<'a> {
        match self {
            Ramp::Neutral => RampView {
                hue: &p.neutrals.hue,
                chroma: &p.neutrals.chroma,
                l: &p.neutrals.l,
            },
            Ramp::Accent => RampView {
                hue: &p.accent.hue,
                chroma: &p.accent.chroma,
                l: &p.accent.l,
            },
            Ramp::Text => RampView {
                hue: &p.text.hue,
                chroma: &p.text.chroma,
                l: &p.text.l,
            },
        }
    }

    /// Max chroma for this ramp's chroma slider (neutrals/text stay near-grey).
    fn chroma_max(self) -> f32 {
        match self {
            Ramp::Accent => 0.3,
            Ramp::Neutral | Ramp::Text => 0.05,
        }
    }

    /// Number of lightness stops this ramp carries.
    fn stops(self) -> usize {
        match self {
            Ramp::Neutral => 7,
            Ramp::Accent => 4,
            Ramp::Text => 2,
        }
    }
}

/// Borrowed view over a ramp's scalars, so one code path reads any ramp.
struct RampView<'a> {
    hue: &'a f32,
    chroma: &'a f32,
    l: &'a [f32],
}

impl PaletteParam {
    fn label(self) -> String {
        match self {
            PaletteParam::Hue(_) => "Hue".into(),
            PaletteParam::Chroma(_) => "Chroma".into(),
            PaletteParam::L(_, i) => format!("L {i}"),
            PaletteParam::DisabledTextAlpha => "Disabled \u{3b1}".into(),
        }
    }

    /// Slider `(min, max)` for this param.
    fn range(self) -> (f32, f32) {
        match self {
            PaletteParam::Hue(_) => (0.0, 360.0),
            PaletteParam::Chroma(ramp) => (0.0, ramp.chroma_max()),
            PaletteParam::L(..) | PaletteParam::DisabledTextAlpha => (0.0, 1.0),
        }
    }

    fn get(self, p: &EditablePalette) -> f32 {
        match self {
            PaletteParam::Hue(ramp) => *ramp.array(p).hue,
            PaletteParam::Chroma(ramp) => *ramp.array(p).chroma,
            PaletteParam::L(ramp, i) => ramp.array(p).l[i],
            PaletteParam::DisabledTextAlpha => p.disabled_text_alpha_modifier,
        }
    }

    fn set(self, p: &mut EditablePalette, v: f32) {
        match self {
            PaletteParam::Hue(Ramp::Neutral) => p.neutrals.hue = v,
            PaletteParam::Hue(Ramp::Accent) => p.accent.hue = v,
            PaletteParam::Hue(Ramp::Text) => p.text.hue = v,
            PaletteParam::Chroma(Ramp::Neutral) => p.neutrals.chroma = v,
            PaletteParam::Chroma(Ramp::Accent) => p.accent.chroma = v,
            PaletteParam::Chroma(Ramp::Text) => p.text.chroma = v,
            PaletteParam::L(Ramp::Neutral, i) => p.neutrals.l[i] = v,
            PaletteParam::L(Ramp::Accent, i) => p.accent.l[i] = v,
            PaletteParam::L(Ramp::Text, i) => p.text.l[i] = v,
            PaletteParam::DisabledTextAlpha => p.disabled_text_alpha_modifier = v,
        }
    }
}

/// Every param controlling one ramp: hue, chroma, then each lightness stop.
fn ramp_params(ramp: Ramp) -> Vec<PaletteParam> {
    let mut v = vec![PaletteParam::Hue(ramp), PaletteParam::Chroma(ramp)];
    v.extend((0..ramp.stops()).map(|i| PaletteParam::L(ramp, i)));
    v
}

/// Bake the edited palette into the live theme whenever it changes (fires once at
/// startup too, harmlessly re-deriving the dark theme already installed).
fn rebuild_theme_on_edit(editor: Res<PaletteEditor>, mut theme: ResMut<UiTheme>) {
    if editor.is_changed() {
        *theme = UiTheme(build_theme(&editor.0.resolve(), default_token_slots()));
    }
}

fn dialog() -> impl Scene {
    let palette = default_dark_palette();
    let mut text_params = ramp_params(Ramp::Text);
    text_params.push(PaletteParam::DisabledTextAlpha);
    bsn! {
        @PlumeDialog {
            @title: bsn! { caption("Theme editor") },
            @width: px(320),
            @left: px(320),
            @top: px(40),
            @contents: bsn_list! {
                // Bounded frame holding the scrollbar; the inner node scrolls.
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        height: px(560),
                    }
                    ScrollbarGutter(size::SCROLLBAR_GUTTER)
                    Children [
                        (
                            #inner
                            Node {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                overflow: Overflow::scroll_y(),
                            }
                            ScrollArea
                            Children [
                                param_group("Neutrals", ramp_params(Ramp::Neutral), &palette),
                                param_group("Accent", ramp_params(Ramp::Accent), &palette),
                                param_group("Text", text_params, &palette),
                            ]
                        ),
                        (
                            @PlumeScrollbar {
                                @target: #inner,
                                @orientation: {ControlOrientation::Vertical},
                            }
                            Node {
                                position_type: PositionType::Absolute,
                                right: px(0),
                                top: px(0),
                                bottom: px(0),
                                width: size::SCROLLBAR_WIDTH,
                            }
                        ),
                    ]
                ),
            }
        }
    }
}

/// A titled [`PlumeSubPane`] holding a labelled slider per `param`.
fn param_group(title: &str, params: Vec<PaletteParam>, palette: &EditablePalette) -> impl Scene {
    let mut rows: Vec<Box<dyn SceneList>> = vec![];
    rows.extend(params.into_iter().map(|param| param_row(param, palette)));
    let contents: Box<dyn SceneList> = Box::new(rows);
    bsn! {
        @PlumeSubpane {
            @header: bsn! { caption(title.to_string()) }
            @contents: {contents},
        }
    }
}

/// A labelled slider bound to one [`PaletteParam`]. Its observer writes the new value
/// straight into [`PaletteEditor`]; the change-detection system rebuilds the theme.
fn param_row(param: PaletteParam, palette: &EditablePalette) -> Box<dyn SceneList> {
    let (min, max) = param.range();
    let value = param.get(palette);
    bsn! {
        row()
        Children [
            (label_dim(param.label()) Node { width: px(80) }),
            (
                @PlumeSlider {
                    @min: {min},
                    @max: {max},
                }
                SliderValue({value})
                on(move |change: On<ValueChange<f32>>, mut editor: ResMut<PaletteEditor>| {
                    param.set(&mut editor.0, change.value);
                })
            ),
        ]
    }
    .into()
}
