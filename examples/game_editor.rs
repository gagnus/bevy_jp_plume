//! A full-screen "mock" game editor built entirely from plume primitives.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

#[path = "common/mod.rs"]
mod common;

use common::debug_hub::DebugDialogRegistry;
use common::theme_editor::ThemeEditorPlugin;

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

#[derive(Clone, PartialEq)]
struct SceneNode {
    id: u32,
    label: String,
    icon: Icon,
    visible: bool,
    expanded: bool,
    children: Vec<SceneNode>,
}

fn leaf(id: u32, label: &str, icon: Icon, visible: bool) -> SceneNode {
    SceneNode {
        id,
        label: label.to_owned(),
        icon,
        visible,
        expanded: false,
        children: Vec::new(),
    }
}

fn branch(id: u32, label: &str, icon: Icon, expanded: bool, children: Vec<SceneNode>) -> SceneNode {
    SceneNode {
        id,
        label: label.to_owned(),
        icon,
        visible: true,
        expanded,
        children,
    }
}

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
            lucide::CASTLE,
            true,
            vec![
                branch(
                    2,
                    "Entrance Hall",
                    lucide::LANDMARK,
                    true,
                    vec![
                        leaf(3, "Torch Sconce", lucide::LIGHTBULB, true),
                        leaf(4, "Oak Door", lucide::DOOR_CLOSED, true),
                        leaf(5, "Cobweb", lucide::BOX, false),
                    ],
                ),
                branch(
                    6,
                    "Crypt Level",
                    lucide::LAYERS,
                    false,
                    vec![
                        leaf(7, "Sarcophagus", lucide::BOX, true),
                        leaf(8, "Cursed Altar", lucide::BOX, true),
                    ],
                ),
            ],
        ),
        branch(
            9,
            "Encounters",
            lucide::SWORDS,
            true,
            vec![
                leaf(10, "Wandering Wraith", lucide::GHOST, true),
                leaf(11, "Gravekeeper", lucide::SKULL, true),
            ],
        ),
        branch(
            12,
            "Lighting",
            lucide::LIGHTBULB,
            false,
            vec![
                leaf(13, "Sun", lucide::LIGHTBULB, true),
                leaf(14, "Torch Flicker", lucide::LIGHTBULB, true),
            ],
        ),
    ]
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<Editor>()
        // The theme editor without the debug hub's floating launcher: the
        // toolbar's palette button is this example's launcher.
        .add_plugins(ThemeEditorPlugin(false))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, editor_ui);
    common::apply_args(&mut app, false);
    app.run();
}

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
            theme.slot_color(None, ThemeSlot::Neutral1),
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
        .gap(px(1))
        .align_items(AlignItems::Stretch);
    })
    .background_slot(ThemeSlot::Neutral0);
    editor.set_if_neq(state);
    if theme_editor_open != registry.is_open(THEME_EDITOR_TITLE) {
        registry.set_open(THEME_EDITOR_TITLE, theme_editor_open);
    }
}

fn toolbar(ui: &mut Ui, state: &mut Editor, bg: Color, theme_editor_open: &mut bool) {
    ui.horizontal(|ui| {
        ui.tool_button(lucide::FILE);
        ui.tool_button(lucide::FOLDER_OPEN);
        ui.tool_button(lucide::SAVE);
        ui.separator();

        ui.tool_button(lucide::UNDO_2);
        ui.tool_button(lucide::REDO_2);
        ui.tool_button(lucide::CLIPBOARD_PASTE);
        ui.tool_button(lucide::TRASH_2);
        ui.separator();

        for (tool, icon) in [
            (Tool::Select, lucide::MOUSE_POINTER_2),
            (Tool::Move, lucide::MOVE),
            (Tool::Rotate, lucide::ROTATE_CW),
            (Tool::Scale, lucide::SCALING),
        ] {
            let mut selected = state.tool == tool;
            if ui.tool_button(icon).checkable(&mut selected).flat().clicked {
                state.tool = tool;
            }
        }
        ui.separator();

        ui.tool_button(lucide::MAGNET)
            .checkable(&mut state.snap_to_grid)
            .flat();
        ui.tool_button(lucide::GRID_3X3)
            .checkable(&mut state.show_grid)
            .flat();

        ui.flex_spacer();
        if ui
            .tool_button(lucide::PLAY)
            .checkable(&mut state.playing)
            .flat()
            .clicked
        {
            state.playing = true;
        }
        ui.tool_button(lucide::PAUSE);
        if ui.tool_button(lucide::SQUARE).clicked {
            state.playing = false;
        }

        ui.separator();
        ui.tool_button(lucide::PALETTE).checkable(theme_editor_open);
    })
    .background(bg)
    .padding(size::SPACE_TIGHT);
}

