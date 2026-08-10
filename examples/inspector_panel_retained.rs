//! Retained (bsn!) twin of the `inspector_panel` example: the same docked panel over
//! the same `Inspector` resource, bound with `on()` observers instead of imm's `&mut`.
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use bevy_jp_plume::retained::{
    Activate, Checkable, Checked, ColorSwatchValue, Flat, InheritableFont, InteractionDisabled,
    PlumeColorEdit, PlumeColorPicker, PlumeColorSwatch, PlumeDialog, PlumeDisclosure,
    PlumeFontSize, PlumeMenuBar, PlumeMenuButton, PlumeRadio, PlumeRadioGroup, PlumeScreen,
    PlumeScrollArea, PlumeSection, PlumeSlider, PlumeSplitter, PlumeTab, PlumeTabs, PlumeTextInput,
    PlumeToggleSwitch, PlumeToolButton, SectionCollapsed, Selected, SetValue, SliderValue, TabSlot,
    ThemeBackgroundSlot, Tooltip, ValueChange, caption, caption_small_caps, column, fa_icon,
    flex_spacer, row, screen, separator, space, tab_body,
};

#[path = "common/mod.rs"]
mod common;

use common::Options;
use common::inspector_panel::{
    BASE_FONT_PX, Blend, Cull, Inspector, MAX_DOCUMENTS, Material, SceneNode, Tab,
};

const GUTTER: Val = Val::Px(78.0);

fn main() {
    let mut app = common::demo_app(false);
    app.insert_resource(Inspector::initial())
        .add_systems(Startup, scene.spawn())
        .add_systems(
            Update,
            (
                common::log_on_change::<Inspector>,
                push_material,
                push_ui_scale,
                push_tree_rows,
                push_documents,
                push_hud_visible,
            ),
        );
    app.run();
}

// The [`Inspector`] field a control is bound to.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
enum Bound {
    Name,
    BaseColor,
    Emissive,
    Blend,
    Cull,
    Metallic,
    Roughness,
    UiScale,
    Filter,
    Picker,
}

impl Bound {
    fn number(self, s: &mut Inspector) -> Option<&mut f32> {
        Some(match self {
            Bound::Metallic => &mut s.material.metallic,
            Bound::Roughness => &mut s.material.roughness,
            Bound::UiScale => &mut s.ui_scale,
            _ => return None,
        })
    }

    fn text(self, s: &mut Inspector) -> Option<&mut String> {
        Some(match self {
            Bound::Name => &mut s.material.name,
            Bound::Filter => &mut s.hierarchy.filter,
            _ => return None,
        })
    }

    fn color(self, s: &mut Inspector) -> Option<&mut Color> {
        Some(match self {
            Bound::BaseColor | Bound::Picker => &mut s.material.base_color,
            Bound::Emissive => &mut s.material.emissive,
            _ => return None,
        })
    }

    fn choice(self, s: &Inspector) -> Option<usize> {
        Some(match self {
            Bound::Blend => s.material.blend.index(),
            Bound::Cull => s.material.cull.index(),
            _ => return None,
        })
    }

    fn set_choice(self, s: &mut Inspector, index: usize) {
        match self {
            Bound::Blend => s.material.blend = Blend::from_index(index),
            Bound::Cull => s.material.cull = Cull::from_index(index),
            _ => {}
        }
    }
}

// Marks a tree row with the node it draws, so the filter/fold system can hide it.
#[derive(Component, Clone, Copy, Default)]
struct TreeRow(usize);

// Marks the per-node controls, which bind by index rather than by field.
#[derive(Component, Clone, Copy)]
enum NodeBound {
    Expanded(usize),
    Visible(usize),
}

fn on_number(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<f32>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.number(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_text(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<String>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.text(&mut s) {
                field.clone_from(&ev.value);
            }
        })
    }
}

