//! Editor inspector as a self-contained feature plugin: a docked side panel whose
//! two tabs hold a material editor and a scrolling scene tree, beside a document strip.
use std::cell::Cell;

use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::{Options, log_on_change};

/// Base font size the panel scales from.
pub const BASE_FONT_PX: f32 = 14.0;

const GUTTER: f32 = 78.0;

/// Cap on open documents; the retained twin spawns a fixed tab pool this size.
pub const MAX_DOCUMENTS: usize = 8;

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
    pub icon: FaIcon,
    pub depth: usize,
    pub expanded: bool,
    pub visible: bool,
}

impl SceneNode {
    fn new(name: &str, icon: FaIcon, depth: usize) -> Self {
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
        for _ in 0..3 {
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
    pub ui_scale: f32,
    /// The viewport's share of the width; the panel takes the rest. The
    /// splitter writes it as its divider is dragged, which is what makes the
    /// layout the app's to save rather than the widget's to remember.
    pub split: f32,
    pub documents: Documents,
    pub material: Material,
    pub hierarchy: Hierarchy,
    /// Whether the viewport HUD shows; driven from the View menu.
    pub show_hud: bool,
    /// Autosave flag; driven from the View menu, only logged.
    pub autosave: bool,
}

impl Inspector {
    pub fn initial() -> Self {
        Self {
            ui_scale: 1.0,
            split: 0.75,
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
        use font_awesome::solid as fa;
        Self {
            filter: String::new(),
            nodes: vec![
                SceneNode::new("Dungeon", fa::SITEMAP, 0),
                SceneNode::new("Corridor_A", fa::CUBE, 1),
                SceneNode::new("Torch_01", fa::FIRE, 2),
                SceneNode::new("Torch_02", fa::FIRE, 2),
                SceneNode::new("Vault", fa::CUBE, 1),
                SceneNode::new("Chest", fa::BOX_ARCHIVE, 2),
                SceneNode::new("Spawn_Point", fa::LOCATION_DOT, 1),
            ],
        }
    }
}

/// Adds the inspector panel: its resource, its panel system, and its log.
pub struct InspectorPanelPlugin;

impl Plugin for InspectorPanelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Inspector::initial())
            .add_systems(Update, inspector_panel_ui)
            .add_systems(Update, log_on_change::<Inspector>);
    }
}

fn inspector_panel_ui(mut root: PlumeRoot, mut state: ResMut<Inspector>) {
    let mut s = state.clone();
    root.screen(|ui| {
        menu_bar(ui, &mut s);
        // Same reason as `tab` below: neither the split nor the documents can stay
        // borrowed from `s` while the other pane's bodies edit the rest of it.
        let mut split = s.split;
        let mut documents = s.documents.clone();
        let show_hud = s.show_hud;
        ui.split_horizontal(
            &mut split,
            |ui| document_pane(ui, &mut documents, show_hud),
            |ui| panel(ui, &mut s),
        )
        // The panel never gets narrower than its controls need; the document pane
        // gives down to a couple of squeezed tabs, past which its strip scrolls.
        // Dragging well past the panel's floor closes it entirely.
        .min_panes(px(120), px(260))
        .collapsible(false, true)
        .align_items(AlignItems::Stretch)
        .auto_hide_divider(true)
        .grow();
        s.split = split;
        s.documents = documents;
    })
    .background_slot(ThemeSlot::Neutral0)
    .font_size(BASE_FONT_PX * s.ui_scale);
    state.set_if_neq(s);
}

// The imm twin of the retained example's `menu_bar`: the same File and View
// menus over the same `Inspector` state, bound with `&mut`s instead of `on()`.
fn menu_bar(ui: &mut Ui, s: &mut Inspector) {
    ui.menu_bar(|bar| {
        bar.menu("File", |menu| {
            if menu.item("New Document").shortcut("Ctrl+N").clicked {
                s.documents.add();
            }
            if menu.item("Save").shortcut("Ctrl+S").clicked {
                s.documents.save_active();
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
            menu.item("Exit").enabled(false);
        });
        bar.menu("View", |menu| {
            menu.item_toggle("Show HUD", &mut s.show_hud);
            menu.item_toggle("Autosave", &mut s.autosave);
        });
    });
}

// The strip of open documents over the one viewport they share, with the HUD
// floating in its corner.
fn document_pane(ui: &mut Ui, documents: &mut Documents, show_hud: bool) {
    ui.vertical(|ui| {
        document_strip(ui, documents);
        match documents.active_document() {
            Some(document) => viewport(ui, document),
            None => empty_viewport(ui),
        }
        if show_hud {
            viewport_hud(ui, documents);
        }
    })
    .gap(Val::ZERO)
    .grow();
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
                    ui.icon(font_awesome::solid::FILE_CODE).no_shrink();
                    // Its own clipping box, so a squeezed name is cut at its
                    // edge instead of running on under the ✕.
                    ui.horizontal(|ui| {
                        ui.caption(&document.name).no_wrap();
                    })
                    .clip();
                    if document.dirty {
                        ui.icon(font_awesome::solid::CIRCLE)
                            .no_shrink()
                            .font_scale(0.5)
                            .text_color(Color::WHITE);
                    }
                    if ui
                        .tool_button(font_awesome::solid::XMARK)
                        .no_shrink()
                        .flat()
                        .variant(ButtonVariant::Plain)
                        .clicked
                    {
                        closing.set(Some(document.id));
                    }
                })
                // Wide enough that a squeezed tab keeps its ✕ reachable.
                .min_width(em(6))
                // The viewport below the strip is shared, not per-tab.
                .no_body();
            }
        })
        // The strip sits over the viewport, not over a surface.
        .inverted()
        .grow();
        if ui
            .tool_button(font_awesome::solid::PLUS)
            .variant(ButtonVariant::Plain)
            .flat()
            .tooltip("Open a new document")
            .clicked
        {
            opening.set(true);
        }
    })
    .gap(size::GAP_TIGHT)
    .padding(UiRect::right(size::GAP_TIGHT))
    // Matches the inverted strip, so the ✚ shares its band.
    .background_slot(ThemeSlot::Neutral1);
    documents.active = active;
    if let Some(id) = closing.get() {
        documents.close(id);
    }
    if opening.get() {
        documents.add();
    }
}

