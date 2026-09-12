//! Retained (bsn!) twin of the `inspector_panel` example: the same docked panel over
//! the same `Inspector` resource, bound with `on()` observers instead of imm's `&mut`.
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use bevy_jp_plume::retained::{
    Activate, Checkable, Checked, Flat, InteractionDisabled, PlumeButton, PlumeColorEdit,
    PlumeColorPicker, PlumeColorSwatch, PlumeDialog, PlumeDisclosure, PlumeMenuBar,
    PlumeMenuButton, PlumeModal, PlumeRadio, PlumeRadioGroup, PlumeReorderable,
    PlumeReorderableItem, PlumeScrollArea, PlumeSection, PlumeSelect, PlumeSlider, PlumeSplitter,
    PlumeTab, PlumeTabs, PlumeTextInput, PlumeToggleSwitch, PlumeToolButton, Propagate,
    ReorderMove, RequestClose, SectionCollapsed, Selected, SetValue, SliderValue,
    ThemeBackgroundSlot, Tooltip, ValueChange, caption, column, flex_spacer, icon, menu_anchor,
    modal_title, row, screen, separator, small_caps, space,
};

#[path = "common/mod.rs"]
mod common;

use common::Options;
use common::inspector_panel::{
    Blend, Cull, Inspector, Layer, LayerId, MAX_DOCUMENTS, Material, SceneNode, Tab, hud_theme,
    register_hud_theme, viewport_color,
};

const GUTTER: Val = Val::Em(6.0);

