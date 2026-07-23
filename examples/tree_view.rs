//! The world-outliner tree-view dialog on its own — a thin mount of
//! `common::tree_view`. If it earns its keep, the same plugin can join `showcase`.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::tree_view::TreeViewPlugin(true));
    app.run();
}
