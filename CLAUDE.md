# bevy_jp_plume ("Plume")

A bevy_ui control framework for game editors. Forked from `bevy_feathers`
(bevy fork at `../3rdParty/bevy`, branch `local`) and diverging deliberately.

- Code style + Bevy naming: [docs/code_rules.md](docs/code_rules.md)

## Plume rules (non-negotiable)

### Comments: very, very minimal — HARD LIMIT
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
constructor — no obscure companion components required. Controls
self-update their own value (still emitting `ValueChange`); app-vs-widget
conflicts are arbitrated by the immediate-mode layer, not per-entity markers.

### Dialog-first
`PlumeDialog` is the movable/floating dialog and the primary container.
There is no modal dialog. No panes; `group`/`subpane` for structure.

### Mouse-only interaction
No Tab-key navigation, focus ring, or keyboard activation of controls.
Text entry keeps click-to-focus and keyboard input. `TabIndex` survives
purely as the click-to-focus marker (`acquire_focus_tab_index` is
registered; `TabNavigationPlugin` and its Tab-key handler are not) —
without it, `PointerFocusPlugin` blurs on every press and focus-dependent
widgets (text input, select popups) break.

## Layout

`src/controls/`, `src/theme/` (palette → slot → token pipeline),
`src/utils/` (cursor, fonts, corners, constants), `src/imm/`
(immediate-mode API — the public surface), `src/display/`, `src/containers/`.