fn main() {
    let mut app = common::demo_app(false);
    app.insert_resource(Inspector::initial())
        .add_systems(Startup, (register_hud_theme, scene.spawn()))
        .add_systems(
            Update,
            (
                common::log_on_change::<Inspector>,
                push_material,
                push_tree_rows,
                push_documents,
                push_hud_visible,
                push_panel_tab,
                push_rem_value,
                push_layers,
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
    Filter,
    Picker,
    // By identity, not index: a dragged row keeps its binding.
    Opacity(LayerId),
}

impl Bound {
    fn number(self, s: &mut Inspector) -> Option<&mut f32> {
        Some(match self {
            Bound::Metallic => &mut s.material.metallic,
            Bound::Roughness => &mut s.material.roughness,
            Bound::Opacity(id) => &mut layer_mut(s, id)?.opacity,
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

// The layer list's root, its rows, and each row's enabled toggle.
#[derive(Component, Clone, Copy, Default)]
struct LayerList;

#[derive(Component, Clone, Copy, Default)]
struct LayerRow(LayerId);

#[derive(Component, Clone, Copy, Default)]
struct LayerEnabled(LayerId);

fn layer_mut(s: &mut Inspector, id: LayerId) -> Option<&mut Layer> {
    s.material.layers.iter_mut().find(|layer| layer.id == id)
}

// The list's order is its `Children` order; a model-side change (Revert)
// is pushed by re-parenting the rows in model order. A drag has already moved
// the row, so its own step finds nothing to do here.
fn push_layers(
    state: Res<Inspector>,
    q_lists: Query<(Entity, &Children), With<LayerList>>,
    q_rows: Query<&LayerRow>,
    q_enabled: Query<(Entity, &LayerEnabled)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    let layers = &state.material.layers;
    for (list, children) in q_lists.iter() {
        let row_of = |id: LayerId| {
            children
                .iter()
                .find(|child| q_rows.get(*child).is_ok_and(|row| row.0 == id))
        };
        let ordered: Vec<Entity> = layers.iter().filter_map(|layer| row_of(layer.id)).collect();
        if ordered.len() == children.len() && ordered.iter().copied().ne(children.iter()) {
            commands.entity(list).replace_children(&ordered);
        }
    }
    for (entity, enabled) in q_enabled.iter() {
        if let Some(layer) = layers.iter().find(|layer| layer.id == enabled.0) {
            let value = layer.enabled;
            commands.trigger(SetValue { entity, value });
        }
    }
}

// Marks a panel tab's body with the tab that shows it; the strip is bodyless,
// so `push_panel_tab` does the swapping.
#[derive(Component, Clone, Copy, Default)]
struct TabPane(Tab);

// Marks the footer's live rem readout.
#[derive(Component, Default, Clone)]
struct RemValue;

fn push_panel_tab(state: Res<Inspector>, mut q_panes: Query<(&TabPane, &mut Node)>) {
    if !state.is_changed() {
        return;
    }
    for (pane, mut node) in q_panes.iter_mut() {
        let wanted = if pane.0 == state.tab {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != wanted {
            node.display = wanted;
        }
    }
}

fn push_rem_value(rem_size: Res<RemSize>, mut q_values: Query<&mut Text, With<RemValue>>) {
    if !rem_size.is_changed() {
        return;
    }
    for mut text in q_values.iter_mut() {
        let wanted = format!("{}", rem_size.0);
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

// Marks the per-node controls, which bind by index rather than by field.
#[derive(Component, Clone, Copy)]
enum NodeBound {
    Expanded(usize),
    Visible(usize),
}

// Manual: `#[default]` needs a unit variant, and both variants carry an index.
impl Default for NodeBound {
    fn default() -> Self {
        NodeBound::Expanded(0)
    }
}

fn on_number(bound: Bound) -> impl Scene {
    bsn! {
        bound
        on(move |ev: On<ValueChange<f32>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.number(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_text(bound: Bound) -> impl Scene {
    bsn! {
        bound
        on(move |ev: On<ValueChange<String>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.text(&mut s) {
                field.clone_from(&ev.value);
            }
        })
    }
}

fn on_color(bound: Bound) -> impl Scene {
    bsn! {
        bound
        on(move |ev: On<ValueChange<Color>>, mut s: ResMut<Inspector>| {
            if let Some(field) = bound.color(&mut s) {
                *field = ev.value;
            }
        })
    }
}

fn on_choice(bound: Bound) -> impl Scene {
    bsn! {
        bound
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
#[derive(Component, Clone, Copy, Default)]
struct DocSlot(usize);

#[derive(Component, Clone, Copy, Default)]
struct DocName(usize);

#[derive(Component, Clone, Copy, Default)]
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

    for entity in q_panes.iter() {
        // The imm twin paints a hue per document here; a themed slot would keep
        // overwriting it, so the two are mutually exclusive on this entity.
        match documents.active_document() {
            Some(document) => {
                commands
                    .entity(entity)
                    .remove::<ThemeBackgroundSlot>()
                    .insert(BackgroundColor(viewport_color(document.id)));
            }
            None => {
                commands
                    .entity(entity)
                    .insert(ThemeBackgroundSlot(ThemeSlot::Neutral1));
            }
        }
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
    bsn! { @root() }
}

fn root() -> impl Scene {
    bsn! {
        @screen()
        ThemeBackgroundSlot(ThemeSlot::Neutral0)
        Children [
            @top_bar()
            --
            // The viewport and the panel, with a divider to re-proportion
            // them. The panel is the sized pane, anchored in em so it holds
            // its width (and tracks the font size) while the viewport
            // absorbs window resizes; `min_second` is its floor, so the
            // drag stops where the layout would have anyway. The viewport
            // has no floor of its own and gives all the way down.
            @PlumeSplitter {
                @size: em(30),
                @sized_pane: SplitPane::Second,
                @min_first: Val::ZERO,
                @min_second: em(28),
                @collapsible_second: true,
                @first: bsn! { @documents() },
                @second: bsn! { @panel() },
                @auto_hide: true,
            }
            Node { flex_grow: 1.0 }
        ]
    }
}

// The imm twin's `export_modal`: a select and tooltipped buttons inside a modal,
// which is where their layering has to be checked - the select's popup over the
// barrier, the tooltips over everything.
fn export_modal() -> impl Scene {
    bsn! {
        @PlumeModal {
            @title: bsn! { @modal_title("Export Scene") },
            @width: em(26.0),
            @contents: bsn! {
                @caption("Nothing behind this takes a click until it is answered.")
                --
                @row()
                Children [
                    @caption("Culling") Node { width: em(6.0) }
                    --
                    @PlumeSelect {
                        @options: {Cull::select_options()},
                        @selected: 0,
                    }
                    Node { width: Val::ZERO, flex_grow: 1.0 }
                ]
                --
                @separator()
                --
                @row()
                Children [
                    @PlumeToolButton {
                        @caption: bsn! { @icon(lucide::FOLDER_OPEN) },
                    }
                    Tooltip("Pick the output directory")
                    --
                    @PlumeToolButton {
                        @caption: bsn! { @icon(lucide::UNDO_2) },
                    }
                    Tooltip("Reset these settings to their defaults")
                    --
                    @flex_spacer()
                    --
                    @PlumeButton {
                        @caption: bsn! { @caption("Cancel") },
                        @variant: ButtonVariant::Outline,
                    }
                    Tooltip("Close without writing anything")
                    on(close_export_modal)
                    --
                    @PlumeButton {
                        @caption: bsn! { @caption("Export") },
                        @variant: ButtonVariant::Primary,
                    }
                    Tooltip("Write the scene with the settings above")
                    on(close_export_modal)
                ]
            },
        }
    }
}

// `RequestClose` propagates, so triggering it on the button reaches the modal.
fn close_export_modal(activate: On<Activate>, mut commands: Commands) {
    commands.trigger(RequestClose {
        source: activate.event_target(),
    });
}

// The menus and the document strip share one line, the retained twin of the imm
// example's `top_bar`.
fn top_bar() -> impl Scene {
    let tabs: Vec<_> = (0..MAX_DOCUMENTS).map(document_tab).collect();
    bsn! {
        @row()
        Children [
            @menu_bar()
            --
            @separator()
            --
            @row()
            Children [
                // No `@body`, and no tab names one: a strip that only
                // reports which document is showing.
                @PlumeTabs { @header: {Box::new(tabs) as Box<dyn SceneList>} }
                DocTabs
                Node { flex_basis: Val::ZERO, flex_grow: 1.0 }
                on(|ev: On<ValueChange<usize>>, mut s: ResMut<Inspector>| {
                    s.documents.active = s.documents.open.get(ev.value).map(|doc| doc.id);
                })
                --
                @PlumeToolButton {
                    @caption: bsn! { @icon(lucide::PLUS) },
                    @variant: ButtonVariant::Plain,
                }
                Flat
                Tooltip("Open a new document")
                on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.add())
            ]
        ]
    }
}

fn menu_bar() -> impl Scene {
    bsn! {
        @PlumeMenuBar
        Children [
            @PlumeMenuButton { @label: "File" }
            Children [
                @PlumeMenuButton {
                    @label: "New Document",
                    @shortcut: {Some("Ctrl+N".to_string())},
                }
                on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.add())
                --
                @PlumeMenuButton {
                    @label: "Save",
                    @shortcut: {Some("Ctrl+S".to_string())},
                }
                on(|_: On<Activate>, mut s: ResMut<Inspector>| s.documents.save_active())
                --
                @separator()
                --
                @PlumeMenuButton { @label: "Recent" }
                Children [
                    @recent_item("corridor_00.rs")
                    --
                    @recent_item("vault_01.wgsl")
                    --
                    @recent_item("torch_02.ron")
                ]
                --
                @separator()
                --
                @PlumeMenuButton { @label: "Export…" }
                // Parentless: the barrier is a fixed, full-viewport layout
                // root, so it needs no place in the tree.
                on(|_: On<Activate>,
                    q_open: Query<(), With<PlumeModal>>,
                    mut commands: Commands| {
                    if q_open.iter().next().is_none() {
                        commands.spawn_scene(export_modal());
                    }
                })
                --
                @separator()
                --
                @PlumeMenuButton { @label: "Exit" }
                InteractionDisabled
            ]
            --
            @PlumeMenuButton { @label: "View" }
            Children [
                @PlumeMenuButton { @label: "Show HUD" }
                Checkable
                Checked
                on(|ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    s.show_hud = ev.value;
                })
                --
                @PlumeMenuButton { @label: "Autosave" }
                Checkable
                on(|ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    s.autosave = ev.value;
                })
            ]
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

// The one viewport the documents share, with the HUD floating in its corner;
// the strip that picks the document lives in `top_bar`.
fn documents() -> impl Scene {
    bsn! {
        @column()
        Node {
            flex_grow: 1.0,
            min_height: Val::ZERO,
            row_gap: Val::ZERO,
        }
        Children [
            @viewport()
            --
            @viewport_hud()
        ]
    }
}

fn document_tab(slot: usize) -> impl Scene {
    bsn! {
        @PlumeTab {
            @caption: bsn! {
                @icon(lucide::FILE_CODE)
                --
                // `tab_label`'s box, hand-built because the caption inside it
                // has to carry the marker `push_documents` writes through.
                @row()
                Node { min_width: Val::ZERO, overflow: Overflow::clip() }
                Children [
                    @caption("")
                    DocName(slot)
                    Node { min_width: Val::ZERO }
                    TextLayout { linebreak: LineBreak::NoWrap }
                ]
                --
                // Unsaved marker. A plain node rather than a glyph, so its size
                // is the dot's own and not the header font's.
                Node {
                    width: em(0.5),
                    height: em(0.5),
                    border_radius: BorderRadius::MAX_ELLIPTICAL,
                    flex_shrink: 0.0,
                    display: Display::None,
                }
                DocDirty(slot)
                BackgroundColor(Color::WHITE)
                --
                @PlumeToolButton {
                    @caption: bsn! { @icon(lucide::X) },
                    @variant: ButtonVariant::Plain,
                }
                Flat
                on(move |_: On<Activate>, mut s: ResMut<Inspector>| {
                    if let Some(id) = s.documents.open.get(slot).map(|doc| doc.id) {
                        s.documents.close(id);
                    }
                })
            },
        }
        DocSlot(slot)
        Node { display: Display::None, min_width: em(6) }
    }
}

// The imm twin's floating HUD. A headerless `PlumeDialog` is the retained panel:
// absolutely positioned, so the corner it pins to is the pane's own. The
// `Propagate` is the retained spelling of the imm twin's `.theme(hud_theme())`.
fn viewport_hud() -> impl Scene {
    let theme = hud_theme();
    bsn! {
        Propagate::<ThemeId>(theme)
        @PlumeDialog {
            @header: false,
            @inset: {Corner::BottomRight.inset(em(1), em(1))},
            @contents: bsn! {
                @row()
                Children [
                    @icon(lucide::BOXES)
                    --
                    @caption("")
                    HudCount
                    --
                    @separator()
                    --
                    @PlumeToolButton {
                        @caption: bsn! { @icon(lucide::SAVE) },
                        @variant: ButtonVariant::Plain,
                    }
                    Flat
                    Tooltip("Save the active document")
                    on(|_: On<Activate>, mut s: ResMut<Inspector>| {
                        s.documents.save_active();
                    })
                ]
            },
        }
        HudRoot
    }
}

fn viewport() -> impl Scene {
    bsn! {
        @column()
        ViewportPane
        Node { flex_grow: 1.0, min_height: Val::ZERO }
        Children [
            @flex_spacer()
            --
            @row()
            Children [
                @flex_spacer()
                --
                @icon(lucide::BOXES)
                --
                @caption("")
                ViewportLabel
                --
                @flex_spacer()
            ]
            --
            @flex_spacer()
        ]
    }
}

fn panel() -> impl Scene {
    // Owned values, because `bsn!` defers these calls past any local's lifetime.
    let Inspector {
        material,
        hierarchy,
        ..
    } = Inspector::initial();
    bsn! {
        @column()
        InspectorPanel
        // Width and minimum belong to the splitter now; the panel just fills
        // the pane it is given. The tab row sits on the screen's own ground;
        // only the body below it carries the panel slab.
        Node {
            flex_grow: 1.0,
            min_height: Val::ZERO,
            row_gap: Val::ZERO,
        }
        Children [
            @tab_row()
            --
            @column()
            Node {
                flex_grow: 1.0,
                min_height: Val::ZERO,
                padding: size::SPACE,
            }
            ThemeBackgroundSlot(ThemeSlot::Neutral1)
            Children [
                @tab_pane(Tab::Material, bsn! { @material_tab(material) })
                --
                @tab_pane(Tab::Hierarchy, bsn! { @hierarchy_tab(hierarchy.nodes) })
                --
                @separator()
                --
                @footer()
            ]
        ]
    }
}

// The panel's tab buttons with the material actions on the same row, the
// retained twin of the imm example's bodyless strip: the tabs only report, and
// `push_panel_tab` swaps the panes below.
fn tab_row() -> impl Scene {
    bsn! {
        @row()
        Node { padding: UiRect::right(size::SPACE) }
        Children [
            @PlumeTabs {
                @header: bsn! {
                    @PlumeTab {
                        @caption: bsn! {
                            @icon(lucide::PALETTE)
                            --
                            @caption("Material")
                        },
                    }
                    Selected
                    --
                    @PlumeTab {
                        @caption: bsn! {
                            @icon(lucide::FOLDER_TREE)
                            --
                            @caption("Hierarchy")
                        },
                    }
                },
            }
            on(|ev: On<ValueChange<usize>>, mut s: ResMut<Inspector>| {
                s.tab = Tab::from_index(ev.value);
            })
            --
            @flex_spacer()
            --
            @PlumeToolButton {
                @caption: bsn! { @icon(lucide::SAVE) },
            }
            Flat
            Tooltip("Save material")
            --
            @PlumeToolButton {
                @caption: bsn! { @icon(lucide::UNDO_2) },
            }
            Flat
            Tooltip("Revert to the last saved values")
            on(|_: On<Activate>, mut s: ResMut<Inspector>| {
                (s.material, s.hierarchy) = Default::default();
            })
            --
            // The imm twin's `.menu()`: a menu hung off the app's own button,
            // which keeps its tool-button look. Rows are the menu bar's.
            @PlumeToolButton {
                @caption: bsn! { @icon(lucide::ELLIPSIS_VERTICAL) },
            }
            Flat
            Tooltip("More material actions")
            @menu_anchor(bsn! {
                @PlumeMenuButton {
                    @label: "Copy Values",
                    @shortcut: {Some("Ctrl+C".to_string())},
                }
                on(|_: On<Activate>| info!("copy material values"))
                --
                @PlumeMenuButton { @label: "Paste Values" }
                InteractionDisabled
                --
                @separator()
                --
                @PlumeMenuButton { @label: "Autosave" }
                Checkable
                on(|ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    s.autosave = ev.value;
                })
            })
        ]
    }
}

// One tab's body, shown while its tab is the selected one.
fn tab_pane(tab: Tab, contents: impl SceneList) -> impl Scene {
    let display = if tab == Tab::default() {
        Display::Flex
    } else {
        Display::None
    };
    bsn! {
        @column()
        TabPane(tab)
        Node {
            flex_grow: 1.0,
            min_height: Val::ZERO,
            display: display,
        }
        Children [
            {contents}
        ]
    }
}

fn material_tab(m: Material) -> impl Scene {
    let (name, base, emissive) = (m.name, m.base_color, m.emissive);
    let (metallic, roughness) = (m.metallic, m.roughness);
    let (blend, cull) = (m.blend, m.cull);
    let layers = m.layers;
    bsn! {
        @PlumeScrollArea {
            @contents: bsn! {
                @PlumeSection {
                    @header: bsn! {
                        @caption("Surface")
                        @small_caps()
                    },
                    @contents: bsn! {
                        @row()
                        Children [
                            @field_label("Name")
                            --
                            @PlumeColorSwatch { @initial_color: base }
                            --
                            @PlumeTextInput {
                                @value: name,
                                @placeholder: {Some("Material name".to_string())},
                            }
                            Node { width: Val::ZERO, flex_grow: 1.0 }
                            @on_text(Bound::Name)
                        ]
                        --
                        @color_row("Base color", base, Bound::BaseColor)
                        --
                        @color_row("Emissive", emissive, Bound::Emissive)
                    },
                }
                --
                @PlumeSection {
                    @header: bsn! {
                        @caption("Shading")
                        @small_caps()
                    },
                    @contents: bsn! {
                        @radio_row("Blend", blend, Bound::Blend)
                        --
                        @radio_row("Cull", cull, Bound::Cull)
                        --
                        @slider_row("Metallic", metallic, Bound::Metallic)
                        --
                        @slider_row("Roughness", roughness, Bound::Roughness)
                    },
                }
                --
                @layers_section(layers)
                --
                @PlumeSection {
                    @header: bsn! {
                        @caption("Color picker")
                        @small_caps()
                    },
                    @contents: bsn! {
                        @PlumeColorPicker { @initial_color: base }
                        @on_color(Bound::Picker)
                    },
                }
                SectionCollapsed
            },
        }
    }
}

// The imm twin's `ui.reorderable`: rows the grip drags, each step landing in the
// model through `ValueChange<ReorderMove>` on the list.
fn layers_section(layers: Vec<Layer>) -> impl Scene {
    let rows: Vec<_> = layers.into_iter().map(layer_row).collect();
    bsn! {
        @PlumeSection {
            @header: bsn! {
                @caption("Layers")
                @small_caps()
            },
            @contents: bsn! {
                @PlumeReorderable { @contents: {Box::new(rows) as Box<dyn SceneList>} }
                LayerList
                on(|ev: On<ValueChange<ReorderMove>>, mut s: ResMut<Inspector>| {
                    ev.value.apply(&mut s.material.layers);
                })
            },
        }
    }
}

fn layer_row(layer: Layer) -> impl Scene {
    let Layer {
        id,
        name,
        icon: glyph,
        opacity,
        enabled,
    } = layer;
    let label_width = GUTTER * 0.6;
    bsn! {
        @PlumeReorderableItem {
            @contents: bsn! {
                @icon(glyph)
                --
                @caption(name)
                Node { width: label_width }
                --
                @PlumeSlider {
                    @min: 0.0,
                    @max: 1.0,
                    @precision: {Some(2)},
                }
                SliderValue(opacity)
                Node { width: Val::ZERO, flex_grow: 1.0 }
                @on_number(Bound::Opacity(id))
                --
                @PlumeToggleSwitch
                LayerEnabled(id)
                @{enabled.then(|| bsn! { Checked })}
                on(move |ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                    if let Some(layer) = layer_mut(&mut s, id) {
                        layer.enabled = ev.value;
                    }
                })
            },
        }
        LayerRow(id)
    }
}

fn hierarchy_tab(nodes: Vec<SceneNode>) -> impl Scene {
    let rows: Vec<_> = nodes
        .into_iter()
        .enumerate()
        .map(|(i, node)| node_row(i, node))
        .collect();
    bsn! {
        @column()
        Node { flex_grow: 1.0, min_height: Val::ZERO }
        Children [
            @row()
            Children [
                @icon(lucide::SEARCH)
                --
                @PlumeTextInput { @placeholder: {Some("Filter…".to_string())} }
                Node { width: Val::ZERO, flex_grow: 1.0 }
                @on_text(Bound::Filter)
            ]
            --
            @PlumeScrollArea { @contents: {Box::new(rows) as Box<dyn SceneList>} }
        ]
    }
}

fn node_row(index: usize, node: SceneNode) -> impl Scene {
    let (name, glyph, depth) = (node.name, node.icon, node.depth);
    let parent_of_next = node.expanded;
    bsn! {
        @row()
        TreeRow(index)
        Children [
            @space(em(depth))
            --
            @PlumeDisclosure
            NodeBound::Expanded(index)
            @{parent_of_next.then(|| bsn! { Checked })}
            on(move |ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                s.hierarchy.nodes[index].expanded = ev.value;
            })
            --
            @icon(glyph)
            --
            @caption(name)
            --
            @flex_spacer()
            --
            @PlumeToggleSwitch
            NodeBound::Visible(index)
            Checked
            on(move |ev: On<ValueChange<bool>>, mut s: ResMut<Inspector>| {
                s.hierarchy.nodes[index].visible = ev.value;
            })
        ]
    }
}

fn footer() -> impl Scene {
    bsn! {
        @row()
        Children [
            @field_label("Rem")
            --
            @PlumeSlider {
                @min: 10.,
                @max: 20.,
                @step: {Some(1.)},
                @precision: {Some(0)},
            }
            SliderValue(size::MEDIUM_FONT_PX)
            Node { width: Val::ZERO, flex_grow: 1.0 }
            on(move |ev: On<ValueChange<f32>>, mut r: ResMut<RemSize>| {
                r.0 = ev.value;
            })
            --
            @caption(format!("{}", size::MEDIUM_FONT_PX))
            RemValue
            Node { width: em(2) }
        ]
    }
}

fn field_label(text: &str) -> impl Scene {
    let text = text.to_string();
    bsn! {
        @caption(text)
        Node { width: GUTTER }
    }
}

fn color_row(label: &str, color: Color, bound: Bound) -> impl Scene {
    bsn! {
        @row()
        Children [
            @field_label(label)
            --
            @PlumeColorEdit { @initial_color: color }
            @on_color(bound)
        ]
    }
}

fn slider_row(label: &str, value: f32, bound: Bound) -> impl Scene {
    bsn! {
        @row()
        Children [
            @field_label(label)
            --
            @PlumeSlider {
                @min: 0.0,
                @max: 1.0,
                @precision: {Some(2)},
            }
            SliderValue(value)
            Node { width: Val::ZERO, flex_grow: 1.0 }
            @on_number(bound)
        ]
    }
}

fn radio_row<T: Options>(label: &str, selected: T, bound: Bound) -> impl Scene {
    let radios: Vec<_> = T::ALL
        .iter()
        .map(|&option| {
            let checked = option == selected;
            bsn! {
                @PlumeRadio { @caption: bsn! { @caption(option.label().to_string()) } }
                @{checked.then(|| bsn! { Checked })}
            }
        })
        .collect();
    bsn! {
        @row()
        Children [
            @field_label(label)
            --
            @PlumeRadioGroup
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: size::SPACE,
            }
            bound
            Children [
                {radios}
            ]
            @on_choice(bound)
        ]
    }
}
