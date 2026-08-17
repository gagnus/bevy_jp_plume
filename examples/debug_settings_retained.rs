//! Retained (bsn!) twin of the `debug_settings` example: the same panel over the
//! same `DebugSettings` resource, bound with `on()` observers instead of imm's `&mut`.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use bevy_jp_plume::retained::{
    Activate, Checked, InheritableFont, PlumeButton, PlumeCheckbox, PlumeDialog, PlumeFontSize,
    PlumeNumberInput, PlumePopup, PlumeSection, PlumeSelect, PlumeSlider, PlumeTextInput,
    PlumeToggleSwitch, PlumeToolButton, PopupDismiss, PopupPlacement, PopupSocket, SetValue,
    SliderValue, ThemeBackgroundSlot, ThemeTextSlot, Tooltip, TooltipContent, ValueChange, caption,
    close_popup, column, flex_spacer, icon, popup_socket, row, separator, small_caps,
};

#[path = "common/mod.rs"]
mod common;

use common::Options;
use common::debug_settings::{BASE_FONT_PX, DebugSettings, LogLevel, OverlayCorner, ViewMode};

fn main() {
    let mut app = common::demo_app(false);
    app.init_resource::<DebugSettings>()
        .add_systems(Startup, scene.spawn())
        .add_systems(
            Update,
            (
                common::log_on_change::<DebugSettings>,
                push_settings,
                push_ui_scale,
                show_capture_warning,
            ),
        );
    app.run();
}

// The [`DebugSettings`] field a control is bound to.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
enum Bound {
    Wireframe,
    ShowColliders,
    FreezeCulling,
    ViewMode,
    Gamma,
    PauseSim,
    TimeScale,
    FpsOverlay,
    EntityInspector,
    Overlay,
    LogLevel,
    CaptureDir,
    Noclip,
    InfiniteHealth,
    MoveSpeed,
    UiScale,
}

impl Bound {
    fn flag(self, s: &mut DebugSettings) -> Option<&mut bool> {
        Some(match self {
            Bound::Wireframe => &mut s.wireframe,
            Bound::ShowColliders => &mut s.show_colliders,
            Bound::FreezeCulling => &mut s.freeze_culling,
            Bound::PauseSim => &mut s.pause_sim,
            Bound::FpsOverlay => &mut s.fps_overlay,
            Bound::EntityInspector => &mut s.entity_inspector,
            Bound::Noclip => &mut s.noclip,
            Bound::InfiniteHealth => &mut s.infinite_health,
            _ => return None,
        })
    }

    fn number(self, s: &mut DebugSettings) -> Option<&mut f32> {
        Some(match self {
            Bound::Gamma => &mut s.gamma,
            Bound::TimeScale => &mut s.time_scale,
            Bound::MoveSpeed => &mut s.move_speed,
            Bound::UiScale => &mut s.ui_scale,
            _ => return None,
        })
    }

    fn text(self, s: &mut DebugSettings) -> Option<&mut String> {
        match self {
            Bound::CaptureDir => Some(&mut s.capture_dir),
            _ => None,
        }
    }

    fn choice(self, s: &DebugSettings) -> Option<usize> {
        Some(match self {
            Bound::ViewMode => s.view_mode.index(),
            Bound::Overlay => s.overlay.index(),
            Bound::LogLevel => s.log_level.index(),
            _ => return None,
        })
    }

    fn set_choice(self, s: &mut DebugSettings, index: usize) {
        match self {
            Bound::ViewMode => s.view_mode = ViewMode::from_index(index),
            Bound::Overlay => s.overlay = OverlayCorner::from_index(index),
            Bound::LogLevel => s.log_level = LogLevel::from_index(index),
            _ => {}
        }
    }
}

// Push the resource back into the controls, so Reset moves what you can see.
fn push_settings(
    mut settings: ResMut<DebugSettings>,
    q_bound: Query<(Entity, &Bound)>,
    mut commands: Commands,
) {
    if !settings.is_changed() {
        return;
    }
    // Reading through the change-detection bypass keeps this push from re-triggering itself.
    let settings = settings.bypass_change_detection();

    for (entity, &bound) in q_bound.iter() {
        if let Some(&mut value) = bound.flag(settings) {
            commands.trigger(SetValue { entity, value });
        }
        if let Some(&mut value) = bound.number(settings) {
            commands.trigger(SetValue { entity, value });
        }
        if let Some(value) = bound.text(settings) {
            let value = value.clone();
            commands.trigger(SetValue { entity, value });
        }
        if let Some(value) = bound.choice(settings) {
            commands.trigger(SetValue { entity, value });
        }
    }
}

