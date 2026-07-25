//! The colour-picker body on its own — a thin mount of `common::color_picker`,
//! shown inline so the layout and colour round-trip can be tuned before it moves
//! into a popover.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::color_picker::ColorPickerPlugin);
    app.run();
}
