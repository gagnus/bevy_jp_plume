//! Editor inspector as a self-contained feature plugin: a docked side panel whose
//! two tabs hold a material editor and a scrolling scene tree, under a top bar
//! where the menus share a line with the document strip.
use std::cell::Cell;

use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use super::{Options, log_on_change};

/// Base font size the panel scales from.
pub const BASE_FONT_PX: f32 = 14.0;

const GUTTER: Val = Val::Em(6.0);

/// Cap on open documents; the retained twin spawns a fixed tab pool this size.
pub const MAX_DOCUMENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    Material,
    Hierarchy,
}

impl Options for Tab {
    const ALL: &'static [Self] = &[Self::Material, Self::Hierarchy];

    fn label(self) -> &'static str {
        match self {
            Self::Material => "Material",
            Self::Hierarchy => "Hierarchy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Blend {
    #[default]
    Opaque,
    Mask,
    Translucent,
}

impl Options for Blend {
    const ALL: &'static [Self] = &[Self::Opaque, Self::Mask, Self::Translucent];

    fn label(self) -> &'static str {
        match self {
            Self::Opaque => "Opaque",
            Self::Mask => "Mask",
            Self::Translucent => "Blend",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Cull {
    #[default]
    Back,
    Front,
    None,
}

impl Options for Cull {
    const ALL: &'static [Self] = &[Self::Back, Self::Front, Self::None];

    fn label(self) -> &'static str {
        match self {
            Self::Back => "Back",
            Self::Front => "Front",
            Self::None => "None",
        }
    }
}

/// One row of the scene tree: a name, a type icon, and its fold/visibility state.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    pub name: String,
    pub icon: Icon,
    pub depth: usize,
    pub expanded: bool,
    pub visible: bool,
}

impl SceneNode {
    fn new(name: &str, icon: Icon, depth: usize) -> Self {
        Self {
            name: name.into(),
            icon,
            depth,
            expanded: true,
            visible: true,
        }
    }
}

/// One open "document": what a tab in the strip stands for.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: DocId,
    pub name: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocId(pub u32);

/// The open documents and which one the viewport is showing.
#[derive(Debug, Clone, PartialEq)]
pub struct Documents {
    pub open: Vec<Document>,
    pub active: Option<DocId>,
    next_id: u32,
}

impl Default for Documents {
    fn default() -> Self {
        let mut documents = Self {
            open: Vec::new(),
            active: None,
            next_id: 0,
        };
        for _ in 0..MAX_DOCUMENTS / 4 {
            documents.add();
        }
        documents.open[0].dirty = false;
        documents.open[2].dirty = false;
        documents.active = documents.open.first().map(|doc| doc.id);
        documents
    }
}

impl Documents {
    /// Open a fresh, unsaved document and make it the active one.
    pub fn add(&mut self) {
        const STEMS: &[&str] = &[
            "corridor", "vault", "torch", "chest", "spawn", "ramp", "portal", "shrine",
        ];
        const EXTENSIONS: &[&str] = &["rs", "wgsl", "ron"];
        if self.open.len() >= MAX_DOCUMENTS {
            return;
        }
        let id = DocId(self.next_id);
        self.next_id += 1;
        let index = id.0 as usize;
        let name = format!(
            "{}_{:02}.{}",
            STEMS[index % STEMS.len()],
            id.0,
            EXTENSIONS[index % EXTENSIONS.len()]
        );
        self.open.push(Document {
            id,
            name,
            dirty: true,
        });
        self.active = Some(id);
    }

    pub fn close(&mut self, id: DocId) {
        let Some(index) = self.open.iter().position(|doc| doc.id == id) else {
            return;
        };
        let closed = self.open.remove(index);
        info!("closed document {}", closed.name);
        if self.active == Some(id) {
            // The neighbour that slid into its place, or the new last one.
            self.active = self
                .open
                .get(index)
                .or_else(|| self.open.last())
                .map(|doc| doc.id);
        }
    }