fn on_color(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<Color>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.color(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_choice(bound: Bound) -> impl Scene {
    bsn! {
        template_value(bound)
        on(move |ev: On<ValueChange<usize>>, mut s: ResMut<Inspector>| {
            bound.set_choice(&mut s, ev.value);
        })
    }
}

// Push the resource back into the controls, so Revert moves what you can see.
fn push_material(
    mut state: ResMut<Inspector>,
    q_bound: Query<(Entity, &Bound)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    let s = state.bypass_change_detection();

    for (entity, &bound) in q_bound.iter() {
        if let Some(&mut value) = bound.number(s) {
            commands.trigger(SetValue { entity, value });
        }
        if let Some(&mut value) = bound.color(s) {
            commands.trigger(SetValue { entity, value });
        }
        if let Some(value) = bound.choice(s) {
            commands.trigger(SetValue { entity, value });
        }
        if let Some(value) = bound.text(s) {
            let value = value.clone();
            commands.trigger(SetValue { entity, value });
        }
    }
}

// The imm twin's `.font_size()` on the panel, as a retained cascade root.
fn push_ui_scale(
    state: Res<Inspector>,
    q_panel: Query<
        (Entity, Option<&InheritableFont>),
        Or<(With<PlumeScreen>, With<InspectorPanel>)>,
    >,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    let wanted = PlumeFontSize::Px(BASE_FONT_PX * state.ui_scale);
    for (entity, font) in q_panel.iter() {
        if font.is_some_and(|font| font.font_size == Some(wanted)) {
            continue;
        }
        commands.entity(entity).insert(InheritableFont {
            font_size: Some(wanted),
            ..default()
        });
    }
}

// Retained rows are spawned once, so filtering and folding hide them rather than
// rebuilding the list.
fn push_tree_rows(
    state: Res<Inspector>,
    mut q_rows: Query<(&TreeRow, &mut Node)>,
    q_bound: Query<(&NodeBound, Entity)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    let nodes = &state.hierarchy.nodes;
    let filter = state.hierarchy.filter.to_lowercase();

    let mut shown = vec![true; nodes.len()];
    let mut collapsed_at: Option<usize> = None;
    for (i, node) in nodes.iter().enumerate() {
        let parent = nodes.get(i + 1).is_some_and(|next| next.depth > node.depth);
        match collapsed_at {
            Some(collapsed) if node.depth > collapsed => {
                shown[i] = false;
                continue;
            }
            _ => collapsed_at = None,
        }
        if parent && !node.expanded {
            collapsed_at = Some(node.depth);
        }
        if !filter.is_empty() && !node.name.to_lowercase().contains(&filter) {
            shown[i] = false;
        }
    }

    for (row, mut node) in q_rows.iter_mut() {
        let wanted = if shown[row.0] {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != wanted {
            node.display = wanted;
        }
    }
    for (bound, entity) in q_bound.iter() {
        let value = match *bound {
            NodeBound::Expanded(i) => nodes[i].expanded,
            NodeBound::Visible(i) => nodes[i].visible,
        };
        commands.trigger(SetValue { entity, value });
    }
}

// The strip's pooled tabs and the header parts a system pushes into: retained tabs
// are spawned once, so opening and closing shows and hides slots.
#[derive(Component, Clone, Copy)]
struct DocSlot(usize);

#[derive(Component, Clone, Copy)]
struct DocName(usize);

#[derive(Component, Clone, Copy)]
struct DocDirty(usize);

#[derive(Component, Default, Clone)]
struct DocTabs;

#[derive(Component, Default, Clone)]
struct ViewportPane;

#[derive(Component, Default, Clone)]
struct ViewportLabel;

#[derive(Component, Default, Clone)]
struct HudCount;

#[allow(clippy::too_many_arguments)]
fn push_documents(
    state: Res<Inspector>,
    mut q_slots: Query<(&DocSlot, &mut Node)>,
    mut q_dots: Query<(&DocDirty, &mut Node), Without<DocSlot>>,
    mut q_names: Query<(&DocName, &mut Text)>,
    mut q_labels: Query<&mut Text, (With<ViewportLabel>, Without<DocName>)>,
    mut q_counts: Query<&mut Text, (With<HudCount>, Without<ViewportLabel>, Without<DocName>)>,
    q_tabs: Query<Entity, With<DocTabs>>,
    q_panes: Query<Entity, With<ViewportPane>>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    let documents = &state.documents;

    let set_display = |node: &mut Node, shown: bool| {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    for (slot, mut node) in q_slots.iter_mut() {
        set_display(&mut node, slot.0 < documents.open.len());
    }
    for (dot, mut node) in q_dots.iter_mut() {
        set_display(
            &mut node,
            documents.open.get(dot.0).is_some_and(|doc| doc.dirty),
        );
    }
    for (name, mut text) in q_names.iter_mut() {
        let wanted = documents.open.get(name.0).map_or("", |doc| &doc.name);
        if text.0 != wanted {
            text.0 = wanted.to_owned();
        }
    }
    for mut text in q_labels.iter_mut() {
        let wanted = documents
            .active_document()
            .map_or("No documents open", |doc| &doc.name);
        if text.0 != wanted {
            text.0 = wanted.to_owned();
        }
    }

    for mut text in q_counts.iter_mut() {
        let wanted = format!("{} open", documents.open.len());
        if text.0 != wanted {
            text.0 = wanted;
        }
    }

    let empty_slot = if documents.open.is_empty() {
        ThemeSlot::Neutral2
    } else {
        ThemeSlot::Transparent
    };
    for entity in q_panes.iter() {
        commands
            .entity(entity)
            .insert(ThemeBackgroundSlot(empty_slot));
    }
    if let Some(index) = documents
        .open
        .iter()
        .position(|doc| Some(doc.id) == documents.active)
    {
        for entity in q_tabs.iter() {
            commands.trigger(SetValue {
                entity,
                value: index,
            });
        }
    }
}

// Marks the panel column, the root of the UI-scale font cascade.
#[derive(Component, Default, Clone)]
struct InspectorPanel;

fn scene() -> impl SceneList {
    bsn_list![root()]
}

fn root() -> impl Scene {
    bsn! {
        screen()
        template_value(ThemeBackgroundSlot(ThemeSlot::Neutral0))
        Children [
            menu_bar(),
            (
                // The viewport and the panel, with a divider to re-proportion
                // them. The panel's old `width: 25%` is the starting fraction
                // and its `min_width` is now the splitter's floor, so the drag
                // stops where the layout would have anyway. The viewport's
                // minimum is `Auto` — whatever its content needs.
                @PlumeSplitter {
                    @fraction: 0.75,
                    @min_second: px(260),
                    @collapsible_second: true,
                    @first: bsn_list![documents()],
                    @second: bsn_list![panel()],
                    @auto_hide: true,

                }
                Node { flex_grow: 1.0 }
            ),
        ]
    }
}

fn menu_bar() -> impl Scene {
    bsn! {
        @PlumeMenuBar
        ThemeBackgroundSlot(ThemeSlot::Neutral1)
        Children [
            (
                @PlumeMenuButton { @label: "File" }
                Children [
                    (
                        @PlumeMenuButton {
                            @label: "New Document",
                            @shortcut: {Some("Ctrl+N".to_string())},
                        }
                        on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.add())
                    ),
                    (
                        @PlumeMenuButton {
                            @label: "Save",
                            @shortcut: {Some("Ctrl+S".to_string())},
                        }
                        on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.save_active())
                    ),
                    separator(),
                    (
                        @PlumeMenuButton { @label: "Recent" }
                        Children [
                            recent_item("corridor_00.rs"),
                            recent_item("vault_01.wgsl"),
                            recent_item("torch_02.ron"),
                        ]
                    ),
                    separator(),
                    (
                        @PlumeMenuButton { @label: "Exit" }
                        InteractionDisabled
                    ),
                ]
            ),
            (
                @PlumeMenuButton { @label: "View" }
                Children [
                    (
                        @PlumeMenuButton { @label: "Show HUD" }
                        Checkable
                        Checked
                        on(|ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                            s.show_hud = ev.value;
                        })
                    ),
                    (
                        @PlumeMenuButton { @label: "Autosave" }
                        Checkable
                        on(|ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                            s.autosave = ev.value;
                        })
                    ),
                ]
            ),
        ]
    }
}

