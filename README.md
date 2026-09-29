# bevy_jp_plume ("Plume")

A bevy_ui control framework with an immediate mode front-end for game debug UI.
Forked from `bevy_feathers` and diverging deliberately.

Original code MIT OR Apache-2.0 (see LICENSE-MIT / LICENSE-APACHE).

See CLAUDE.md for the design rules that distinguish Plume from feathers.

## Local Dependencies (temporary)

Plume targets Bevy 0.20, which is not yet released, so it currently builds against
local checkouts:

|Library|Branch|Path in `Cargo.toml`|
|---|---|---|
|bevy|release-0.20.0|../3rdParty/bevy|
|bevy_immediate|bevy-0.20|../3rdParty/bevy_immediate|

bevy_immediate's `bevy-0.20` branch takes bevy from crates.io, so also edit its
workspace `Cargo.toml` to point each `bevy_*` dependency at `../bevy/crates/<name>`.
Otherwise two copies of bevy end up in the build.

You might be better off waiting until Bevy 0.20 is officially released to use it!

## Quick start

Add `PlumePlugins`, then describe your UI from an ordinary system. Call the
widgets every frame; Plume keeps the real ones on screen in step:

```rust,no_run
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

#[derive(Resource, Clone, PartialEq)]
struct Audio {
    open: bool,
    volume: f32,
    muted: bool,
}

fn audio_dialog(mut root: PlumeRoot, mut audio: ResMut<Audio>) {
    // Edit a copy, so bevy only sees the resource change when something did.
    let mut a = audio.clone();
    root.dialog("Audio", &mut a.open).show(|ui| {
        ui.horizontal(|ui| {
            ui.caption("Volume");
            ui.slider(&mut a.volume, 0.0..=1.0).enabled(!a.muted);
        });
        ui.checkbox(&mut a.muted, "Mute");
        if ui.button("Reset").clicked {
            a.volume = 0.5;
            a.muted = false;
        }
    });
    audio.set_if_neq(a);
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(Audio { open: true, volume: 0.5, muted: false })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, audio_dialog)
        .run();
}
```

The `imm` module docs explain what is going on underneath. To see everything
at once, run the showcase:

```sh
cargo run --example showcase
```

## Screenshots

The showcase example: the feature dialogs floating over the debug hub:

![The showcase example: feature dialogs over the debug hub](docs/showcase.png)

The gallery example: every control in its enabled and disabled state, on each
of the three neutral surfaces:

![The gallery example: all controls across the three neutral surfaces](docs/gallery.png)

The inspector_panel example (available as imm version by default and a retained copy 
to see the difference): a docked editor panel built from tabs, splitter,
sections and the color widgets:

![The inspector_panel example: a docked editor panel](docs/inspector_panel.png)

Simple theme editor:

![Five theme palettes applied to the same UI, split diagonally](docs/theme_medley.png)

## Widgets

Every widget is available in both modes: retained scenes built with `bsn!`
(names from `bevy_jp_plume::retained`; `PlumeXxx` is a scene component spawned
as `@PlumeXxx { … }` with a matching `PlumeXxxProps`, lowercase names are scene
functions spawned as `@name(…)`), and immediate mode (from
`bevy_jp_plume::prelude`).

In the immediate column:

- `root` is the `PlumeRoot` system param; it opens the top-level surfaces.
- `ui` is the `Ui` a surface hands its closure; it builds the widgets inside.
- `response` is what every `ui.…` call returns; it carries the result
  (`.clicked`, `.changed`) and chains per-widget extras (`.tooltip(…)`,
  `.popup(…)`).

### Containers

| Widget | Retained | Immediate |
| ------ | -------- | --------- |
| Screen | `screen()` | `root.screen(\|ui\| …)` |
| Row | `row()` | `ui.horizontal(\|ui\| …)` |
| Column | `column()` | `ui.vertical(\|ui\| …)` |
| Dialog | `PlumeDialog` | `root.dialog(title, &mut open).show(\|ui\| …)` |
| Modal | `PlumeModal`, `modal_title()` | `root.modal(title, &mut open).show(\|ui\| …)` |
| Panel (headerless surface) | `PlumeDialog` with `header: false` | `root.panel().show(\|ui\| …)` |
| Popup | `PlumePopup`, `popup_socket()`, `close_popup(commands, entity)` | `response.popup(&mut open)` |
| Scroll area | `PlumeScrollArea` | `ui.scroll_area_vertical(\|ui\| …)` / `ui.scroll_area_horizontal(\|ui\| …)` |
| Section | `PlumeSection` | `ui.section("Header", \|ui\| …)` |
| Splitter | `PlumeSplitter`, `SplitPane` | `ui.split_horizontal(&mut split, \|ui\| …, \|ui\| …)` / `ui.split_vertical(…)` |
| Tabs | `PlumeTabs`, `PlumeTab`, `tab_label()`, `tab_body()` | `ui.tabs(&mut selected, \|tabs\| …)` |
| Reorderable list | `PlumeReorderable`, `PlumeReorderableItem` | `ui.reorderable(&mut items, key_fn, \|ui, item\| …)` |