    pub fn save_active(&mut self) {
        if let Some(doc) = self.active_mut() {
            doc.dirty = false;
        }
    }

    pub fn active_document(&self) -> Option<&Document> {
        self.open.iter().find(|doc| Some(doc.id) == self.active)
    }

    fn active_mut(&mut self) -> Option<&mut Document> {
        let active = self.active;
        self.open.iter_mut().find(|doc| Some(doc.id) == active)
    }
}

/// Backing state for both the imm panel and its retained twin. One field per tab,
/// so the boxed tab bodies borrow disjoint parts of it.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct Inspector {
    pub tab: Tab,
    /// The panel's width, anchored in em so it holds through window resizes and
    /// tracks the font size. The splitter writes it as its divider is dragged,
    /// which is what makes the layout the app's to save rather than the
    /// widget's to remember.
    pub split: SplitSize,
    pub documents: Documents,
    pub material: Material,
    pub hierarchy: Hierarchy,
    /// Whether the viewport HUD shows; driven from the View menu.
    pub show_hud: bool,
    /// Autosave flag; driven from the View menu, only logged.
    pub autosave: bool,
    /// The Export modal: whether it is up, and what its select is set to.
    pub export_open: bool,
    pub export_cull: Cull,
}

impl Inspector {
    pub fn initial() -> Self {
        Self {
            split: SplitSize::new(em(30.0)),
            show_hud: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub base_color: Color,
    pub emissive: Color,
    pub blend: Blend,
    pub cull: Cull,
    pub metallic: f32,
    pub roughness: f32,
    /// Paint order, bottom first; the reorderable list's model.
    pub layers: Vec<Layer>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "Brick_Wall_01".into(),
            base_color: Color::srgb(0.62, 0.31, 0.24),
            emissive: Color::BLACK,
            blend: Blend::default(),
            cull: Cull::default(),
            metallic: 0.0,
            roughness: 0.72,
            layers: vec![
                Layer::new(1, "Base", lucide::LAYERS, 1.0),
                Layer::new(2, "Dirt", lucide::DROPLET, 0.6),
                Layer::new(3, "Moss", lucide::LEAF, 0.35),
                Layer::new(4, "Decal", lucide::STAMP, 0.8),
            ],
        }
    }
}

/// A layer's identity, whatever its position in the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayerId(pub u32);

/// One layer of the material: reordered by drag, faded by its slider.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub icon: Icon,
    pub opacity: f32,
    pub enabled: bool,
}

impl Layer {
    fn new(id: u32, name: &str, icon: Icon, opacity: f32) -> Self {
        Self {
            id: LayerId(id),
            name: name.into(),
            icon,
            opacity,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hierarchy {
    pub filter: String,
    pub nodes: Vec<SceneNode>,
}

impl Default for Hierarchy {
    fn default() -> Self {
        Self {
            filter: String::new(),
            nodes: vec![
                SceneNode::new("Dungeon", lucide::FOLDER_TREE, 0),
                SceneNode::new("Corridor_A", lucide::BOX, 1),
                SceneNode::new("Torch_01", lucide::FLAME, 2),
                SceneNode::new("Torch_02", lucide::FLAME, 2),
                SceneNode::new("Vault", lucide::BOX, 1),
                SceneNode::new("Chest", lucide::ARCHIVE, 2),
                SceneNode::new("Spawn_Point", lucide::MAP_PIN, 1),
            ],
        }
    }
}

/// The viewport HUD's own light theme, exercising per-subtree theming; both
/// inspector twins render their HUD with it.
pub fn hud_theme() -> ThemeId {
    ThemeId::new("Hud")
}

/// Startup system registering the HUD theme's palette.
pub fn register_hud_theme(mut theme: ResMut<UiTheme>) {
    theme.set_palette(
        hud_theme(),
        bevy_jp_plume::theme::palettes::default_light_palette(),
    );
}

/// Adds the inspector panel: its resource, its panel system, its HUD theme,
/// and its log.
pub struct InspectorPanelPlugin;

impl Plugin for InspectorPanelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Inspector::initial())
            .add_systems(Startup, register_hud_theme)
            .add_systems(Update, (export_shortcut, inspector_panel_ui).chain())
            .add_systems(Update, log_on_change::<Inspector>);
    }
}

