# bevy_jp_plume ("Plume")

A bevy_ui control framework with an immediate mode front-end for game debug UI. 
Forked from `bevy_feathers` and diverging deliberately.

- Code style + Bevy naming: [docs/code_rules.md](docs/code_rules.md)
- Scene composition + imm/retained rules: [docs/plume_rules.md](docs/plume_rules.md)

## Plume rules (non-negotiable)

### Comments: very, very minimal (HARD LIMIT)

The Comments section of code_rules.md applies at maximum strictness. Never
write plan/step references ("implements step 2"), history or attribution
notes, or multi-line explanations of anything a reader can see in the code.
One line stating a non-obvious constraint is the ceiling. When in doubt,
write no comment.

### Plume-only surface

Apps import only `bevy_jp_plume`. Never require a consumer to mix plume
types with the underlying `bevy_ui_widgets` types. Everything an app
legitimately touches (`Checked`, `Selected`, `SliderValue`, `ValueChange`,
`Activate`, ...) is re-exported or wrapped here. Where feathers made apps
reach down (e.g. `RadioGroup`), plume provides its own variant.

### Controls work when dropped in

Every control must behave sensibly with nothing but its `@PlumeX { ... }`
constructor, with no obscure companion components required. Controls
self-update their own value (still emitting `ValueChange`); app-vs-widget
conflicts are arbitrated by the immediate-mode layer, not per-entity markers.

### Debug-overlay-first

The target is debug overlays over a running game (imgui/egui's emphasis),
not a full Unity-style editor. `PlumeDialog` is the movable/floating
dialog and the primary container; `PlumeModal` centers over a barrier
that blocks the app behind it, for the rare prompt that must be answered.
Full-screen roots (menu bar, root panels) are in scope; `group`/`section`
for structure within a surface.

### Full keyboard reach

Every interactive control is tabbable (`TabIndex(0)`) and keyboard-
operable via the headless `bevy_ui_widgets` handlers (Enter/Space
activates, arrows move sliders/radios/lists). Keyboard focus shows a
focus ring: `FocusIndicator` on the entity the ring should hug (the
box/disc for checkbox/radio, the root for button/toggle/slider),
`FocusWithinIndicator` on frames whose inner child holds focus (text
input); ring only when `InputFocusVisible` (keyboard-driven focus).
Tabbable entities need a `TabGroup` ancestor: `PlumeDialog` carries one,
app-built root panels add their own (re-exported at the crate root).
Disabled controls leave the Tab order (`FocusPlugin` swaps `TabIndex` to
-1 and back). Without a `TabIndex`, `PointerFocusPlugin` blurs on every
press and focus-dependent widgets break, so every control keeps one.

## Layout

`src/controls/`, `src/theme/` (palette → slot → token pipeline),
`src/utils/` (cursor, fonts, corners, constants), `src/imm/`
(immediate-mode API, the public surface), `src/display/`, `src/containers/`.
