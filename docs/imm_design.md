# Step 6.0 research — bevy_immediate as the reconciler under Plume's imm API

Investigation of [PPakalns/bevy_immediate](https://github.com/PPakalns/bevy_immediate) (v0.8.0,
git main, examined 2026-07-18) as the foundation for `src/imm/`, per the step-6.0 checklist.
Recommendation at the end; decision pending.

## Compatibility (the decisive experiment)

- crates.io releases target bevy 0.19; the fork is `0.20.0-dev`, so **the crates.io package is
  unusable directly** (type mismatch across bevy versions).
- **Experiment**: cloned bevy_immediate git main, rewrote its `workspace.dependencies` bevy
  entries to path-deps on `../3rdParty/bevy/crates/*` (plus
  `bevy_input_focus = { features = ["bevy_picking"] }`, same as plume's own Cargo.toml).
  `cargo check -p bevy_immediate --no-default-features --features ui,bevy_ui_widgets,bevy_scene`
  **compiles clean with zero source changes**. All four member crates pass, including the
  optional `floating_ui`.
- Consequence: taking the dependency means a fork/vendored clone of bevy_immediate with one
  Cargo.toml commit, kept beside the bevy fork in `3rdParty`. The author tracks bevy main
  himself (commented-out `[patch.crates-io]` git pins in the workspace Cargo.toml), so upstream
  releases follow bevy releases; merge burden looks small.

## Crate anatomy

Workspace of four crates, ~6k lines total:

- `bevy_immediate_core` (~1.5k lines) — the reconciler. `Imm<Caps>` context, `ImmEntity`
  builder, `ImmId` hashing, entity↔id mapping, iteration-stamped GC, per-entity hash memory,
  capability access machinery, hotpatching support. Deps: bevy_ecs/app/platform/derive
  (+ bevy_scene optional), ahash, type-map, stackbox_2, log.
- `bevy_immediate_attach` (142 lines) — `ImmediateAttach<Caps>` trait: attach a `construct(ui,
  &mut Params)` fn to a root marker component, with a user-chosen `SystemParam`. The pattern for
  a later `add_debug_page` equivalent.
- `bevy_immediate_ui` — capability set for bevy_ui + headless `bevy_ui_widgets`: clicked,
  activated, checked, selected, slider_value, text_input (EditableText), number_input, disabled,
  text, look, layout_order, anchored. Feathers-specific caps are feature-gated off.
- `bevy_immediate_floating_ui` — optional: tooltips, anchored popups (dropdown/combobox),
  resizable/draggable floating windows. Only crate pulling `rand`. Plume keeps `PlumeDialog`,
  but this is minable for the tooltip/resize roadmap items.

MIT. Production-used (Settletopia). Active (201 commits). README warns "expect some breaking
changes, minimized" — a vendored pin neutralizes this.

## The step-6.0 checklist answered

- **System-param shape**: `ImmCtx<Caps>` is a plain `#[derive(SystemParam)]` — Commands, a
  `FilteredEntity` query, resource access, change ticks. **No exclusive `&mut World`
  anywhere**; imm systems parallelize normally. Capabilities declare component/resource access
  at plugin build time (`request_component_write::<T>()` etc.), aggregated into one
  `FilteredEntityMut` query — that dynamic-access machinery is the genuinely hard part we get
  for free.
- **Child ordering**: a `UiOrderTracker` resource records visit order during the imm pass; a
  PostUpdate system compares against `Children` and **swaps only when the order differs**
  (tolerates non-imm siblings). Exactly the plan's design.
- **GC/sweep**: each visited entity's `ImmMarker` is stamped with an iteration counter; a
  PostUpdate upkeep system despawns stale-stamped entities (`try_despawn`, tolerant of ancestors
  going first) and bumps the counter. id→entity mapping is maintained by Add/Remove observers on
  `ImmMarker`, so despawns clean the map automatically. Matches the plan.
- **Change detection**: consume, not poll — a `TrackValueChangePlugin<T>` observer captures
  `ValueChange<T>` into a per-entity `NewValueChange<T>` mailbox component; the next imm pass
  takes it. Only user edits produce `ValueChange`, so widget-vs-app writes are distinguished
  without a last-written cache for the common cases.
- **Click latency**: same-frame. `Pointer<Click>` observer records into a resource cleared in
  `First`; an imm system running in `Update` (after picking delivery) sees this frame's click.

## Deviations from the plan's first-cut design (all handled in Plume's wrapper layer)

1. **IDs are positional, not source-location.** `ui.ch()` auto-ids increment a per-scope
   counter — conditional widgets shift their siblings' identities (README tells you to use
   `.ch_id` for anything that appears/disappears). No `#[track_caller]` anywhere in the crate.
   Fix in the wrapper: every `PlumeUi` method is `#[track_caller]` and passes
   `Location::caller()` (+ optional key) through the public
   `ch_with_manual_id(ImmIdBuilder::Hierarchy(...))`, giving egui-style stable ids exactly as
   planned. `ui.push_id(key, …)` maps to `with_add_id_pref`.