fn left_panel(ui: &mut Ui, state: &mut Editor) {
    ui.tabs(&mut state.left_tab, |tabs| {
        tabs.tab(LeftTab::Scene, "Scene")
            .icon(lucide::FOLDER_TREE)
            .body(|ui| {
                for (i, node) in state.tree.iter_mut().enumerate() {
                    ui.push_id(i, |ui| tree_row(ui, node, &mut state.selected, 0));
                }
            });
        tabs.tab(LeftTab::Prefabs, "Prefabs")
            .icon(lucide::BOXES)
            .body(|ui| {
                ui.caption("Prefab palette — drag a prefab into the scene.");
            });
    })
    .width(px(280));
}

fn tree_row(ui: &mut Ui, node: &mut SceneNode, selected: &mut u32, depth: usize) {
    let has_children = !node.children.is_empty();
    let is_selected = *selected == node.id;

    // One clickable surface, so the selection fill spans the row edge to edge; the
    // twisty and eye keep their own clicks, swallowing the press before it arrives.
    let clicked = ui
        .button_container(|ui| {
            if depth > 0 {
                ui.space(em(depth));
            }
            ui.disclosure(&mut node.expanded).visible(has_children);
            ui.icon(node.icon);
            ui.caption(&node.label);
            ui.flex_spacer();
            // Shown on row hover, and whenever it is hiding something, so a hidden
            // node still reads as hidden with the pointer elsewhere.
            let show_eye = ui.hovered() || !node.visible;
            if ui
                .tool_button(if node.visible {
                    lucide::EYE
                } else {
                    lucide::EYE_OFF
                })
                .variant(ButtonVariant::Plain)
                .inert()
                .visible(show_eye)
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
    ui.vertical(|ui| {
        ui.vertical(|_| {}).grow().background(Color::BLACK);
        bottom_dock(ui, state);
    })
    .gap(px(1))
    .grow();
}

fn bottom_dock(ui: &mut Ui, state: &mut Editor) {
    ui.tabs(&mut state.bottom_tab, |tabs| {
        tabs.tab(BottomTab::Output, "Output")
            .icon(lucide::LIST)
            .body(|ui| {
                for line in [
                    "[info] Loaded dungeon seed 0xC0FFEE (42 rooms)",
                    "[info] Baked navmesh in 18.4 ms",
                    "[warn] Cobweb has no collider — skipped",
                    "[info] Play mode ready",
                ] {
                    ui.caption(line);
                }
            });
        tabs.tab(BottomTab::Assets, "Assets")
            .icon(lucide::FOLDER_OPEN)
            .body(|ui| {
                ui.caption("Asset browser — imported meshes, textures and materials appear here.");
            });
    })
    .height(px(180));
}

fn right_panel(ui: &mut Ui, state: &mut Editor) {
    // A local copy of the selection so the Properties body can borrow the rest of
    // `state` (the whole inspector) without colliding with the tab's `&mut`.
    let mut right_tab = state.right_tab;
    ui.tabs(&mut right_tab, |tabs| {
        tabs.tab(RightTab::Properties, "Properties")
            .icon(lucide::SLIDERS_HORIZONTAL)
            .body(|ui| {
                inspector(ui, state);
            });
        tabs.tab(RightTab::Add, "Add")
            .icon(lucide::PLUS)
            .body(|ui| {
                ui.caption("Component browser — pick a component to add to the entity.");
            });
    })
    .width(px(320));
    state.right_tab = right_tab;
}

fn inspector(ui: &mut Ui, state: &mut Editor) {
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
