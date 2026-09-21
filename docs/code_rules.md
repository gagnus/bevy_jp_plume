# Code Rules

Generic Rust + Bevy code rules. This file is duplicated across projects;
the canonical copy lives wherever was edited most recently. Propagate
changes by hand for now.

## Development Philosophy

Work in small, verified increments. After each change: `cargo check`,
then run the app, then visually confirm. Never proceed to the next
piece while the current piece is broken, unverified, or has obvious
rough edges. If an attempt gets tangled, revert to the last clean
state rather than pushing on.

## Testing Philosophy

The default is: don't write a test. A test that doesn't catch a real
regression is net negative — it has to be maintained, it can give false
confidence, and it adds noise to failures.

Pure-functional rule combinations with subtle interactions where a wrong
result would be silent are the right place for tests. Asset loading,
rendering output, UI layout, anything that's immediately obvious when
you run the app — not the right place.

## Descriptive names over short or generic ones

Prefer names that say what the thing *is*. Single letters (`i`, `n`,
`v`) and generic words (`index`, `count`, `value`, `data`, `tmp`,
`result`) are almost always worse than a name that tells the reader what
is being counted or held.

Acceptable exceptions:

- Idiomatic math: `x, y, z` for coordinates, `u, v` for UVs, `dx/dy`
  for deltas, `n` for a surface normal when context is unambiguous.
- Closure arguments used on the very next line when the element type
  is already obvious (`parts.iter().map(|part| part.id)` is fine;
  `|p| p.id` is not).

## Newtype IDs over type aliases

Wrap IDs in a tuple struct so the compiler keeps them distinct —
`struct RoomId(usize)`, not `type RoomId = usize`. A type alias is
interchangeable with every other alias of the same underlying type, so
nothing stops you passing a `RoomId` where a `KeyId` was meant; the
newtype makes that a compile error.

## No bare `unwrap()`

Use `expect("reason")` and write a message that explains the invariant
that was violated, not just the variable name. Bevy's `Single<...>`
system param and `Query::single()` panic loudly enough that we don't
need the same discipline there, but anywhere we'd reach for `.unwrap()`
on an `Option` or `Result`, use `expect` instead.

## Use `..Default::default()` for partial struct construction

```rust
Thing {
    a: Some(...),
    b: Some(...),
    ..Thing::default()
}
```

Not a field-by-field copy of every default — the diff against
`default()` is what makes the call site readable.

## Don't duplicate logic

If two pieces of code do the same thing, extract a shared function. The
second copy is the signal to refactor, not the third.

## Nest single-use helpers

A helper used in exactly one place lives as a nested function inside
its caller, not at module scope. Module-level functions imply "called
from elsewhere"; nesting signals "private to this routine".

```rust
fn outer(...) {
    fn helper(...) -> ... { ... }
    ...
}
```

## Avoid cloning where possible

Sometimes it's the only option. But destructuring or borrowing
temporarily is usually better than `.clone()`. When a clone is
intentional, leave a one-line comment at the call site.

## Comments

**Every `.rs` file** gets a `//!` at the top, max 2 lines: what the file
is. A `lib.rs` may add a one-line "how to use this module" (e.g. "Add
`FooPlugin` and spawn `Foo` entities.") — the reader of a crate root
*is* the caller. History and process are still off-limits.

**Every `pub` type, trait, and free function** gets a `///`, max 2 lines.
`pub(crate)` and `pub(super)` items use `//` like private items — they are internal details, not API.
Methods too, unless both the verb *and* the subject are clear from the
name alone (`with_grid`, `iter` on an obviously-named type). Never
document icon/glyph constant tables — the name is the documentation.

**Inline `//`:** default to silence. Add one only when the WHY is
non-obvious: a constraint, an invariant, a cross-module link, a
workaround. Max 2 lines; never a paragraph.

**Test body:** one line stating the invariant, only when the test name
doesn't already say it.

**Keep comments up to date.** When the code changes, the comment changes
with it or gets deleted. Prefer deleting: a stale comment is worse than
none.

**Always cut:**

- Anything derivable from the names and the 3 lines around it.
- History or process ("after the migration", "Phase 1 of…").
- On `///`: naming specific callers ("called by X", "used in Y system") or
  what parameters/return types already express.
- Filler framing ("Important:", "Note that:", "Mutable runtime state
  for the pipeline." before a struct named exactly that).
- Stale cross-references — if `Foo::bar` no longer exists, delete the
  comment, not just the link.

## Run `cargo +nightly fmt` after every edit

Always. Keeps diffs clean.

The `+nightly` is required, not a preference: `rustfmt.toml` sets
`unstable_features = true` to get `group_imports` and
`imports_granularity`, and stable rustfmt silently ignores both. A plain
`cargo fmt` therefore leaves imports unsorted and produces a diff against
whatever the next `cargo +nightly fmt` writes.

# Bevy-specific naming

## No `Component` or `Resource` suffix

Components and resources shouldn't carry their kind in their name:
`PlayerController`, not `PlayerControllerComponent`; `Settings`, not
`SettingsResource`. The `#[derive(Component)]` and the call site
(`Res<Settings>`) already say what the thing is.

## `_entity` suffix for `Entity` variables

When a variable holds an `Entity`, suffix it `_entity` whenever the
name would otherwise be ambiguous with a higher-level concept:
`armature_entity`, `player_entity`, `target_entity`. In obvious
one-line contexts (`for child in children.iter()`), the suffix is
overkill — `Children` only yields `Entity` and the reader can see it.

## `_handle` suffix for `Handle` variables

When a variable holds an asset handle, suffix it `_handle` whenever the
name would otherwise be ambiguous with a higher-level concept.

## `query_` prefix for `Query` system params

System parameters of type `Query<...>` use a `query_` prefix:
`query_players`, `query_pending_armatures`. Helps readers spot the ECS
fan-out at a glance. Doesn't extend to `Res`, `EventReader`,
`Commands`, or other system params — there's no equivalent ambiguity.

# Clippy

## `type_complexity` and `too_many_arguments` are accepted in systems

Bevy systems naturally produce gnarly `Query<...>` types and long
parameter lists — that's the framework, not a smell. Don't restructure a
system, bundle params into an ad-hoc struct, or add a type alias purely
to silence `clippy::type_complexity` ("very complex type used") or
`clippy::too_many_arguments`. Leave the system idiomatic. Reach for a
`#[derive(SystemParam)]` bundle only when it groups params that are
genuinely a unit and improves readability on its own merit — never just
to appease the lint.

Suppress these two with an `#[allow(...)]` attribute, not by reshaping
the code:

- `clippy::type_complexity` — allow crate-wide as an inner attribute at
  the top of `lib.rs` / `main.rs` (`#![allow(clippy::type_complexity)]`),
  since it's pervasive. This matches what Bevy's own crates do.
- `clippy::too_many_arguments` — allow per-system
  (`#[allow(clippy::too_many_arguments)]` on the fn), since it's the rare
  exception, not the rule.

## Fix every other clippy lint

These two are the only ones we silence. Everything else clippy reports is
a real, cheap improvement (`map_or` → `is_none_or`, needless clones,
redundant closures, …) — fix it, don't `#[allow]` it. If some other lint
later turns out to genuinely fight Bevy idiom, we add it to the list
above then, case-by-case — never pre-emptively.