2. **Slider who-wins mid-drag differs.** bevy_immediate's `.slider(&mut v)` lets an app write
   land during a drag (app pushes whenever `v != SliderValue`); the plan wants widget-wins
   while interacting (Reset-mid-drag case). Plume writes its own slider capability with the
   planned last-written cache — capabilities are pluggable and this one is ~100 lines. The
   text-input capability, by contrast, already implements the plan's rule exactly (while
   focused, `EditableText` is source of truth and external values are never pushed in;
   app pushes only when unfocused).
3. **Dialog ✕ round-trip.** `PlumeDialog` currently despawns itself on `RequestClose`; under
   the reconciler the id would just respawn next frame. Retained-layer change: close requests
   set a marker/flag component instead; the wrapper reads it, writes `*open = false`, stops
   visiting the id, and the sweep despawns. (Keep despawn-on-close for retained-only users via
   an observer that plume's imm layer replaces.)
4. **Response shape.** `ImmEntity` methods are consuming builder chains returning `Self`;
   the plan's `ImmResponse { clicked, changed, hovered, entity }` is assembled by the wrapper
   from the clicked/interaction/value capabilities before returning.
5. **Capabilities see only imm-managed entities** (found during pass 1): `ImmCapQueryParam`
   is `With<ImmMarker>`-filtered, so a capability cannot reach scene-spawned children (e.g.
   a select's rows). Rule: any state a capability needs must live **on the widget's root
   entity**, maintained by the retained layer — hence `SelectedIndex` on `PlumeSelect` plus
   the `SetSelectedIndex` event for writes. Follow this pattern for future composite widgets.

## How the bespoke API sits on top

The crate is explicitly designed for this (the `power_user` example builds a custom API):

- `PlumeCaps` — plume's own `impl_capability_set!` (clicked, activated, layout_order, disabled,
  text, text_input + plume-written caps for slider who-wins, dialog close, hovered).
- Retained scenes plug straight in: unseen id →
  `.on_spawn_apply_scene(|| bsn! { PlumeButton { … } })`. Prop updates on change via
  `on_hash_change_insert` / plume capabilities. Theming untouched, as planned — the reconciler
  writes value/prop components only.
- App-facing surface: `PlumeUi` (a small `SystemParam` wrapping `ImmCtx<PlumeCaps>` +
  root handling) with methods `button/checkbox/slider/dialog/…` returning `ImmResponse`.
  bevy_immediate types appear nowhere in app code; docs stay plume-branded.

### First target example (pass-1 acceptance)

**Decision: this example is the target of the first implementation pass** — pass 1 is "implement
what's required to make this run", nothing more. It lands as a real example
(`examples/audio_settings.rs`) and is the acceptance test: if it needs any wiring beyond the one
system, pass 1 isn't done. Exact method names may still shift during implementation; the scope
doesn't.

```rust

#[derive(Resource)]
struct AudioSettings {
    open: bool,
    volume: f32,
    muted: bool,
    output: usize,
}

// Registration: PlumePlugins + this one system in Update. No observers, no sync systems.
fn audio_settings_ui(mut ui: PlumeUi, mut settings: ResMut<AudioSettings>) {
    // Destructure so `open` and the fields can be borrowed simultaneously (egui-style).
    let AudioSettings { open, volume, muted, output } = &mut *settings;

    ui.dialog("Audio", open, |ui| {
        ui.horizontal(|ui| {
            ui.caption("Volume");
            if ui.slider(volume, 0.0..=1.0).changed {
                // react same-frame if needed; the value is already written back
            }
            ui.number(volume).enabled(!*muted);
        });

        ui.checkbox(muted, "Mute");
        ui.select(output, &["Speakers", "Headphones"]);
        ui.separator();

        if ui.button("Reset").clicked {
            *volume = 0.5;
            *muted = false;
        }
    });
}
```

Semantics the wrapper guarantees here:

- While `*open` is false the dialog id is simply not visited, so the sweep despawns the whole
  subtree; flipping it true respawns fresh. Clicking ✕ sets the close-request flag on the
  retained entity, which the *next* `ui.dialog` call reads and writes back as `*open = false`.
- The dialog entity persists across frames while open, so a user-dragged position sticks — the
  reason this is a reconciler and not respawn-every-frame.
- Every widget call is `#[track_caller]`, so the `if`-gated `number` doesn't shift its
  siblings' identities; only widgets emitted inside loops need `ui.push_id`.

And the flavor of what one wrapper method does underneath (checkbox, abbreviated):

```rust
#[track_caller]
pub fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse {
    let mut e = self
        .imm
        .ch_with_manual_id(loc_id())         // Location::caller() → ImmIdBuilder::Hierarchy
        .on_spawn_apply_scene(|| bsn! { PlumeCheckbox { caption: [{caption(label)}] } })
        .checked(value);                     // mailbox-based who-wins from the capability
    e.on_hash_change_typ_insert::<CaptionKey, _, _, _>(&label, /* re-caption */ …);
    ImmResponse::from_caps(&mut e)           // clicked / changed / hovered / entity
}
```

## Recommendation

**Take the dependency, as a vendored fork wrapped by the bespoke `PlumeUi` layer.** The core
reconciler is a near-exact implementation of the step-6 design (ordering, GC, mailbox change
detection, same-frame clicks, parallel-friendly system param), it compiles against the bevy
fork today without a single source edit, and its capability system is purpose-built for
exactly the "bespoke API on top" requirement — so the API compromise the dependency was feared
to force turns out not to exist. Building our own would re-derive this design minus the
dynamic-access machinery and the child-reorder algorithm, the two subtle parts.

Costs accepted: one more `3rdParty` clone to bump alongside bevy; wrapper-layer workarounds
for the three deviations above; upstream API churn absorbed by the pin.

## Pass 1 — scope and build order

Pass 1 delivers exactly the first target example above. In scope:

- Vendored bevy_immediate fork in `3rdParty` (Cargo.toml path-dep commit only), wired as a
  plume dependency (`core` + `attach` + `ui` features; no `floating_ui`, no `bevy_feathers`).
- `PlumeCaps` capability set + `PlumeUi` system param; `#[track_caller]` location ids;
  `ImmResponse { clicked, changed, hovered, entity }`.
- Widgets/containers the example uses, wrapping the existing retained scenes:
  `dialog` (with `&mut open` round-trip + persistent drag), `horizontal`, `flex_spacer`,
  `caption`, `slider`, `number`, `checkbox`, `select`, `separator`, `button`, plus the
  `ImmResponse` builders `.enabled()`, `.width(Val)`, `.grow()`.
- Retained-layer change: `PlumeDialog` close-request flag replacing self-despawn (deviation 3).
- Plume-owned slider/number who-wins capability (widget wins mid-drag — deviation 2).
- Layout decision (settled during pass 1, after trying the alternative): contents slots keep
  the feathers **Stretch** default — a Start/hug default broke grow-based row layouts and
  ragged sections. Instead: checkbox/radio roots are `Pickable::IGNORE` so their stretched
  dead space isn't clickable, and per-widget sizing is opt-in via the `ImmResponse` builders
  `.width(Val)` and `.grow()` (both change-gated so a quiet frame never dirties layout).
  Rows pack Start on their main axis by flexbox nature; `ui.flex_spacer()` + `.grow()` are
  the distribution knobs.

Explicitly deferred to pass 2+: radio, toggle, text_edit, section/group,
`.flat()`/`.push_id()` polish, and the theme_editor dogfood (step 7 unchanged). Plus, found
while dogfooding pass 1:

- **Top-level text context**: a widget at the root scope (outside any dialog/row) has no
  `InheritableFont`/text-color ancestor, so `ui.caption` there renders in Bevy's default
  font. Controls must work anywhere — the plan's standing note ("every imm root panel
  establishes the same text context as the dialog root") needs an actual home, e.g. the imm
  layer wrapping top-level widgets in a styled root panel, or text scenes carrying their own
  font fallback.
- **Dialog width prop** on `ui.dialog`, so `.grow()`-based rows work in imm dialogs
  (since built — see "Pass 2" below).
- **Dynamic-label text updates** (labels are currently hashed into the id, so a changing
  caption respawns its entity every change — fine for settings labels, wrong for readouts
  like the debug caption that exposed the font gap).

Internally the pass still proceeds in small compiling commits (fork wiring → PlumeUi skeleton
with button/caption → value widgets + who-wins → dialog round-trip → select), stop-and-revert
rather than patching forward — but the example is the single acceptance test, and nothing gets
built that it doesn't need.

## Pass 2 — imm surface status & remaining gaps

Pass 2 is driven by `examples/debug_settings.rs` (the smoke debug menu, resource-backed, live
`Debug`-printed, Reset→`Default`). Building it added, on top of pass 1: `toggle`, `vertical`
(and `horizontal`/`vertical` now return `ImmResponse` so containers take `.grow()`/`.width()`),
`section` (+ retained `section_frame`/`section_body` split mirroring the dialog, + `SectionRoot`
plain marker so the frame isn't inserted as a bare scene-component), `icon_button`/`tool_button`,
and `ImmResponse` builders `.variant()`/`.primary()`, `.range()`/`.precision()`/`.suffix()`,
`.align_top()` (since superseded by `.align_items()`), `.start_collapsed()`, `push_id`.
`FaIcon`/`FaFace` gained `Hash` (icon is part of
a widget id). `section_body` is `align_items: Stretch` (a `Start` slip regressed row fills).

### Exposed vs. gaps (per control)

| Control | imm surface | Not yet exposed |
|---|---|---|
| button | `button`, `icon_button`, `tool_button`, `.variant()`/`.primary()`, `.enabled()` | `corners` — deferred, see below |
| checkbox | `checkbox(&mut bool, label)`, `.enabled()` | — |
| toggle | `toggle(&mut bool)`, `.enabled()` | — (no props by design) |
| slider | `slider(&mut f32, range)`, `.step()`, `.precision()` (drag rounding), `.enabled()`, `.grow()`/`.width()` | — |
| number | `number(&mut f32)`, `.range()`, `.step()`, `.precision()`, `.suffix()`, `.enabled()`, `.width()` | — |
| select | `select(&mut usize, &[opts])`, `.max_visible()`, `.enabled()`, `.grow()`/`.width()` | `corners` |
| text_edit | `text_edit(&mut String)`, `.placeholder()`, `.suffix()`, `.enabled()`, `.grow()`/`.width()` | `filter`, `max_characters`, `visible_width` |
| radio | `radio(&mut value, variant, label)`, `.enabled()` | group arrow-key nav (each radio is its own tab stop) |
| color_swatch | **none** | whole widget: `show_alpha`, `opaque_color_percentage` |
| dialog | `dialog(title, &mut open)` builder: `.width()`/`.height()`/`.max_height()` (live), `.at(left, top)` (spawn-only), `.closable()`, `.movable()`, `.show(f)` | — |
| section | `section(header, f)`, `.start_collapsed()` | `collapsible: false` (titled non-collapsing box) |
| containers | `horizontal`/`vertical`/`group`/`screen` (+`.grow`/`.width`/`.align_items`), `separator`, `space`, `flex_spacer`, `push_id` | `row`/`column` gap knobs (deferred, see below) |

### Priority order for closing the gaps (agreed 2026-07-19)

**None** — every per-control gap above is closed; next is the theme_editor dogfood (step 7).

Done 2026-07-20: **layout axis pass**. `kind::Container` split into `kind::Row` (`horizontal`)
and `kind::Column` (`vertical`/`group`/`screen`) behind a `kind::Container` marker trait,
matching the existing `Numeric`/`Field` pattern. Both kinds share `.align_items()` today — the
split earns nothing for that method alone and is an investment in row-only/column-only builders
later. `.align_top()` is **gone**: it set `align_items: Start` but was implemented on every
container, so on a column it silently left-aligned and dropped the `Stretch` that `.grow()`
resolves against. Added `.align_self(AlignSelf)` (universal) for the case a column's `Stretch`
has to be escaped by one child — previously only expressible by wrapping the child in a
`horizontal` + `flex_spacer`.

Done 2026-07-20: **bounded, scrolling dialogs**. `.height(Val)` (fixed outer height) and
`.max_height(Val)` (hug content up to a ceiling) on the imm dialog builder, plus matching
`height`/`max_height` on `PlumeDialogProps` for the retained `PlumeDialog`. Either one bounds the
frame and grows the body a scrolling viewport; neither set keeps the old hug-the-content shape and
spawns no scroll entities at all. `max_height` is the one to reach for on settings dialogs.

**Both dialogs build the same structure** — scrolling is a dialog property, not something a
caller assembles. `examples/smoke.rs` used to hand-build a scroll frame, gutter, `ScrollArea` and
`PlumeScrollbar` inside its `@contents`; it now passes `@max_height: px(600)` and nothing else.
The two paths diverge only in how the scrollbar names the viewport it drives: the retained scene
resolves a `#inner` reference inside its own `bsn!`, while the imm layer holds a live `Entity`
(`EntityTemplate::from_reference` needs macro-generated invocation data, so it can't be built by
hand across the imm boundary). Only the scrollbar's placement is shared, as
`dialog_scrollbar_node`.

Structure mirrors what the theme editor used to build by hand: body → `dialog_scroll_frame`
(`ScrollbarGutter`) → `dialog_scroll_area` (`ScrollArea`, `Overflow::scroll_y`) + an absolutely
positioned `PlumeScrollbar` beside it. Vertical only, by construction — the orientation and
overflow axis are hardcoded, so no caller can produce a sideways-scrolling dialog. The scroll
frame stays a distinct entity from `PlumeDialogBody` because `update_scrollbar_visibility`
*assigns* `padding.right` for the gutter, which on the body would eat its own padding whenever
content started overflowing. The scrollbar finds its target because `ch_id` returns the `Entity`
before the spawn command flushes, and `EntityTemplate: From<Entity>` takes it directly — no `#id`
scene path needed across the imm boundary.

Two traps worth remembering. **`min_height: 0` is load-bearing** on the body, scroll frame and
scroll area: flex items floor at their content size, so without it a bounded dialog pushes content
out the bottom instead of scrolling. And the frame carries **`min_height: DIALOG_HEADER_HEIGHT`**
— `min` resolves after `max`, so a `height`/`max_height` set below the title bar's own height is
clamped away instead of painting the bar outside the dialog's border. Verified: a bounded dialog
scrolls with the bar visible and the gutter reserved, an unbounded one is unchanged, `height(20)`
degrades to a bare title bar with intact chrome, and a select popup opening past the bottom edge
is *not* clipped by the scroll area (`OverrideClip` on `PlumeMenuPopup` already handled it).

`separator` and the new `space(length)` are orientation-free by construction: `flex_basis` sizes
the main axis whichever way the parent flows, and the separator adds `align_self: Stretch` to
span the cross axis even in a row's `Center`. No parent lookup, no direction flag — and it fixed
a silent failure, since the old height-only separator rendered as a zero-size node in a row.

Done 2026-07-19: **slider/number `.step()` + slider `.precision()`** — one `.step()` builder
(change-gated `SliderStep` insert; the retained widgets already read it for arrow keys and
Up/Down), and `.precision()` now covers sliders too (inserts `SliderPrecision` for drag
rounding when the entity isn't a number input). `debug_settings`'s `slider_row` takes a `step`
arg and applies both to slider and number. Screenshot-verified; arrow-step/drag-rounding feel
needs the manual pass.

Done 2026-07-19 (later the same day):

- **Dialog config** — `ui.dialog(title, &mut open)` returns an `ImmDialog` builder
  (`.width()`, `.at(left, top)`, `.closable(false)`, `.movable(false)`), and `.show(|ui| …)`
  builds the body (`#[must_use]` guards a forgotten `show`). Width is app-owned and live
  (change-gated `Node` write); position is spawn-only — once open, the user's dragging owns
  it. `debug_settings` drives `.width(px(600))` with two equal-`.grow()` columns.
- **`radio`** — egui-style `ui.radio(&mut value, variant, label)` (`T: PartialEq`): checked
  derives from `*value == variant` through the existing checked sync, so a "group" is just
  radios sharing a binding — no container entity, and sibling uncheck falls out of app-wins a
  frame after another radio is picked. `RadioGroup` arrow-nav is not wired: each radio is its
  own tab stop and Space operates it.
- **`text_edit`** — `ui.text_edit(&mut String)` plus `.placeholder()` (spawn-only, like
  `.suffix()`). The field child is capability-unreachable (deviation 5), so the retained layer
  maintains a **`TextInputValue` mirror on the frame root**: the scene seeds it as the initial
  text (`PlumeTextInputProps.value`), a `seed_text_input_value` system pushes the seed into
  the buffer, every buffer change mirrors back (skipping buffers with `pending_edits`, so a
  queued seed/write is never clobbered by its own pre-edit state), and `SetTextInputValue` is
  the write event. `CapabilityPlumeText` syncs select-style (hash-memory of the last synced
  text). Who-wins: typing lands in the app string per edit (not on commit); app pushes wait
  for blur. `player_profile.rs` is the acceptance example.

**Kind-typed responses** (2026-07-19): `ImmResponse<'r, 'w, 's, K = kind::Any>` — every widget
returns a response typed with a zero-sized `imm::kind` marker (`Button`, `Slider`, `Number`,
`Text`, `Select`, `Section`, `Container`; `Any` for caption/checkbox/toggle/radio), and builders
are gated per kind, so a mismatched chain (`ui.button(…).step(…)`) is a compile error instead of
silent component litter (`.step()` used to insert `SliderStep` on anything; `.suffix()` spawned a
visible child). Universal on all kinds: `.enabled()`, `.grow()`, `.width()`. Shared-kind traits:
`kind::Numeric` (slider+number → `.step()`), `kind::Field` (number+text → `.suffix()`).
`.precision()` split into per-kind impls (slider inserts `SliderPrecision`; number sets
`PlumeNumberInput.precision` + re-touches `SliderValue`) — no more runtime component sniffing.

**select `.max_visible()`** (2026-07-19): change-gated; post-spawn it re-derives the popup list
view's `max_height` via `set_select_max_visible` (a world walk owned by `select.rs`, since the
list view is a scene child outside capability reach). Exercised inline on `debug_settings`'s
Log-level select (`.max_visible(3)` — five options, popup scrolls; popup behavior needs the
manual pass, screenshots only show it closed).

Example idiom (all three imm examples — `audio_settings`, `debug_settings`, `player_profile`):
resource-backed with `Default` + `Clone` + `PartialEq`; build the UI against a clone and write
back with `set_if_neq` (passing `&mut settings.field` straight from a `ResMut` marks the
resource changed every frame regardless of edits); shared scaffolding in `examples/common/mod.rs`
(`#[path]`-included): `log_on_change::<R>` prints the resource on real changes, and
`apply_args` parses the example command line (`argh`) — `--screenshot shot.png` saves a PNG +
exits for headless verification, `--light` swaps the dark default palette for the light one
(`cargo run --example … -- --screenshot shot.png --light`).

### Naming: the container is `section` (renamed from `subpane` 2026-07-19)

The header-over-body container is **`section`** throughout (`PlumeSection`/`Props`,
`section_frame`/`section_body`, the `Section*` markers, `SectionPlugin`, the public
`SectionCollapsed`, `section.rs`, imm `ui.section`, tokens `plume.section.*`). `subpane` was a
feathers leftover implying a subdivision of a "pane" that doesn't exist; `section` was chosen
over `collapsing`/`collapsable_pane` because it doesn't lie about the non-collapsible
`collapsible: false` case, drops the "pane" baggage, fits the single-word container set, and
dodges the collapsible/collapsable spelling bikeshed. Collapse is a behavior of a section
(`.start_collapsed()`, `collapsible: false`), not its identity.

### Deferred until a real use case (not just missing — deliberately waiting)

- **button `.corners()`** — only earns its keep for segmented/pill button groups (three
  touching buttons as one shape); no consumer yet.
- **`row`/`column` gap knobs** — no consumer has needed to deviate from `size::GAP`.
- **`justify_content`** — `flex_spacer` covers main-axis distribution so far.
- **per-kind directional align sugar** (`Row::align_top`, `Column::align_left`) — the kind
  split makes it possible; wait until `align_items`' axis flip is a demonstrated nuisance.

### Id scheme: occurrence counter (DONE 2026-07-20 — killed most `push_id`)

Previously an id was `hash(parent, Location::caller(), key)` with **no positional
disambiguation**, so a widget-emitting helper (or loop body) called more than once under the
same parent produced colliding ids unless the caller wrapped each call in `push_id`. Forgetting
it was a silent, nasty failure — verified 2026-07-19: continuous per-frame `entity id collision`
warnings, the colliding widgets despawn+respawn every frame (thrash, entity ids climb). No
panic; the UI still roughly renders, which hides it.

Implemented (a strict superset of the per-location-counter candidate): the counter is keyed by
the **fully resolved hierarchy id** — `Imm` carries a per-pass `hierarchy_occurrences` map
(fresh each system run, so no reset logic), and `ImmIdBuilder::Hierarchy` resolution counts
occurrences of each resolved `hash(parent, id_pref, sui_id)`. The first occurrence keeps the
plain id (no churn vs. the old scheme; a widget appearing once resolves identically); repeat
occurrences mix in their occurrence index. Because the key includes location *and* the widget's
own key *and* scope, a conditional widget only ever shifts later repeats of its own exact id —
distinct-location/key/scope siblings keep the stable-conditional property (deviation 1). Since
it lives in `resolve`, `ch_id` dups are covered too, not just `loc_id`. This is the first
source change in the vendored fork beyond the Cargo.toml path-dep rewrite (~20 lines:
`immediate/id.rs` + the `Imm` field).

`push_id` survives as an explicit override for identity that should follow **data** rather than
call order: reorderable `Vec` rows, or repeats after a conditional sibling *of the same id*
(positional identity shifts there, respawning widget-local state like collapse/focus).
`debug_settings` now uses zero `push_id` calls — its repeated `toggle_row`/`select_row`/
`slider_row` helpers are the acceptance test (no collision warnings, screenshot-identical).

### Top-level text context: `ui.screen` (DONE 2026-07-20)

`caption()` spawns a bare `Text` + `ThemedText` with no font or color of its own; both arrive by
propagation from an ancestor carrying `InheritableFont` (→ `Propagate<TextFont>`) and
`InheritableThemeTextColor` (→ `Propagate<TextColor>`). `build_immediate_root` starts with
`entity: None` — a *virtual* root — so top-level widgets spawn as separate UI roots with no
common parent, and a bare `ui.caption` at root scope had no such ancestor and fell back to
Bevy's default font. Nesting already fixed it (`row`/`column`, and therefore
`ui.horizontal`/`ui.vertical`, carry the pair); only unwrapped root widgets were affected.

Resolved with an **explicit container**, `ui.screen(|ui| …)`, over auto-wrapping every root:
auto-wrapping would force a full-screen surface on dialog-centric apps (the existing examples
are exactly that — a trigger button plus `ui.dialog`) and would need a way to descend `Imm`'s
private `current` permanently, i.e. another vendored-fork change. `screen` is an ordinary
container like `group`/`vertical`, so it costs nothing when unused.

The `screen()` scene is a viewport-filling `column`: absolute, 100%×100%, flex column with
`GAP`/`PAD`, plus the same `InheritableFont` + `InheritableThemeTextColor` pair. It is
deliberately **transparent** (no background token — an overlay over whatever renders behind)
and `Pickable::IGNORE`, so empty areas don't swallow picks meant for the scene below while
children keep their own picking. `examples/screen_demo.rs` is the acceptance case: bare
caption, separator, buttons and a checkbox at root scope, all themed.

### Still-open cross-cutting items (carried from pass 1)

- **Dynamic-label text updates**: labels are hashed into the widget id, so a changing caption
  respawns its entity every change — fine for static labels, wrong for live readouts. Wants a
  text-update path that leaves the entity in place.

### New retained port-back candidate found in pass 2

- **Editable-field alignment dropped on BSN spawn** — `bevy_ui`'s `update_editable_text_styles`
  applies parley alignment only on `TextLayout::is_changed`, but a scene-spawned field's change
  fires before it is layout-eligible (`ComputedUiRenderTargetInfo` lands later), so a non-default
  justify (the number input's right-align) is silently dropped. Plume works around it with a
  `reapply_field_justify` system re-touching `TextLayout` on `Added<ComputedUiRenderTargetInfo>`.
  Same class as port-back item 2 (is_changed-filtered style systems miss scene-spawned entities).