fn viewport(ui: &mut Ui, document: &Document) {
    ui.vertical(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.icon(font_awesome::solid::CUBES);
            ui.caption(&document.name).text_color_slot(ThemeSlot::Text1);
            ui.flex_spacer();
        });
        ui.flex_spacer();
    })
    .grow();
}

fn empty_viewport(ui: &mut Ui) {
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
    .background_slot(ThemeSlot::Neutral3);
}

// Declared inside the pane, so its corner is the pane's and it follows the
// splitter with no anchoring to arrange.
fn viewport_hud(ui: &mut Ui, documents: &mut Documents) {
    ui.panel()
        .at_corner(Corner::BottomRight, px(16), px(16))
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.icon(font_awesome::solid::CUBES);
                ui.caption(&format!("{} open", documents.open.len()));
                ui.separator();
                if ui
                    .tool_button(font_awesome::solid::FLOPPY_DISK)
                    .flat()
                    .tooltip("Save the active document")
                    .clicked
                {
                    documents.save_active();
                }
            });
        });
}

fn panel(ui: &mut Ui, s: &mut Inspector) {
    ui.vertical(|ui| {
        header(ui, s);
        ui.separator();
        // The tab key can't stay borrowed from `s` while the bodies edit the rest of it.
        let mut tab = s.tab;
        ui.tabs(&mut tab, |tabs| {
            let (material, hierarchy) = (&mut s.material, &mut s.hierarchy);
            tabs.tab(Tab::Material, "Material")
                .icon(font_awesome::solid::PALETTE)
                .body(|ui| material_tab(ui, material));
            tabs.tab(Tab::Hierarchy, "Hierarchy")
                .icon(font_awesome::solid::SITEMAP)
                .body(|ui| hierarchy_tab(ui, hierarchy));
        })
        .grow();
        s.tab = tab;
        ui.separator();
        footer(ui, s);
    })
    // Width and minimum belong to the splitter now; the panel fills its pane,
    // and the seam's line is the splitter's divider rather than a panel border.
    .grow()
    .background_slot(ThemeSlot::Neutral1)
    .padding(size::PAD)
    .font_size(BASE_FONT_PX * s.ui_scale);
}

fn header(ui: &mut Ui, s: &mut Inspector) {
    ui.horizontal(|ui| {
        ui.icon(font_awesome::solid::SLIDERS);
        ui.caption("Inspector")
            .small_caps()
            .text_color_slot(ThemeSlot::Text0);
        ui.flex_spacer();
        ui.tool_button(font_awesome::solid::FLOPPY_DISK)
            .flat()
            .tooltip("Save material");
        if ui
            .tool_button(font_awesome::solid::ARROW_ROTATE_LEFT)
            .flat()
            .tooltip("Revert to the last saved values")
            .clicked
        {
            (s.material, s.hierarchy) = Default::default();
        }
    });
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
                ui.icon(font_awesome::solid::TRIANGLE_EXCLAMATION);
                ui.caption("A material needs a name").no_wrap();
            })
            .text_color(Color::srgb(0.90, 0.35, 0.32));
        }
        field(ui, "Base color", |ui| {
            ui.color_edit(&mut s.base_color);
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

    ui.section("Color picker", |ui| {
        ui.color_picker(&mut s.base_color);
    })
    .start_collapsed();
}

fn hierarchy_tab(ui: &mut Ui, s: &mut Hierarchy) {
    ui.horizontal(|ui| {
        ui.icon(font_awesome::solid::MAGNIFYING_GLASS);
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
        ui.space(px(node.depth as f32 * 14.0));
        if parent {
            ui.disclosure(&mut node.expanded);
        } else {
            ui.space(size::ROW_HEIGHT);
        }
        ui.icon(node.icon);
        ui.caption(&node.name).no_wrap();
        ui.flex_spacer();
        ui.toggle(&mut node.visible);
    });
}

fn footer(ui: &mut Ui, s: &mut Inspector) {
    ui.horizontal(|ui| {
        ui.caption("UI scale").width(px(GUTTER));
        ui.slider(&mut s.ui_scale, 0.5..=2.0)
            .grow()
            .step(0.05)
            .precision(2);
        ui.caption(&format!("{:.0}%", s.ui_scale * 100.0))
            .width(px(44));
    });
}

fn field(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(GUTTER));
        f(ui);
    });
}

fn modified_row(ui: &mut Ui, label: &str, modified: bool, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let caption = ui.caption(label).width(px(GUTTER));
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