fn recent_item(name: &str) -> impl Scene {
    let name = name.to_string();
    bsn! {
        @PlumeMenuButton { @label: {name.clone()} }
        on(move |_: On<Activate>| info!("open recent {name}"))
    }
}

// The View menu's HUD toggle, applied to the retained HUD dialog.
fn push_hud_visible(state: Res<Inspector>, mut q_hud: Query<&mut Node, With<HudRoot>>) {
    if !state.is_changed() {
        return;
    }
    for mut node in q_hud.iter_mut() {
        let wanted = if state.show_hud {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != wanted {
            node.display = wanted;
        }
    }
}

#[derive(Component, Default, Clone)]
struct HudRoot;

// The strip of open documents over the one viewport they share, the retained twin
// of the imm example's bodyless tab strip.
fn documents() -> impl Scene {
    let tabs: Vec<_> = (0..MAX_DOCUMENTS).map(document_tab).collect();
    bsn! {
        column()
        Node { flex_grow: 1.0, min_height: Val::ZERO, row_gap: Val::ZERO }
        Children [
            (
                row()
                Node {
                    column_gap: size::GAP_TIGHT,
                    padding: UiRect::right(size::GAP_TIGHT),
                }
                template_value(ThemeBackgroundSlot(ThemeSlot::Neutral1))
                Children [
                    (
                        // No `@body`, and no tab names one: a strip that only
                        // reports which document is showing.
                        @PlumeTabs { @header: {Box::new(tabs) as Box<dyn SceneList>} }
                        DocTabs
                        // The strip sits over the viewport, not over a surface.
                        template_value(TabSlot(ThemeSlot::Neutral0))
                        Node { width: Val::ZERO, flex_grow: 1.0 }
                        on(|ev: On<ValueChange<usize>>, mut s: ResMut<Inspector>| {
                            s.documents.active = s.documents.open.get(ev.value).map(|doc| doc.id);
                        })
                    ),
                    (
                        @PlumeToolButton {
                            @caption: bsn! { fa_icon(font_awesome::solid::PLUS) },
                            @variant: ButtonVariant::Plain,
                        }
                        Flat
                        Tooltip("Open a new document")
                        on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.add())
                    ),
                ]
            ),
            viewport(),
            viewport_hud(),
        ]
    }
}

