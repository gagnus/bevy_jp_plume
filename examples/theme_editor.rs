//! The theme-editor dialog on its own — a thin mount of `common::theme_editor`. The
//! same plugin is one of several the combined `showcase` example brings together.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(true);
    app.add_plugins(common::theme_editor::ThemeEditorPlugin(true));
    app.run();
}