// Ctrl+E, so the modal has an opener that is not a press: a press would dismiss
// an open popup on its way, which is the case the modal's own dismissal covers.
fn export_shortcut(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<Inspector>) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyE) {
        state.export_open = true;
    }
}

fn inspector_panel_ui(
    mut root: PlumeRoot,
    mut state: ResMut<Inspector>,
    mut rem_size: ResMut<RemSize>,
) {
    let mut s = state.clone();
    let mut rem = *rem_size;
    // Lifted out of the destructure below: the menu item that opens the modal
    // runs inside the screen, and the modal itself is built after it.
    let mut export_open = s.export_open;
    let mut viewport_entity = None;

    root.screen(|ui| {
        let Inspector {
            tab,
            split,
            documents,
            material,
            hierarchy,
            show_hud,
            autosave,
            ..
        } = &mut s;

        top_bar(ui, documents, show_hud, autosave, &mut export_open);

        ui.split_horizontal(
            split,
            |ui| {
                viewport_entity = document_pane(ui, documents);
            },
            |ui| panel(ui, tab, material, hierarchy, autosave, &mut rem),
        )
        // The panel is the sized pane, so the document pane alone absorbs
        // window resizes. It never gets narrower than its controls need; the
        // document pane gives down to a couple of squeezed tabs, past which its
        // strip scrolls. Dragging well past the panel's floor closes it entirely.
        .sized_pane(SplitPane::Second)
        .min_panes(Val::ZERO, em(28))
        .collapsible(false, true)
        .auto_hide_divider(true)
        .grow();
    })
    .background_slot(ThemeSlot::Neutral0);

    if s.show_hud
        && let Some(viewport_entity) = viewport_entity
    {
        viewport_hud(&mut root, &mut s.documents, viewport_entity);
    }

    export_modal(&mut root, &mut export_open, &mut s.export_cull);
    s.export_open = export_open;

    state.set_if_neq(s);
    rem_size.set_if_neq(rem);
}

// The File and View menus sharing a line with the document strip. The retained
// twin (`top_bar` there too) binds with `on()` where this takes `&mut`s.
fn top_bar(
    ui: &mut Ui,
    documents: &mut Documents,
    show_hud: &mut bool,
    autosave: &mut bool,
    export_open: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.menu_bar(|bar| {
            bar.menu("File", |menu| {
                if menu.item("New Document").shortcut("Ctrl+N").clicked {
                    documents.add();
                }
                if menu.item("Save").shortcut("Ctrl+S").clicked {
                    documents.save_active();
                }
                menu.separator();
                menu.submenu("Recent", |menu| {
                    for name in ["corridor_00.rs", "vault_01.wgsl", "torch_02.ron"] {
                        if menu.item(name).clicked {
                            info!("open recent {name}");
                        }
                    }
                });
                menu.separator();
                if menu.item("Export…").shortcut("Ctrl+E").clicked {
                    *export_open = true;
                }
                menu.separator();
                menu.item("Exit").enabled(false);
            });
            bar.menu("View", |menu| {
                menu.item_toggle("Show HUD", show_hud);
                menu.item_toggle("Autosave", autosave);
            });
        });
        ui.separator();
        document_strip(ui, documents);
    });
}

// The one viewport the open documents share, with the HUD floating in its
// corner; the strip that picks the document lives in `top_bar`.
fn document_pane(ui: &mut Ui, documents: &mut Documents) -> Option<Entity> {
    let mut viewport_entity = None;
    ui.vertical(|ui| {
        viewport_entity = Some(match documents.active_document() {
            Some(document) => viewport(ui, document),
            None => empty_viewport(ui),
        });
    })
    .gap(Val::ZERO)
    .grow();
    viewport_entity
}