fn document_tab(slot: usize) -> impl Scene {
    bsn! {
        @PlumeTab {
            @caption: bsn_list![
                fa_icon(font_awesome::solid::FILE_CODE),
                (
                    // `tab_label`'s box, hand-built because the caption inside it
                    // has to carry the marker `push_documents` writes through.
                    row()
                    Node { min_width: Val::ZERO, overflow: Overflow::clip() }
                    Children [
                        (
                            caption("")
                            template_value(DocName(slot))
                            Node { min_width: Val::ZERO }
                            TextLayout { linebreak: LineBreak::NoWrap }
                        ),
                    ]
                ),
                (
                    // Unsaved marker. A plain node rather than a glyph, so its size
                    // is the dot's own and not the header font's.
                    Node {
                        width: em(0.5),
                        height: em(0.5),
                        border_radius: BorderRadius::MAX_ELLIPTICAL,
                        flex_shrink: 0.0,
                        display: Display::None,
                    }
                    template_value(DocDirty(slot))
                    BackgroundColor(Color::WHITE)
                ),
                (
                    @PlumeToolButton {
                        @caption: bsn! { fa_icon(font_awesome::solid::XMARK) },
                        @variant: ButtonVariant::Plain,
                    }
                    Flat
                    on(move |_: On<Activate>, mut s: ResMut<Inspector>| {
                        if let Some(id) = s.documents.open.get(slot).map(|doc| doc.id) {
                            s.documents.close(id);
                        }
                    })
                ),
            ],
        }
        template_value(DocSlot(slot))
        Node { display: Display::None, min_width: em(6), }
    }
}

// The imm twin's floating HUD. A headerless `PlumeDialog` is the retained panel:
// absolutely positioned, so the corner it pins to is the pane's own.
fn viewport_hud() -> impl Scene {
    bsn! {
        @PlumeDialog {
            @header: false,
            @inset: {Corner::BottomRight.inset(px(16), px(16))},
            @contents: bsn_list![
                (
                    row()
                    Children [
                        fa_icon(font_awesome::solid::CUBES),
                        (caption("") HudCount),
                        separator(),
                        (
                            @PlumeToolButton {
                                @caption: bsn! { fa_icon(font_awesome::solid::FLOPPY_DISK) },
                                @variant: ButtonVariant::Plain,
                            }
                            Flat
                            Tooltip("Save the active document")
                            on(|_: On<Activate>, mut s: ResMut<Inspector>| {
                                s.documents.save_active();
                            })
                        ),
                    ]
                ),
            ],
        }
        HudRoot
    }
}

