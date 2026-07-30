//! The font-scaling dialog on its own — a thin mount of `common::font_scaling`. The
//! same plugin is one of several the combined `showcase` example brings together.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::font_scaling::FontScalingPlugin(true));
    app.run();
}
