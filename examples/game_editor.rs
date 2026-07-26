//! A full-screen "mock" game editor built entirely from plume primitives.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    constants::{font_awesome::solid as fa, size},
    controls::ButtonVariant,
    imm::{PlumeImm, PlumeRoot, Ui},
    theme::{UiTheme, slots::ThemeSlot},
    tokens,
};

#[path = "common/mod.rs"]
mod common;

use common::{debug_hub::DebugDialogRegistry, theme_editor::ThemeEditorPlugin};

/// The active manipulation tool, shown pressed-in on the tool bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Tool {
    #[default]
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum LeftTab {
    #[default]
    Scene,
    Prefabs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum RightTab {
    #[default]
    Properties,
    Add,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum BottomTab {
    #[default]
    Output,
    Assets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshKind {
    Cube,
    Sphere,
    Capsule,
    Plane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColliderKind {
    None,
    Box,
    Sphere,
    Trimesh,
}

/// One entity in the scene tree: a display name, a type icon, its visibility
/// toggle and fold state, and its children.
#[derive(Clone, PartialEq)]
struct SceneNode {
    id: u32,
    label: String,
    icon: bevy_jp_plume::constants::FaIcon,
    visible: bool,
    expanded: bool,
    children: Vec<SceneNode>,
}

fn leaf(id: u32, label: &str, icon: bevy_jp_plume::constants::FaIcon, visible: bool) -> SceneNode {
    SceneNode {
        id,
        label: label.to_owned(),
        icon,
        visible,
        expanded: false,
        children: Vec::new(),
    }
}

fn branch(
    id: u32,
    label: &str,
    icon: bevy_jp_plume::constants::FaIcon,
    expanded: bool,
    children: Vec<SceneNode>,
) -> SceneNode {
    SceneNode {
        id,
        label: label.to_owned(),
        icon,
        visible: true,
        expanded,
        children,
    }
}

/// All the state the mock editor's controls read and write — a stand-in for the
/// real ECS world the panels would inspect.
#[derive(Resource, Clone, PartialEq)]
struct Editor {
    tool: Tool,
    snap_to_grid: bool,
    show_grid: bool,
    playing: bool,

    left_tab: LeftTab,
    right_tab: RightTab,
    bottom_tab: BottomTab,

    tree: Vec<SceneNode>,
    selected: u32,

    // Inspector fields for the "selected" entity.
    name: String,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    mesh: MeshKind,
    cast_shadows: bool,
    base_color: Color,
    metallic: f32,
    roughness: f32,
    collider: ColliderKind,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tool: Tool::default(),
            snap_to_grid: true,
            show_grid: true,
            playing: false,
            left_tab: LeftTab::default(),
            right_tab: RightTab::default(),
            bottom_tab: BottomTab::default(),
            tree: sample_tree(),
            selected: 4,
            name: "Gravekeeper".to_owned(),
            position: [12.0, 0.0, -8.5],
            rotation: [0.0, 45.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            mesh: MeshKind::Capsule,
            cast_shadows: true,
            base_color: Color::srgb(0.55, 0.13, 0.16),
            metallic: 0.1,
            roughness: 0.7,
            collider: ColliderKind::Box,
        }
    }
}

fn sample_tree() -> Vec<SceneNode> {
    vec![
        branch(
            1,
            "The Sunless Keep",
            fa::DUNGEON,
            true,
            vec![
                branch(
                    2,
                    "Entrance Hall",
                    fa::ARCHWAY,
                    true,
                    vec![
                        leaf(3, "Torch Sconce", fa::LIGHTBULB, true),
                        leaf(4, "Oak Door", fa::DOOR_CLOSED, true),
                        leaf(5, "Cobweb", fa::CUBE, false),
                    ],
                ),
                branch(
                    6,
                    "Crypt Level",
                    fa::LAYER_GROUP,
                    false,
                    vec![
                        leaf(7, "Sarcophagus", fa::CUBE, true),
                        leaf(8, "Cursed Altar", fa::CUBE, true),
                    ],
                ),
            ],
        ),
        branch(
            9,
            "Encounters",
            fa::DRAGON,
            true,
            vec![
                leaf(10, "Wandering Wraith", fa::GHOST, true),
                leaf(11, "Gravekeeper", fa::SKULL, true),
            ],
        ),
        branch(
            12,
            "Lighting",
            fa::LIGHTBULB,
            false,
            vec![
                leaf(13, "Sun", fa::LIGHTBULB, true),
                leaf(14, "Torch Flicker", fa::LIGHTBULB, true),
            ],
        ),
    ]
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<Editor>()
        // Mount the theme editor's feature plugin (registry + dialog + live rebake) but
        // not the debug hub's floating launcher — the toolbar's palette button is our
        // launcher. Starts closed.
        .add_plugins(ThemeEditorPlugin(false))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, editor_ui);
    common::apply_args(&mut app, false);
    app.run();
}

/// Title the theme-editor feature registered itself under; the toolbar's palette
/// button flips this registry entry to open/close the dialog.
const THEME_EDITOR_TITLE: &str = "Theme Editor";

fn editor_ui(
    mut root: PlumeRoot,
    mut editor: ResMut<Editor>,
    theme: Res<UiTheme>,
    mut registry: ResMut<DebugDialogRegistry>,
) {
    let mut state = editor.clone();
    let mut theme_editor_open = registry.is_open(THEME_EDITOR_TITLE);
    root.screen(|ui| {
        toolbar(
            ui,
            &mut state,
            theme.palette[ThemeSlot::Neutral1],
            &mut theme_editor_open,
        );
        // The split fills all the height the toolbar leaves; its children stretch
        // to that full height rather than centering (a row's default).
        ui.horizontal(|ui| {
            left_panel(ui, &mut state);
            center_panel(ui, &mut state);
            right_panel(ui, &mut state);
        })
        .grow()
        .align_items(AlignItems::Stretch);
    })
    // `screen` is transparent by design (it floats over a 3D scene); this editor has
    // nothing behind it, so fill it with the theme's window background instead of
    // letting the bare camera clear color show through the gaps between panels.
    .background(theme.color(&tokens::WINDOW_BG))
    .pad(UiRect::ZERO);
    editor.set_if_neq(state);
    if theme_editor_open != registry.is_open(THEME_EDITOR_TITLE) {
        registry.set_open(THEME_EDITOR_TITLE, theme_editor_open);
    }
}

fn toolbar(ui: &mut Ui, state: &mut Editor, bg: Color, theme_editor_open: &mut bool) {
    ui.horizontal(|ui| {
        // File.
        ui.tool_button(fa::FILE);
        ui.tool_button(fa::FOLDER_OPEN);
        ui.tool_button(fa::FLOPPY_DISK);
        ui.separator();

        // Undo / redo / clipboard.
        ui.tool_button(fa::ARROW_ROTATE_LEFT);
        ui.tool_button(fa::ARROW_ROTATE_RIGHT);
        ui.tool_button(fa::PASTE);
        ui.tool_button(fa::TRASH);
        ui.separator();

        // Manipulation tools — a radio group of pressed-in icon buttons.
        for (tool, icon) in [
            (Tool::Select, fa::ARROW_POINTER),
            (Tool::Move, fa::ARROWS_UP_DOWN_LEFT_RIGHT),
            (Tool::Rotate, fa::ROTATE),
            (Tool::Scale, fa::UP_RIGHT_AND_DOWN_LEFT_FROM_CENTER),
        ] {
            if ui
                .tool_button(icon)
                .checkable()
                .flat()
                .checked(state.tool == tool)
                .clicked
            {
                state.tool = tool;
            }
        }
        ui.separator();

        // Snapping toggles.
        if ui
            .tool_button(fa::MAGNET)
            .checkable()
            .flat()
            .checked(state.snap_to_grid)
            .clicked
        {
            state.snap_to_grid = !state.snap_to_grid;
        }
        if ui
            .tool_button(fa::BORDER_ALL)
            .checkable()
            .flat()
            .checked(state.show_grid)
            .clicked
        {
            state.show_grid = !state.show_grid;
        }

        // Playback lives at the trailing edge.
        ui.flex_spacer();
        if ui
            .tool_button(fa::PLAY)
            .checkable()
            .flat()
            .checked(state.playing)
            .clicked
        {
            state.playing = true;
        }
        ui.tool_button(fa::PAUSE);
        if ui.tool_button(fa::STOP).clicked {
            state.playing = false;
        }

        // Cheeky: pop plume's own theme editor so the whole UI can be repainted live.
        ui.separator();
        if ui
            .tool_button(fa::PALETTE)
            .checkable()
            .checked(*theme_editor_open)
            .clicked
        {
            *theme_editor_open = !*theme_editor_open;
        }
    })
    .background(bg)
    .pad(size::GAP / 2.0);
}

fn left_panel(ui: &mut Ui, state: &mut Editor) {
    ui.tabs(&mut state.left_tab, |tabs| {
        tabs.tab(LeftTab::Scene, "Scene", |ui| {
            for (i, node) in state.tree.iter_mut().enumerate() {
                ui.push_id(i, |ui| tree_row(ui, node, &mut state.selected, 0));
            }
        })
        .icon(fa::SITEMAP);
        tabs.tab(LeftTab::Prefabs, "Prefabs", |ui| {
            ui.caption("Prefab palette — drag a prefab into the scene.");
        })
        .icon(fa::CUBES);
    })
    .width(px(280));
}

/// One tree row, then its children when the row is expanded. `selected` is shared
/// down the recursion so any row can become the selection.
fn tree_row(ui: &mut Ui, node: &mut SceneNode, selected: &mut u32, depth: usize) {
    let has_children = !node.children.is_empty();
    let is_selected = *selected == node.id;

    // The whole row is one clickable surface, so the selection fill spans it edge to
    // edge and the content left-aligns. The twisty and eye inside keep their own
    // clicks — Button/Checkbox swallow the press before it can reach the row.
    let clicked = ui
        .button_container(|ui| {
            if depth > 0 {
                ui.space(size::TEXT_HEIGHT * (depth as f32));
            }
            // A twisty on branches; leaves reserve its width so labels line up.
            if has_children {
                ui.disclosure(&mut node.expanded);
            } else {
                ui.space(size::ROW_HEIGHT);
            }
            ui.icon(node.icon);
            ui.caption(&node.label);
            ui.flex_spacer();
            if ui
                .tool_button(if node.visible { fa::EYE } else { fa::EYE_SLASH })
                .variant(ButtonVariant::Plain)
                .clicked
            {
                node.visible = !node.visible;
            }
        })
        .variant(if is_selected {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Plain
        })
        .flat()
        .clicked;
    if clicked {
        *selected = node.id;
    }

    if has_children && node.expanded {
        for (i, child) in node.children.iter_mut().enumerate() {
            ui.push_id(i, |ui| tree_row(ui, child, selected, depth + 1));
        }
    }
}

fn center_panel(ui: &mut Ui, state: &mut Editor) {
    // Grows to take the width the fixed side panels leave; stacks the viewport
    // over the bottom dock.
    ui.vertical(|ui| {
        // The 3D viewport: just a black fill that eats the remaining height.
        ui.vertical(|_| {}).grow().background(Color::BLACK);
        bottom_dock(ui, state);
    })
    .grow();
}

fn bottom_dock(ui: &mut Ui, state: &mut Editor) {
    ui.tabs(&mut state.bottom_tab, |tabs| {
        tabs.tab(BottomTab::Output, "Output", |ui| {
            // Stand-in log lines so the console reads as populated.
            for line in [
                "[info] Loaded dungeon seed 0xC0FFEE (42 rooms)",
                "[info] Baked navmesh in 18.4 ms",
                "[warn] Cobweb has no collider — skipped",
                "[info] Play mode ready",
            ] {
                ui.caption(line);
            }
        })
        .icon(fa::LIST);
        tabs.tab(BottomTab::Assets, "Assets", |ui| {
            // No asset-grid control yet — describe what would live here.
            ui.caption("Asset browser — imported meshes, textures and materials appear here.");
        })
        .icon(fa::FOLDER_OPEN);
    })
    .height(px(180));
}

fn right_panel(ui: &mut Ui, state: &mut Editor) {
    // A local copy of the selection so the Properties body can borrow the rest of
    // `state` (the whole inspector) without colliding with the tab's `&mut`.
    let mut right_tab = state.right_tab;
    ui.tabs(&mut right_tab, |tabs| {
        tabs.tab(RightTab::Properties, "Properties", |ui| {
            inspector(ui, state);
        })
        .icon(fa::SLIDERS);
        tabs.tab(RightTab::Add, "Add", |ui| {
            ui.caption("Component browser — pick a component to add to the entity.");
        })
        .icon(fa::PLUS);
    })
    .width(px(320));
    state.right_tab = right_tab;
}

fn inspector(ui: &mut Ui, state: &mut Editor) {
    // Entity header: name field and enabled toggle.
    ui.horizontal(|ui| {
        ui.caption("Name");
        ui.text_edit(&mut state.name).grow();
    });

    ui.section("Transform", |ui| {
        vec3_row(ui, "Position", &mut state.position);
        vec3_row(ui, "Rotation", &mut state.rotation);
        vec3_row(ui, "Scale", &mut state.scale);
    });

    ui.section("Mesh", |ui| {
        ui.horizontal(|ui| {
            ui.caption("Source");
            ui.select(&mut state.mesh, |select| {
                select.option(MeshKind::Cube, "Cube");
                select.option(MeshKind::Sphere, "Sphere");
                select.option(MeshKind::Capsule, "Capsule");
                select.option(MeshKind::Plane, "Plane");
            })
            .grow();
        });
        ui.checkbox(&mut state.cast_shadows, "Cast shadows");
    });

    ui.section("Material", |ui| {
        ui.horizontal(|ui| {
            ui.caption("Base color");
            ui.flex_spacer();
            ui.color_swatch(state.base_color);
        });
        ui.horizontal(|ui| {
            ui.caption("Metallic");
            ui.slider(&mut state.metallic, 0.0..=1.0).grow();
        });
        ui.horizontal(|ui| {
            ui.caption("Roughness");
            ui.slider(&mut state.roughness, 0.0..=1.0).grow();
        });
    });

    ui.section("Collider", |ui| {
        ui.horizontal(|ui| {
            ui.caption("Shape");
            ui.select(&mut state.collider, |select| {
                select.option(ColliderKind::None, "None");
                select.option(ColliderKind::Box, "Box");
                select.option(ColliderKind::Sphere, "Sphere");
                select
                    .option(ColliderKind::Trimesh, "Trimesh")
                    .enabled(false);
            })
            .grow();
        });
    });
}

/// A labeled row of three number inputs (X/Y/Z), the inspector's workhorse.
fn vec3_row(ui: &mut Ui, label: &str, value: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(64.0));
        for (axis, component) in ["X", "Y", "Z"].into_iter().zip(value.iter_mut()) {
            ui.push_id(axis, |ui| {
                ui.number(component).precision(2).grow();
            });
        }
    });
}