fn viewport() -> impl Scene {
    bsn! {
        column()
        ViewportPane
        Node { flex_grow: 1.0, min_height: Val::ZERO }
        Children [
            flex_spacer(),
            (
                row()
                Children [
                    flex_spacer(),
                    fa_icon(font_awesome::solid::CUBES),
                    (caption("") ViewportLabel),
                    flex_spacer(),
                ]
            ),
            flex_spacer(),
        ]
    }
}

fn panel() -> impl Scene {
    // Owned values, because `bsn!` defers these calls past any local's lifetime.
    let Inspector {
        ui_scale,
        material,
        hierarchy,
        ..
    } = Inspector::initial();
    bsn! {
        column()
        InspectorPanel
        // Width and minimum belong to the splitter now; the panel just fills
        // the pane it is given.
        Node {
            flex_grow: 1.0,
            padding: size::PAD,
        }
        template_value(ThemeBackgroundSlot(ThemeSlot::Neutral1))
        Children [
            header(),
            separator(),
            tabs(material, hierarchy.nodes),
            separator(),
            footer(ui_scale),
        ]
    }
}

fn header() -> impl Scene {
    bsn! {
        row()
        Children [
            fa_icon(font_awesome::solid::SLIDERS),
            caption_small_caps("Inspector"),
            flex_spacer(),
            (
                @PlumeToolButton {
                    @caption: bsn! { fa_icon(font_awesome::solid::FLOPPY_DISK) },
                }
                Tooltip("Save material")
            ),
            (
                @PlumeToolButton {
                    @caption: bsn! { fa_icon(font_awesome::solid::ARROW_ROTATE_LEFT) },
                }
                Tooltip("Revert to the last saved values")
                on(|_: On<Activate>, mut s: ResMut<Inspector>| {
                    (s.material, s.hierarchy) = Default::default();
                })
            ),
        ]
    }
}

fn tabs(material: Material, nodes: Vec<SceneNode>) -> impl Scene {
    bsn! {
        @PlumeTabs {
            @header: bsn_list![
                (
                    @PlumeTab {
                        @caption: bsn_list![
                            fa_icon(font_awesome::solid::PALETTE),
                            caption("Material"),
                        ],
                        @target: #material,
                    }
                    Selected
                ),
                @PlumeTab {
                    @caption: bsn_list![
                        fa_icon(font_awesome::solid::SITEMAP),
                        caption("Hierarchy"),
                    ],
                    @target: #hierarchy,
                },
            ],
            @body: bsn_list![
                (
                    #material
                    tab_body()
                    Children [
                        material_tab(material),
                    ]
                ),
                (
                    #hierarchy
                    tab_body()
                    Children [
                        hierarchy_tab(nodes),
                    ]
                ),
            ],
        }
        Node { flex_grow: 1.0, min_height: Val::ZERO }
        on(|ev: On<ValueChange<usize>>, mut s: ResMut<Inspector>| {
            s.tab = Tab::from_index(ev.value);
        })
    }
}

