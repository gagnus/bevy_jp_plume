//! The control gallery on its own — a thin mount of `common::gallery`. The same plugin
//! is the full-screen backdrop the combined `showcase` example floats its dialogs over.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app();
    app.add_plugins(common::gallery::GalleryPlugin);
    app.run();
}