// The imm twin's `.font_size()` on the dialog, as a retained cascade root.
fn push_ui_scale(
    settings: Res<DebugSettings>,
    q_dialog: Query<Entity, With<PlumeDialog>>,
    mut commands: Commands,
) {
    if !settings.is_changed() {
        return;
    }
    for entity in q_dialog.iter() {
        commands.entity(entity).insert(InheritableFont {
            font_size: Some(PlumeFontSize::Px(BASE_FONT_PX * settings.ui_scale)),
            ..default()
        });
    }
}

fn on_flag(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<bool>>, mut s: ResMut<DebugSettings>| {
            if let Some(field) = bound.flag(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_number(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<f32>>, mut s: ResMut<DebugSettings>| {
            if let Some(field) = bound.number(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_text(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<String>>, mut s: ResMut<DebugSettings>| {
            if let Some(field) = bound.text(&mut s) {
                field.clone_from(&ev.value);
            }
        })
    }
}

fn on_choice(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<usize>>, mut s: ResMut<DebugSettings>| {
            bound.set_choice(&mut s, ev.value);
        })
    }
}

fn scene() -> impl SceneList {
    bsn_list![root()]
}

fn root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
        }
        template_value(ThemeBackgroundSlot(ThemeSlot::Neutral0))
        Children [
            debug_options_dialog(),
        ]
    }
}

fn debug_options_dialog() -> impl Scene {
    let s = DebugSettings::default();
    bsn! {
        @PlumeDialog {
            @title: bsn! { caption("Debug Options") InheritableFont { font_size: size::DIALOG_HEADER_TEXT_SIZE } },
            @width: em(600.0 / BASE_FONT_PX),
            @inset: {Corner::BottomLeft.inset(px(20), px(20))},
            @contents: bsn_list![
                (
                    row()
                    Node { align_items: AlignItems::Start }
                    Children [
                        (
                            debug_column()
                            Children [
                                rendering_section(&s),
                                physics_section(&s),
                                interface_section(&s),
                            ]
                        ),
                        (
                            debug_column()
                            Children [
                                diagnostics_section(&s),
                                cheats_section(&s),
                            ]
                        ),
                    ]
                ),
                separator(),
                footer(),
            ],
        }
    }
}

fn rendering_section(s: &DebugSettings) -> impl Scene {
    let (view_mode, gamma) = (s.view_mode.index(), s.gamma);
    let (wireframe, show_colliders, freeze) = (s.wireframe, s.show_colliders, s.freeze_culling);
    bsn! {
        @PlumeSection {
            @header: bsn! { caption("Rendering") small_caps() },
            @contents: bsn_list![
                (
                    checkbox("Wireframe", Bound::Wireframe, wireframe)
                    Tooltip("Draw all meshes as wireframe")
                ),
                checkbox("Show colliders", Bound::ShowColliders, show_colliders),
                checkbox("Freeze frustum culling", Bound::FreezeCulling, freeze),
                (
                    select_row("View mode", Bound::ViewMode, ViewMode::select_options(), view_mode, 4)
                    Tooltip("Which render pass fills the viewport")
                ),
                (
                    slider_row("Gamma", Bound::Gamma, gamma, 0.5, 3.0, 2, None)
                    Tooltip("Display gamma correction")
                ),
            ],
        }
    }
}

fn physics_section(s: &DebugSettings) -> impl Scene {
    let (pause_sim, time_scale) = (s.pause_sim, s.time_scale);
    bsn! {
        @PlumeSection {
            @header: bsn! { caption("Physics") small_caps() },
            @contents: bsn_list![
                (
                    checkbox("Pause simulation", Bound::PauseSim, pause_sim)
                    Tooltip("Halt the physics clock; rendering keeps running")
                ),
                slider_row("Time scale", Bound::TimeScale, time_scale, 0.5, 2.0, 2, None),
            ],
        }
    }
}

fn interface_section(s: &DebugSettings) -> impl Scene {
    let ui_scale = s.ui_scale;
    bsn! {
        @PlumeSection {
            @header: bsn! { caption("Interface") small_caps() },
            @contents: bsn_list![
                (
                    slider_row("UI scale", Bound::UiScale, ui_scale, 0.5, 2.0, 2, None)
                    Tooltip("Scales this dialog's text and everything sized from it")
                ),
            ],
        }
    }
}