fn document_strip(ui: &mut Ui, documents: &mut Documents) {
    // Every header is built each frame, so the ✕ reports through a shared cell
    // rather than a `&mut` each; both edits wait for the selection write-back.
    let closing = Cell::new(None);
    let opening = Cell::new(false);
    let mut active = documents.active;
    ui.horizontal(|ui| {
        ui.tabs(&mut active, |tabs| {
            for document in &documents.open {
                tabs.tab_container(Some(document.id), |ui| {
                    ui.icon(lucide::FILE_CODE).no_shrink();
                    // Its own clipping box, so a squeezed name is cut at its
                    // edge instead of running on under the ✕.
                    ui.horizontal(|ui| {
                        ui.caption(&document.name).no_wrap();
                    })
                    .clip();
                    ui.flex_spacer();
                    let hover = ui.hovered();
                    ui.icon(lucide::DOT)
                        .no_shrink()
                        .font_scale(2.0)
                        .text_color(Color::WHITE)
                        .displayed(document.dirty && !hover);
                    if ui
                        .tool_button(lucide::X)
                        .no_shrink()
                        .flat()
                        .font_scale(0.8)
                        .variant(ButtonVariant::Plain)
                        .displayed(hover)
                        .clicked
                    {
                        closing.set(Some(document.id));
                    }
                })
                // Wide enough that a squeezed tab keeps its ✕ reachable.
                .width(em(10))
                .min_width(em(6))
                // The viewport below the strip is shared, not per-tab.
                .no_body();
            }
        })
        .grow();
        if ui
            .tool_button(lucide::PLUS)
            .variant(ButtonVariant::Plain)
            .flat()
            .tooltip("Open a new document")
            .clicked
        {
            opening.set(true);
        }
    });
    documents.active = active;
    if let Some(id) = closing.get() {
        documents.close(id);
    }
    if opening.get() {
        documents.add();
    }
}

/// The viewport's colour for a document: a hue per id, so switching documents
/// shows in the pane itself and not only in the strip. Shared with the retained
/// twin, which has to write the same colour by hand.
pub fn viewport_color(id: DocId) -> Color {
    Color::hsv(
        StdRng::seed_from_u64(id.0 as u64).random_range(0.0..360.0),
        0.5,
        0.5,
    )
}

fn viewport(ui: &mut Ui, document: &Document) -> Entity {
    ui.vertical(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.icon(lucide::BOXES);
            ui.caption(&document.name).text_color_slot(ThemeSlot::Text1);
            ui.flex_spacer();
        });
        ui.flex_spacer();
    })
    .grow()
    .background(viewport_color(document.id))
    .entity
}

fn empty_viewport(ui: &mut Ui) -> Entity {
    ui.vertical(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.caption("No documents open")
                .text_color_slot(ThemeSlot::Text1);
            ui.flex_spacer();
        });
        ui.flex_spacer();
    })
    .grow()
    .background_slot(ThemeSlot::Neutral1)
    .entity
}

// Declared inside the pane, so its corner is the pane's and it follows the
// splitter with no anchoring to arrange.
fn viewport_hud(root: &mut PlumeRoot, documents: &mut Documents, viewport: Entity) {
    root.panel()
        .at_corner_of(viewport, Corner::BottomRight, em(1), em(1))
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.icon(lucide::BOXES);
                ui.caption(&format!("{} open", documents.open.len()));
                ui.separator();
                if ui
                    .tool_button(lucide::SAVE)
                    .flat()
                    .tooltip("Save the active document")
                    .clicked
                {
                    documents.save_active();
                }
            });
        })
        .theme(hud_theme());
}

