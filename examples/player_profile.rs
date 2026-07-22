//! The player-profile dialog on its own — a thin mount of `common::player_profile`. The
//! same plugin is one of several the combined `showcase` example brings together.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app();
    app.add_plugins(common::player_profile::PlayerProfilePlugin);
    app.run();
}
