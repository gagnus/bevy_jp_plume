//! The inspector panel on its own - a thin mount of `common::inspector_panel`.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::inspector_panel::InspectorPanelPlugin);
    app.run();
}