fn panel(
    ui: &mut Ui,
    tab: &mut Tab,
    material: &mut Material,
    hierarchy: &mut Hierarchy,
    autosave: &mut bool,
    rem: &mut RemSize,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.tabs(tab, |tabs| {
                tabs.tab(Tab::Material, "Material")
                    .icon(lucide::PALETTE)
                    .no_body();
                tabs.tab(Tab::Hierarchy, "Hierarchy")
                    .icon(lucide::FOLDER_TREE)
                    .no_body();
            });

            ui.flex_spacer();
            ui.tool_button(lucide::SAVE).flat().tooltip("Save material");
            if ui
                .tool_button(lucide::UNDO_2)
                .flat()
                .tooltip("Revert to the last saved values")
                .clicked
            {
                (*material, *hierarchy) = Default::default();
            }
            // A drop-down off the app's own button rather than a menu bar's: the
            // button keeps its tool-button look and opens the same menu rows.
            ui.tool_button(lucide::ELLIPSIS_VERTICAL)
                .flat()
                .tooltip("More material actions")
                .menu(|menu| {
                    if menu.item("Copy Values").shortcut("Ctrl+C").clicked {
                        info!("copy material values");
                    }
                    menu.item("Paste Values").enabled(false);
                    menu.separator();
                    menu.item_toggle("Autosave", autosave);
                });
        })
        .padding(UiRect::right(size::SPACE));

        ui.vertical(|ui| {
            match tab {
                Tab::Material => {
                    material_tab(ui, material);
                }
                Tab::Hierarchy => {
                    hierarchy_tab(ui, hierarchy);
                }
            }

            ui.separator();
            footer(ui, rem);
        })
        .grow()
        .padding(size::SPACE)
        .background_slot(ThemeSlot::Neutral1);
    })
    .width(percent(100))
    .gap(Val::ZERO);
}

fn material_tab(ui: &mut Ui, s: &mut Material) {
    ui.scroll_area_vertical(|ui| material_fields(ui, s)).grow();
}

fn material_fields(ui: &mut Ui, s: &mut Material) {
    let defaults = Material::default();

    ui.section("Surface", |ui| {
        field(ui, "Name", |ui| {
            ui.color_swatch(s.base_color);
            ui.text_edit(&mut s.name)
                .grow()
                .placeholder("Material name");
        });
        if s.name.trim().is_empty() {
            ui.horizontal(|ui| {
                ui.icon(lucide::TRIANGLE_ALERT);
                ui.caption("A material needs a name").no_wrap();
            })
            .text_color(Color::srgb(0.90, 0.35, 0.32));
        }
        field(ui, "Base color", |ui| {
            ui.color_edit_rgb(&mut s.base_color);
            ui.caption(&hex(s.base_color)).grow();
        });
        field(ui, "Emissive", |ui| {
            ui.color_edit(&mut s.emissive);
            ui.caption(&hex(s.emissive)).grow();
        });
    });

    ui.section("Shading", |ui| {
        field(ui, "Blend", |ui| radios(ui, &mut s.blend));
        field(ui, "Cull", |ui| radios(ui, &mut s.cull));
        modified_row(ui, "Metallic", s.metallic != defaults.metallic, |ui| {
            ui.slider(&mut s.metallic, 0.0..=1.0).grow().precision(2);
        });
        modified_row(ui, "Roughness", s.roughness != defaults.roughness, |ui| {
            ui.slider(&mut s.roughness, 0.0..=1.0).grow().precision(2);
        });
    });

    // Drag a layer's grip to restack; the order is the model's, so Revert
    // restores it like any other field.
    ui.section("Layers", |ui| {
        ui.reorderable(
            &mut s.layers,
            |layer| layer.id,
            |ui, layer| {
                ui.icon(layer.icon);
                ui.caption(&layer.name).no_wrap().width(GUTTER * 0.6);
                ui.slider(&mut layer.opacity, 0.0..=1.0).grow().precision(2);
                ui.toggle(&mut layer.enabled);
            },
        );
    });

    ui.section("Color picker", |ui| {
        ui.color_picker(&mut s.base_color);
    })
    .start_collapsed();
}