### Controls

| Widget | Retained | Immediate |
| ------ | -------- | --------- |
| Button | `PlumeButton` | `ui.button("Label")` |
| Icon button | `PlumeButton` with icon + caption children | `ui.icon_button(lucide::…, "Label")` |
| Tool button | `PlumeToolButton` | `ui.tool_button(lucide::…)` |
| Checkbox | `PlumeCheckbox` | `ui.checkbox(&mut flag, "Label")` |
| Radio | `PlumeRadioGroup`, `PlumeRadio` | `ui.radio(&mut value, Variant, "Label")` |
| Toggle switch | `PlumeToggleSwitch` | `ui.toggle(&mut flag)` |
| Slider | `PlumeSlider` | `ui.slider(&mut value, 0.0..=1.0)` |
| Number input | `PlumeNumberInput` | `ui.number(&mut value)` |
| Text input | `PlumeTextInput` | `ui.text_edit(&mut text)` |
| Select | `PlumeSelect`, `select_options()` | `ui.select(&mut selected, \|sel\| …)` |
| Color edit | `PlumeColorEdit` | `ui.color_edit(&mut color)` (`_rgb` variant) |
| Color picker | `PlumeColorPicker` | `ui.color_picker(&mut color)` (`_rgb` variant) |
| Color swatch | `PlumeColorSwatch` | `ui.color_swatch(color)` (`_rgb` variant) |
| Disclosure | `PlumeDisclosure` | `ui.disclosure(&mut open)` |
| Menu | `PlumeMenuBar`, `PlumeMenuButton`, `menu_anchor()` | `ui.menu_bar(\|bar\| …)` |
| Scrollbar | `PlumeScrollbar` | none (scroll areas manage their own) |

### Display

| Widget | Retained | Immediate |
| ------ | -------- | --------- |
| Caption | `caption("text")` | `ui.caption("text")` |
| Icon | `icon(lucide::…)` | `ui.icon(lucide::…)` |
| Separator | `separator()` | `ui.separator()` |
| Space | `space(length)` | `ui.space(length)` |
| Flex spacer | `flex_spacer()` | `ui.flex_spacer()` |
| Tooltip | `Tooltip("text")` / `TooltipContent` components on the target control | `response.tooltip("text")` / `response.tooltip_container(\|ui\| …)` |
| Tooltip, only when clipped | `TooltipWhenClipped("text")` on the text | `response.tooltip_if_clipped("text")` |

## Credits

Plume stands on two other projects:

- [bevy_feathers](https://github.com/bevyengine/bevy/tree/main/crates/bevy_feathers),
  Bevy's own widget set, created by Talin ([@viridia](https://github.com/viridia))
  and built up by the Bevy contributors (MIT OR Apache-2.0). Plume's controls
  and theming started life as a fork of it, and still sit on Bevy's headless
  `bevy_ui_widgets` for their behavior.
- [bevy_immediate](https://github.com/PPakalns/bevy_immediate) by Pēteris
  Pakalns (MIT). It is the engine under Plume's immediate mode: it keeps track
  of which call made which widget, spawns and removes them, and leaves Plume to
  supply the widgets themselves.

## Fonts

Plume bundles its fonts as embedded assets, so they end up inside any binary
built against it. Their licenses are separate from the crate's MIT/Apache-2.0
and travel with anything you ship.

| Font | Copyright | License |
| ---- | --------- | ------- |
| Noto Sans, Noto Sans Mono | The Noto Project Authors | [SIL OFL 1.1](src/assets/fonts/NotoSans-LICENSE.txt) |
| Lucide | Lucide Icons and Contributors | [ISC](src/assets/fonts/Lucide-LICENSE.txt) |

Both licenses require that the copyright notice and license text accompany
redistribution. A subset of Lucide's icons is MIT (inherited from Feather;
both notices are in its license file).
