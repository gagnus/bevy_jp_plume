//! Tree-view dialog feature plugin: a collapsible world outliner, each node a checkbox
//! and a label, keyed by `push_id` so identity follows the data rather than call order.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};

const TITLE: &str = "World Outliner";

const INDENT: f32 = 14.0;

struct TreeNode {
    label: String,
    checked: bool,
    expanded: bool,
    children: Vec<TreeNode>,
}

fn leaf(label: &str, checked: bool) -> TreeNode {
    TreeNode {
        label: label.to_owned(),
        checked,
        expanded: false,
        children: Vec::new(),
    }
}

fn branch(label: &str, checked: bool, expanded: bool, children: Vec<TreeNode>) -> TreeNode {
    TreeNode {
        label: label.to_owned(),
        checked,
        expanded,
        children,
    }
}

/// The outliner's backing state: a forest of root nodes.
#[derive(Resource)]
pub struct OutlinerState {
    roots: Vec<TreeNode>,
}

impl Default for OutlinerState {
    fn default() -> Self {
        Self {
            roots: vec![
                branch(
                    "The Sunless Keep",
                    true,
                    true,
                    vec![
                        branch(
                            "Entrance Hall",
                            true,
                            true,
                            vec![
                                leaf("Torch Sconce ×4", true),
                                leaf("Oak Door", true),
                                leaf("Cobweb Decor", false),
                            ],
                        ),
                        branch(
                            "Guard Barracks",
                            true,
                            false,
                            vec![leaf("Skeleton Sentry", true), leaf("Weapon Rack", true)],
                        ),
                        branch(
                            "Crypt Level",
                            true,
                            true,
                            vec![
                                branch(
                                    "Bone Pit",
                                    true,
                                    false,
                                    vec![
                                        leaf("Skeletal Warrior", true),
                                        leaf("Skeletal Archer", true),
                                    ],
                                ),
                                leaf("Sarcophagus", true),
                                leaf("Cursed Altar", false),
                            ],
                        ),
                    ],
                ),
                branch(
                    "Lighting",
                    true,
                    true,
                    vec![
                        leaf("Ambient Fog", true),
                        leaf("Torch Flicker", true),
                        leaf("Moon Shaft", false),
                    ],
                ),
                branch(
                    "Navigation",
                    true,
                    false,
                    vec![leaf("Nav Mesh", true), leaf("Patrol Routes", true)],
                ),
                branch(
                    "Encounters",
                    true,
                    true,
                    vec![
                        leaf("Wandering Wraith", true),
                        leaf("Trap: Spike Pit", true),
                        leaf("Boss: The Gravekeeper", false),
                    ],
                ),
                branch(
                    "Audio",
                    false,
                    false,
                    vec![leaf("Dungeon Ambience", false), leaf("Water Drips", false)],
                ),
            ],
        }
    }
}

/// Adds the outliner dialog: its resource plus its dialog system and hub entry.
pub struct TreeViewPlugin(pub bool);

impl Plugin for TreeViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OutlinerState>().add_debug_dialog(
            TITLE,
            font_awesome::solid::SITEMAP,
            self.0,
            tree_view_dialog,
        );
    }
}

fn tree_view_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut state: ResMut<OutlinerState>,
) {
    let mut open = registry.is_open(TITLE);
    // Counts are read before the body borrows the tree mutably.
    let (checked, total) = tree_totals(&state.roots);
    // Deferred whole-tree action, applied after the body closure releases `state`.
    let mut set_all: Option<bool> = None;

    root.dialog(TITLE, &mut open)
        .width(em(24))
        .at(px(40), px(60))
        .show(|ui| {
            // Only the tree scrolls; the footer below stays pinned.
            ui.scroll_area_vertical(|ui| {
                for (i, node) in state.roots.iter_mut().enumerate() {
                    ui.push_id(i, |ui| node_row(ui, node, 0));
                }
            })
            .height(px(300));

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .icon_button(font_awesome::solid::ANGLES_DOWN, "Expand")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    set_all = Some(true);
                }
                if ui
                    .icon_button(font_awesome::solid::ANGLES_UP, "Collapse")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    set_all = Some(false);
                }
                ui.flex_spacer();
                ui.caption(&format!("{checked}/{total} enabled"));
            });
        });

    if let Some(expanded) = set_all {
        for node in &mut state.roots {
            set_expanded_recursive(node, expanded);
        }
    }
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
}

// Render one node's row, then recurse into its children when expanded.
fn node_row(ui: &mut Ui, node: &mut TreeNode, depth: usize) {
    let has_children = !node.children.is_empty();
    ui.horizontal(|ui| {
        if depth > 0 {
            ui.space(px(depth as f32 * INDENT));
        }
        // A twisty toggles the fold; a leaf reserves the same width so labels align.
        if has_children {
            ui.disclosure(&mut node.expanded);
        } else {
            ui.space(size::ROW_HEIGHT);
        }
        ui.checkbox(&mut node.checked, &node.label);
    });

    if has_children && node.expanded {
        for (i, child) in node.children.iter_mut().enumerate() {
            ui.push_id(i, |ui| node_row(ui, child, depth + 1));
        }
    }
}

// Count `(checked, total)` leaves-and-branches over the whole forest.
fn tree_totals(roots: &[TreeNode]) -> (usize, usize) {
    fn walk(node: &TreeNode, checked: &mut usize, total: &mut usize) {
        *total += 1;
        if node.checked {
            *checked += 1;
        }
        for child in &node.children {
            walk(child, checked, total);
        }
    }
    let (mut checked, mut total) = (0, 0);
    for node in roots {
        walk(node, &mut checked, &mut total);
    }
    (checked, total)
}

fn set_expanded_recursive(node: &mut TreeNode, expanded: bool) {
    node.expanded = expanded;
    for child in &mut node.children {
        set_expanded_recursive(child, expanded);
    }
}