fn hierarchy_tab(ui: &mut Ui, s: &mut Hierarchy) {
    ui.horizontal(|ui| {
        ui.icon(lucide::SEARCH);
        ui.text_edit(&mut s.filter).grow().placeholder("Filter…");
    });
    let filter = s.filter.to_lowercase();
    // A node parents the run of deeper rows that follows it.
    let parents: Vec<bool> = (0..s.nodes.len())
        .map(|i| {
            s.nodes
                .get(i + 1)
                .is_some_and(|next| next.depth > s.nodes[i].depth)
        })
        .collect();
    ui.scroll_area_vertical(|ui| {
        let mut collapsed_at: Option<usize> = None;
        for (i, parent) in parents.iter().copied().enumerate() {
            let (depth, expanded) = (s.nodes[i].depth, s.nodes[i].expanded);
            match collapsed_at {
                Some(collapsed) if depth > collapsed => continue,
                _ => collapsed_at = None,
            }
            if parent && !expanded {
                collapsed_at = Some(depth);
            }
            if !filter.is_empty() && !s.nodes[i].name.to_lowercase().contains(&filter) {
                continue;
            }
            ui.push_id(i, |ui| node_row(ui, &mut s.nodes[i], parent));
        }
    })
    .grow();
}

fn node_row(ui: &mut Ui, node: &mut SceneNode, parent: bool) {
    ui.horizontal(|ui| {
        ui.space(em(node.depth));
        ui.disclosure(&mut node.expanded).visible(parent);
        ui.icon(node.icon);
        ui.caption(&node.name).no_wrap();
        ui.flex_spacer();
        ui.toggle(&mut node.visible);
    });
}

fn footer(ui: &mut Ui, rem: &mut RemSize) {
    ui.horizontal(|ui| {
        ui.caption("Rem").width(GUTTER);
        ui.slider(&mut rem.0, 10.0..=20.0)
            .grow()
            .step(1.)
            .precision(0);
        ui.caption(&format!("{}", rem.0)).width(em(2));
    });
}

// The modal the File menu opens. It carries a select and tooltipped buttons on
// purpose: both open surfaces of their own, and a modal is the one place their
// layering has to be checked — the popup sits over the barrier, the tooltip over
// everything.
fn export_modal(root: &mut PlumeRoot, open: &mut bool, cull: &mut Cull) {
    let mut answered = false;
    root.modal("Export Scene", open).width(em(26.0)).show(|ui| {
        ui.caption("Nothing behind this takes a click until it is answered.")
            .text_color_slot(ThemeSlot::Text1);
        ui.horizontal(|ui| {
            ui.caption("Culling").width(em(6.0));
            ui.select(cull, |select| {
                for option in Cull::ALL {
                    select.option(*option, option.label());
                }
            })
            .grow();
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.tool_button(lucide::FOLDER_OPEN)
                .tooltip("Pick the output directory");
            ui.tool_button(lucide::UNDO_2)
                .tooltip("Reset these settings to their defaults");
            ui.flex_spacer();
            answered |= ui
                .button("Cancel")
                .variant(ButtonVariant::Outline)
                .tooltip("Close without writing anything")
                .clicked;
            answered |= ui
                .button("Export")
                .primary()
                .tooltip("Write the scene with the settings above")
                .clicked;
        });
    });
    if answered {
        *open = false;
    }
}

fn field(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.caption(label).width(GUTTER * 0.6);
        f(ui);
    });
}

fn modified_row(ui: &mut Ui, label: &str, modified: bool, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let caption = ui.caption(label).width(GUTTER);
        if modified {
            caption.text_color_slot(ThemeSlot::Accent1);
        }
        f(ui);
    });
}

fn radios<T: Options>(ui: &mut Ui, selected: &mut T) {
    for &option in T::ALL {
        ui.radio(selected, option, option.label());
    }
}

fn hex(color: Color) -> String {
    let c = color.to_srgba();
    format!(
        "#{:02X}{:02X}{:02X}",
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8
    )
}
