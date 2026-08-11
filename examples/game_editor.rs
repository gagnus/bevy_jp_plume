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
    icon: FaIcon,
    visible: bool,
    expanded: bool,
    children: Vec<SceneNode>,
}

fn leaf(id: u32, label: &str, icon: FaIcon, visible: bool) -> SceneNode {
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
    icon: FaIcon,
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
            font_awesome::solid::DUNGEON,
            true,
            vec![
                branch(
                    2,
                    "Entrance Hall",
                    font_awesome::solid::ARCHWAY,
                    true,
                    vec![
                        leaf(3, "Torch Sconce", font_awesome::solid::LIGHTBULB, true),
                        leaf(4, "Oak Door", font_awesome::solid::DOOR_CLOSED, true),
                        leaf(5, "Cobweb", font_awesome::solid::CUBE, false),
                    ],
                ),
                branch(
                    6,
                    "Crypt Level",
                    font_awesome::solid::LAYER_GROUP,
                    false,
                    vec![
                        leaf(7, "Sarcophagus", font_awesome::solid::CUBE, true),
                        leaf(8, "Cursed Altar", font_awesome::solid::CUBE, true),
                    ],
                ),
            ],
        ),
        branch(
            9,
            "Encounters",
            font_awesome::solid::DRAGON,
            true,
            vec![
                leaf(10, "Wandering Wraith", font_awesome::solid::GHOST, true),
                leaf(11, "Gravekeeper", font_awesome::solid::SKULL, true),
            ],
        ),
        branch(
            12,
            "Lighting",
            font_awesome::solid::LIGHTBULB,
            false,
            vec![
                leaf(13, "Sun", font_awesome::solid::LIGHTBULB, true),
                leaf(14, "Torch Flicker", font_awesome::solid::LIGHTBULB, true),
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
            theme.palette(ThemeSlot::Neutral1),
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
        ui.tool_button(font_awesome::solid::FILE);
        ui.tool_button(font_awesome::solid::FOLDER_OPEN);
        ui.tool_button(font_awesome::solid::FLOPPY_DISK);
        ui.separator();

        ui.tool_button(font_awesome::solid::ARROW_ROTATE_LEFT);
        ui.tool_button(font_awesome::solid::ARROW_ROTATE_RIGHT);
        ui.tool_button(font_awesome::solid::PASTE);
        ui.tool_button(font_awesome::solid::TRASH);
        ui.separator();

        for (tool, icon) in [
            (Tool::Select, font_awesome::solid::ARROW_POINTER),
            (Tool::Move, font_awesome::solid::ARROWS_UP_DOWN_LEFT_RIGHT),
            (Tool::Rotate, font_awesome::solid::ROTATE),
            (
                Tool::Scale,
                font_awesome::solid::UP_RIGHT_AND_DOWN_LEFT_FROM_CENTER,
            ),
        ] {
            let mut selected = state.tool == tool;
            if ui.tool_button(icon).checkable(&mut selected).flat().clicked {
                state.tool = tool;
            }
        }
        ui.separator();

        ui.tool_button(font_awesome::solid::MAGNET)
            .checkable(&mut state.snap_to_grid)
            .flat();
        ui.tool_button(font_awesome::solid::BORDER_ALL)
            .checkable(&mut state.show_grid)
            .flat();

        ui.flex_spacer();
        if ui
            .tool_button(font_awesome::solid::PLAY)
            .checkable(&mut state.playing)
            .flat()
            .clicked
        {
            state.playing = true;
        }
        ui.tool_button(font_awesome::solid::PAUSE);
        if ui.tool_button(font_awesome::solid::STOP).clicked {
            state.playing = false;
        }

        ui.separator();
        ui.tool_button(font_awesome::solid::PALETTE)
            .checkable(theme_editor_open);
    })
    .background(bg)
    .padding(size::GAP / 2.0);
}

fn left_panel(ui: &mut Ui, state: &mut Editor) {
    ui.tabs(&mut state.left_tab, |tabs| {
        tabs.tab(LeftTab::Scene, "Scene")
            .icon(font_awesome::solid::SITEMAP)
            .body(|ui| {
                for (i, node) in state.tree.iter_mut().enumerate() {
                    ui.push_id(i, |ui| tree_row(ui, node, &mut state.selected, 0));
                }
            });
        tabs.tab(LeftTab::Prefabs, "Prefabs")
            .icon(font_awesome::solid::CUBES)
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
                ui.space(size::TEXT_HEIGHT * (depth as f32));
            }
            if has_children {
                ui.disclosure(&mut node.expanded);
            } else {
                ui.space(size::ROW_HEIGHT);
            }
            ui.icon(node.icon);
            ui.caption(&node.label);
            ui.flex_spacer();
            if ui
                .tool_button(if node.visible {
                    font_awesome::solid::EYE
                } else {
                    font_awesome::solid::EYE_SLASH
                })
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
            .icon(font_awesome::solid::LIST)
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
            .icon(font_awesome::solid::FOLDER_OPEN)
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
            .icon(font_awesome::solid::SLIDERS)
            .body(|ui| {
                inspector(ui, state);
            });
        tabs.tab(RightTab::Add, "Add")
            .icon(font_awesome::solid::PLUS)
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