fn diagnostics_section(s: &DebugSettings) -> impl Scene {
    let (fps_overlay, entity_inspector) = (s.fps_overlay, s.entity_inspector);
    let (overlay, log_level) = (s.overlay.index(), s.log_level.index());
    bsn! {
        @PlumeSection {
            @header: bsn! { caption("Diagnostics") small_caps() },
            @contents: bsn_list![
                toggle_row("FPS overlay", Bound::FpsOverlay, fps_overlay),
                toggle_row("Entity inspector", Bound::EntityInspector, entity_inspector),
                select_row("Overlay", Bound::Overlay, OverlayCorner::select_options(), overlay, 4),
                select_row("Log level", Bound::LogLevel, LogLevel::select_options(), log_level, 3),
                capture_row(s),
            ],
        }
    }
}

// A path-style field: clear tucked inside the leading edge, a warning inside
// the trailing one — the props twins of `prefix_container` / `suffix_container`.
fn capture_row(s: &DebugSettings) -> impl Scene {
    let value = s.capture_dir.clone();
    let warning_display = capture_warning_display(s);
    bsn! {
        row()
        Children [
            field_label("Capture to"),
            (
                @PlumeTextInput {
                    @value: value,
                    @placeholder: {Some("Beside the app".to_string())},
                    @prefix_container: {Some(clear_capture_button())},
                    @suffix_container: {Some(capture_warning(warning_display))},
                }
                Node { width: Val::ZERO, flex_grow: 1.0 }
                on_text(Bound::CaptureDir)
            ),
        ]
    }
}

fn clear_capture_button() -> Box<dyn SceneList> {
    Box::new(bsn_list![(
        @PlumeToolButton {
            @caption: bsn_list![icon(font_awesome::solid::XMARK)],
            @variant: ButtonVariant::Plain,
        }
        Tooltip("Clear")
        InheritableFont { font_size: {PlumeFontSize::Em(0.8)} }
        on(|_: On<Activate>, mut s: ResMut<DebugSettings>| {
            s.capture_dir.clear();
        })
    )])
}

// On the warning's row, so `show_capture_warning` can find it.
#[derive(Component, Default, Clone)]
struct CaptureWarning;

fn capture_warning_display(s: &DebugSettings) -> Display {
    match s.capture_dir.is_empty() {
        true => Display::None,
        false => Display::Flex,
    }
}

// The imm twin builds the warning only while the path is set; retained builds
// it once and `show_capture_warning` shows and hides it.
fn capture_warning(display: Display) -> Box<dyn SceneList> {
    Box::new(bsn_list![(
        row()
        Node { display: display, padding: UiRect::right(size::SPACE_TIGHT) }
        CaptureWarning
        Children [
            (
                icon(font_awesome::solid::TRIANGLE_EXCLAMATION)
                template_value(ThemeTextSlot(ThemeSlot::Danger0))
                Tooltip("Should leave it blank!")
            ),
        ]
    )])
}

fn show_capture_warning(
    settings: Res<DebugSettings>,
    mut q_warnings: Query<&mut Node, With<CaptureWarning>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut node in q_warnings.iter_mut() {
        let display = capture_warning_display(&settings);
        if node.display != display {
            node.display = display;
        }
    }
}

fn cheats_section(s: &DebugSettings) -> impl Scene {
    let (noclip, infinite_health, move_speed) = (s.noclip, s.infinite_health, s.move_speed);
    bsn! {
        @PlumeSection {
            @header: bsn! { caption("Cheats") small_caps() },
            @contents: bsn_list![
                checkbox("Noclip", Bound::Noclip, noclip),
                checkbox("Infinite health", Bound::InfiniteHealth, infinite_health),
                slider_row("Move speed", Bound::MoveSpeed, move_speed, 1.0, 40.0, 0, Some("m/s".into())),
            ],
        }
    }
}

#[derive(Component, Default, Clone)]
struct ResetConfirm;

fn close_reset_confirm(q_open: &Query<Entity, With<ResetConfirm>>, commands: &mut Commands) {
    if let Ok(popup) = q_open.single() {
        close_popup(commands, popup);
    }
}

fn open_reset_confirm(
    ev: On<Activate>,
    q_childof: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_socket: Query<(), With<PopupSocket>>,
    q_open: Query<Entity, With<ResetConfirm>>,
    mut commands: Commands,
) {
    if q_open.single().is_ok() {
        close_reset_confirm(&q_open, &mut commands);
        return;
    }
    let Ok(parent) = q_childof.get(ev.event_target()) else {
        return;
    };
    let Some(socket) = q_children
        .get(parent.parent())
        .ok()
        .and_then(|children| children.iter().find(|&c| q_socket.contains(c)))
    else {
        return;
    };
    commands
        .spawn_scene(reset_confirm_popup())
        .insert(ChildOf(socket));
}

