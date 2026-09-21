# Plume Rules

Rules specific to this crate, on top of [code_rules.md](code_rules.md)
(which is shared with other projects and must stay generic).

# Scene composition

Everything plume draws is built two ways: an app spawns it by hand
(`retained`), or the immediate layer builds it a frame at a time (`imm`).
Every rule below follows from those two paths having to produce the same
entities.

## Scene component or scene function

Write a **scene component** (`@PlumeX`) when the thing is a widget an app
names as a unit *and* it needs named optional props with defaults,
above all `Box<dyn SceneList>` content slots, which are how the retained
path passes children.

Write a **scene function** (`fn x() -> impl Scene`) when either:

- It is one flat entity of layout or styling with no chrome of its own:
  `column`, `row`, `space`, `separator`, `flex_spacer`, `caption`,
  `icon`. Arguments are fine; a scene function taking a `String` or a
  `Val` is still a scene function.
- It is a *piece* of a composite (next rule).

A scene component whose props are empty and whose scene is a single node
is a scene function that hasn't been written yet, unless the node has
behavior of its own. `PlumeRadioGroup` (unchecks the other radios) and
`PlumeMenuBar` (the root the menu machinery looks for) are single nodes
with no props, but an app names them as widgets and they do more than lay
out, so they stay scene components.

## Composites are cut into pieces both paths compose

A prop takes a *finished* `SceneList`, and the imm layer has no finished
list: its children arrive over the course of a frame. So a widget that
hosts children cannot be built through its own props on the imm path.

Cut it into `pub(crate)` scene functions, one per entity in its chrome,
and let both paths compose them: `dialog_frame`, `tabs_frame` /
`tab_strip` / `tab_button`, `splitter_frame` / `_pane` / `_divider`,
`scroll_frame` / `_viewport` / `_content`, `section_frame`. The scene
component then becomes a thin retained-only wrapper that assembles the
same pieces around its props.

Pieces are `pub(crate)` unless an app has a real reason to hand-build the
widget from them (`tab_body`, `popup_socket`, `tab_label`). Chrome an app
would never spawn alone stays private.

## Systems key on a plain marker, never on a scene component

A scene component only exists on entities spawned through its own `@`
template. The imm path builds composites from pieces and so never inserts
it. A system that queries `With<PlumeX>` therefore silently sees only
half the widgets in the app.

Every widget, composite or leaf, carries a plain `Component` marker
inserted by its own scene, and the systems key on that: `SectionRoot`,
`SplitterRoot`, `TabsRoot`, `MenuButtonRow`, `CheckboxFrame`, or the
headless component bevy already provides (`ScrollArea`, `Dialog`,
`RadioButton`). No exemption for leaves that happen to be spawned
through `@PlumeX` on both paths today: that is a property of how the
imm layer builds them this week, not of the widget.

State a system needs lives on the plain marker too, not on the scene
component (`NumberInputFrame { precision }`, not
`PlumeNumberInput { precision }`), for the same reason.

## `Plume` prefix means scene component

`PlumeX` is reserved for scene components. Plain markers are unprefixed:
`Separator`, `PopupSocket`, `TabButton`, `ScrollContent`. The prefix is
the only signal a reader has that `@PlumeX` is legal in `bsn!`, and a
`Plume`-prefixed plain component makes that signal a lie, public ones
most of all, since `retained` exports both kinds in one list.

Public scene *functions* stay unprefixed (`column`, `caption`), so the
prefix keeps that single meaning. They are module-namespaced, and the
common import is `prelude::*`, which exports no scene functions at all.

One exception, and only one: `PlumeIgnore`. It is a plain component, not a
scene component, and `@PlumeIgnore` is not legal. It keeps the prefix
because the name says exactly what it does (tells Plume to leave this text
alone), and an app reads it as an instruction to the library rather than
as a widget. Do not add a second.

## Capabilities reach only the widget's root entity

`bevy_immediate`'s capability query is `With<ImmMarker>`-filtered, so a
capability can never touch a scene-spawned child. Any state the imm layer
reads or writes must live **on the widget's root**, mirrored there by the
retained layer. That is why `TextInputValue` sits on the frame rather
than the field, and why `SelectedIndex` sits on the select root. Anything
deeper needs a world walk owned by the widget's own module
(`set_select_max_visible`).

Design new composite widgets root-first, or the imm layer cannot drive
them.

## imm and retained are separate dialects

The two surfaces name things alike where there is no reason to differ:
`caption`, `separator`, `space`, `flex_spacer`, `icon` are the same word
on both, and a new pair should match unless it reads badly.

One divergence is settled and deliberate: containers are `row`/`column`
retained, `horizontal`/`vertical` in imm. Leave it. It also frees
`horizontal`/`vertical` to mean an *axis* everywhere else
(`split_horizontal`, `scroll_area_vertical`).

## Never write a scene component as a bare component

`@PlumeX` is the only way to spawn one: it inserts `PlumeX` *and* applies
`PlumeX::scene()`. Writing a bare `PlumeX` in a `bsn!` block is wrong in
both directions it can appear:

- **Inside `PlumeX::scene()` itself** it is redundant (the machinery has
  already inserted it) and it reads as if the marker were not
  guaranteed, contradicting the rule above.
- **Inside a shared piece** it is a genuine bug: the component lands on
  an entity whose scene never ran. `text_input_frame` did this with
  `PlumeTextInput`, so every number input claimed to be a text input.
  A piece two widgets share carries a plain marker, never either
  widget's scene component.

There is no field-setting exception, because a scene component carries no
state to set; see the rule above.

## Content slots stretch

Content slots keep the feathers `Stretch` default. A `Start`/hug default
breaks every `grow`-based row and leaves sections ragged. Per-widget
sizing is opt-in through the response builders instead.

# Behavior

## A scene declares its resting look

The style systems (`update_*_styles`) run in `PreUpdate`, so a control
spawned during `Update` is drawn once before its styler has seen it. Its
`bsn!` scene must be complete for the resting state: tokens, gradient,
cursor, text token. The styler only departs from that for hover, press,
disabled and checked.

When a control looks wrong for a frame, fix the scene, not the schedule.
The stylers write background, text color and cursor, which want three
different slots in the frame, so moving them swaps a one-frame spawn
artifact for a permanent one-frame lag.

## Close a popup with `close_popup`, never by despawning it

Closing usually happens in the middle of a cascade (a row click, a
dismiss press), and later commands in that cascade still target the
popup's entities. `close_popup` hides it at once and a `Last` system
despawns it; despawning it directly from an observer panics.

## Text sits under a surface that establishes its style

Font and text color propagate down to every descendant from the surfaces
that establish them: `screen`, dialogs and popups. Text spawned outside
one resolves neither, falls back to white in the engine font, and trips
the `warn_once!` in the theme module. Text that styles itself opts out
with `PlumeIgnore`.
