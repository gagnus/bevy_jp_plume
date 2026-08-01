//! Editor inspector as a self-contained feature plugin: a docked side panel whose
//! two tabs hold a material editor and a scrolling scene tree.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::{Options, log_on_change};

/// Base font size the panel scales from.
pub const BASE_FONT_PX: f32 = 14.0;

const GUTTER: f32 = 78.0;

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

/// Backing state for both the imm panel and its retained twin. One field per tab,
/// so the boxed tab bodies borrow disjoint parts of it.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct Inspector {
    pub tab: Tab,
    pub ui_scale: f32,
    pub material: Material,
    pub hierarchy: Hierarchy,
}

impl Inspector {
    pub fn initial() -> Self {
        Self {
            ui_scale: 1.0,
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
        ui.horizontal(|ui| {
            viewport(ui);
            panel(ui, &mut s);
        })
        .align_items(AlignItems::Stretch)
        .grow();
    })
    .background_slot(ThemeSlot::Neutral0);
    state.set_if_neq(s);
}

fn viewport(ui: &mut Ui) {
    ui.vertical(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.icon(font_awesome::solid::CUBES);
            ui.caption("Viewport").text_color_slot(ThemeSlot::Text1);
            ui.flex_spacer();
        });
        ui.flex_spacer();
    })
    .grow();
}

fn panel(ui: &mut Ui, s: &mut Inspector) {
    ui.vertical(|ui| {
        header(ui, s);
        ui.separator();
        // The tab key can't stay borrowed from `s` while the bodies edit the rest of it.
        let mut tab = s.tab;
        ui.tabs(&mut tab, |tabs| {
            let (material, hierarchy) = (&mut s.material, &mut s.hierarchy);
            tabs.tab(Tab::Material, "Material", |ui| material_tab(ui, material))
                .icon(font_awesome::solid::PALETTE);
            tabs.tab(Tab::Hierarchy, "Hierarchy", |ui| {
                hierarchy_tab(ui, hierarchy)
            })
            .icon(font_awesome::solid::SITEMAP);
        })
        .grow();
        s.tab = tab;
        ui.separator();
        footer(ui, s);
    })
    .width(percent(25))
    .min_width(px(260))
    .background_slot(ThemeSlot::Neutral1)
    .border_slot(UiRect::left(px(1)), ThemeSlot::Neutral3)
    .padding(size::PAD)
    .font_size(FontSize::Px(BASE_FONT_PX * s.ui_scale));
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
    ui.scroll_area(|ui| material_fields(ui, s)).grow();
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

    ui.section("Colour picker", |ui| {
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
    ui.scroll_area(|ui| {
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