fn reset_confirm_popup() -> impl Scene {
    bsn! {
        @PlumePopup {
            @placement: PopupPlacement::Below,
            @dismiss: PopupDismiss::OutsideClick,
            @contents: bsn_list![
                (
                    caption("Reset all settings to defaults?")
                    TextLayout { linebreak: LineBreak::NoWrap }
                    Node { min_width: Val::ZERO }
                ),
                (
                    row()
                    Children [
                        flex_spacer(),
                        (
                            @PlumeButton {
                                @caption: bsn! { caption("Reset") },
                                @variant: ButtonVariant::Primary,
                            }
                            on(|_: On<Activate>,
                                mut s: ResMut<DebugSettings>,
                                q_open: Query<Entity, With<ResetConfirm>>,
                                mut commands: Commands| {
                                *s = DebugSettings::default();
                                close_reset_confirm(&q_open, &mut commands);
                            })
                        ),
                        (
                            @PlumeButton {
                                @caption: bsn! { caption("Keep") },
                                @variant: ButtonVariant::Outline,
                            }
                            on(|_: On<Activate>,
                                q_open: Query<Entity, With<ResetConfirm>>,
                                mut commands: Commands| {
                                close_reset_confirm(&q_open, &mut commands);
                            })
                        ),
                    ]
                ),
            ],
        }
        ResetConfirm
    }
}

fn footer() -> impl Scene {
    bsn! {
        row()
        Children [
            (
                row()
                Children [
                    (
                        @PlumeButton {
                            @caption: bsn_list![
                                icon(font_awesome::solid::ARROW_ROTATE_LEFT),
                                caption("Reset to defaults"),
                            ],
                            @variant: ButtonVariant::Outline,
                        }
                        template_value(TooltipContent::new(|| bsn_list![
                            (
                                row()
                                Children [
                                    icon(font_awesome::solid::ARROW_ROTATE_LEFT),
                                    caption("Reset to defaults")
                                    template_value(ThemeTextSlot(ThemeSlot::Text0)),
                                ]
                            ),
                            caption("Every debug option returns to its default value"),
                        ]))
                        on(open_reset_confirm)
                    ),
                    popup_socket(),
                ]
            ),
            flex_spacer(),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Cancel") },
                    @variant: ButtonVariant::Outline,
                }
                Tooltip("Discard changes and close")
            ),
            (
                @PlumeButton {
                    @caption: bsn! { caption("Apply") },
                    @variant: ButtonVariant::Primary,
                }
                Tooltip("Apply changes and close")
            ),
        ]
    }
}

fn debug_column() -> impl Scene {
    bsn! {
        column()
        Node {
            width: Val::ZERO,
            flex_grow: 1.0,
        }
    }
}

fn maybe_checked(checked: bool) -> impl Scene {
    checked.then(|| bsn! { Checked })
}

fn checkbox(label: &str, bound: Bound, checked: bool) -> impl Scene {
    let label = label.to_string();
    bsn! {
        @PlumeCheckbox { @caption: bsn! { caption(label) } }
        maybe_checked(checked)
        on_flag(bound)
    }
}

fn field_label(text: &str) -> impl Scene {
    let text = text.to_string();
    bsn! {
        caption(text)
        Node { width: em(84.0 / BASE_FONT_PX) }
    }
}

fn toggle_row(label: &str, bound: Bound, checked: bool) -> impl Scene {
    bsn! {
        row()
        Children [
            caption(label.to_string()),
            flex_spacer(),
            (
                @PlumeToggleSwitch
                maybe_checked(checked)
                on_flag(bound)
            ),
        ]
    }
}

fn select_row(
    label: &str,
    bound: Bound,
    options: Vec<(String, bool)>,
    selected: usize,
    max_visible: usize,
) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeSelect {
                    @options: options,
                    @selected: selected,
                    @max_visible: max_visible,
                }
                Node { width: Val::ZERO, flex_grow: 1.0 }
                on_choice(bound)
            ),
        ]
    }
}

fn slider_row(
    label: &str,
    bound: Bound,
    value: f32,
    min: f32,
    max: f32,
    precision: usize,
    suffix: Option<String>,
) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeSlider {
                    @min: min,
                    @max: max,
                    @precision: {Some(precision as i32)},
                }
                SliderValue(value)
                Node { width: Val::ZERO, flex_grow: 1.0 }
                on_number(bound)
            ),
            (
                @PlumeNumberInput {
                    @value: value,
                    @precision: precision,
                    @min: min,
                    @max: max,
                    @suffix: suffix,
                }
                on_number(bound)
            ),
        ]
    }
}