fn material_tab(m: Material) -> impl Scene {
    let (name, base, emissive) = (m.name, m.base_color, m.emissive);
    let (metallic, roughness) = (m.metallic, m.roughness);
    let (blend, cull) = (m.blend, m.cull);
    bsn! {
        @PlumeScrollArea {
            @contents: bsn_list![
                @PlumeSection {
                    @header: bsn! { caption_small_caps("Surface") },
                    @contents: bsn_list![
                        (
                            row()
                            Children [
                                field_label("Name"),
                                (
                                    @PlumeColorSwatch
                                    template_value(ColorSwatchValue(base))
                                ),
                                (
                                    @PlumeTextInput {
                                        @value: name,
                                        @placeholder: {Some("Material name".to_string())},
                                    }
                                    Node { width: Val::ZERO, flex_grow: 1.0 }
                                    on_text(Bound::Name)
                                ),
                            ]
                        ),
                        color_row("Base color", base, Bound::BaseColor),
                        color_row("Emissive", emissive, Bound::Emissive),
                    ],
                },
                @PlumeSection {
                    @header: bsn! { caption_small_caps("Shading") },
                    @contents: bsn_list![
                        radio_row("Blend", blend, Bound::Blend),
                        radio_row("Cull", cull, Bound::Cull),
                        slider_row("Metallic", metallic, Bound::Metallic),
                        slider_row("Roughness", roughness, Bound::Roughness),
                    ],
                },
                (
                    @PlumeSection {
                        @header: bsn! { caption_small_caps("Color picker") },
                        @contents: bsn_list![
                            (
                                @PlumeColorPicker { @initial_color: base }
                                on_color(Bound::Picker)
                            ),
                        ],
                    }
                    SectionCollapsed
                ),
            ],
        }
    }
}

fn hierarchy_tab(nodes: Vec<SceneNode>) -> impl Scene {
    let rows: Vec<_> = nodes
        .into_iter()
        .enumerate()
        .map(|(i, node)| node_row(i, node))
        .collect();
    bsn! {
        column()
        Node { flex_grow: 1.0, min_height: Val::ZERO }
        Children [
            (
                row()
                Children [
                    fa_icon(font_awesome::solid::MAGNIFYING_GLASS),
                    (
                        @PlumeTextInput { @placeholder: {Some("Filter…".to_string())} }
                        Node { width: Val::ZERO, flex_grow: 1.0 }
                        on_text(Bound::Filter)
                    ),
                ]
            ),
            @PlumeScrollArea { @contents: {Box::new(rows) as Box<dyn SceneList>} },
        ]
    }
}

fn node_row(index: usize, node: SceneNode) -> impl Scene {
    let (name, icon, depth) = (node.name, node.icon, node.depth);
    let parent_of_next = node.expanded;
    bsn! {
        row()
        TreeRow(index)
        Children [
            space(px(depth as f32 * 14.0)),
            (
                @PlumeDisclosure
                template_value(NodeBound::Expanded(index))
                {parent_of_next.then(|| bsn! { Checked })}
                on(move |ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    s.hierarchy.nodes[index].expanded = ev.value;
                })
            ),
            fa_icon(icon),
            caption(name),
            flex_spacer(),
            (
                @PlumeToggleSwitch
                template_value(NodeBound::Visible(index))
                Checked
                on(move |ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    s.hierarchy.nodes[index].visible = ev.value;
                })
            ),
        ]
    }
}

fn footer(scale: f32) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label("UI scale"),
            (
                @PlumeSlider {
                    @min: 0.5,
                    @max: 2.0,
                    @precision: {Some(2)},
                }
                SliderValue(scale)
                Node { width: Val::ZERO, flex_grow: 1.0 }
                on_number(Bound::UiScale)
            ),
        ]
    }
}

fn field_label(text: &str) -> impl Scene {
    let text = text.to_string();
    bsn! {
        caption(text)
        Node { width: GUTTER }
    }
}

fn color_row(label: &str, color: Color, bound: Bound) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeColorEdit { @initial_color: color }
                on_color(bound)
            ),
        ]
    }
}

fn slider_row(label: &str, value: f32, bound: Bound) -> impl Scene {
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeSlider {
                    @min: 0.0,
                    @max: 1.0,
                    @precision: {Some(2)},
                }
                SliderValue(value)
                Node { width: Val::ZERO, flex_grow: 1.0 }
                on_number(bound)
            ),
        ]
    }
}

fn radio_row<T: Options>(label: &str, selected: T, bound: Bound) -> impl Scene {
    let radios: Vec<_> = T::ALL
        .iter()
        .map(|&option| {
            let checked = option == selected;
            bsn! {
                @PlumeRadio { @caption: bsn! { caption(option.label().to_string()) } }
                {checked.then(|| bsn! { Checked })}
            }
        })
        .collect();
    bsn! {
        row()
        Children [
            field_label(label),
            (
                @PlumeRadioGroup
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: size::GAP,
                }
                template_value(bound)
                Children [
                    {radios},
                ]
                on_choice(bound)
            ),
        ]
    }
}
