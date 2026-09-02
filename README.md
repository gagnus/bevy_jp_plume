# bevy_jp_plume ("Plume")

A bevy_ui control framework for game debug UI. Forked from `bevy_feathers`
and diverging deliberately.

Original code MIT OR Apache-2.0 (see LICENSE-MIT / LICENSE-APACHE).

See CLAUDE.md for the design rules that distinguish Plume from feathers.

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
