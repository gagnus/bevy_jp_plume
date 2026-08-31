# bevy_jp_plume ("Plume")

A bevy_ui control framework for game editors.

Forked from `bevy_feathers`

Last reconciled with bevy `main` on **2026-07-16**, at **244e0e6ae**

Original code MIT OR Apache-2.0 (see LICENSE-MIT / LICENSE-APACHE).

See CLAUDE.md for the design rules that distinguish Plume from feathers.

## Fonts

Plume bundles its fonts as embedded assets, so they end up inside any binary
built against it. Their licenses are separate from the crate's MIT/Apache-2.0
and travel with anything you ship.

| Font | Copyright | License |
| ---- | --------- | ------- |
| Noto Sans, Noto Sans Mono | The Noto Project Authors | [SIL OFL 1.1](src/assets/fonts/NotoSans-LICENSE.txt) |
| Font Awesome 7 Free (Solid, Regular) | Font Awesome | [SIL OFL 1.1](src/assets/fonts/FontAwesome-LICENSE.txt) |
| Lucide | Lucide Icons and Contributors | [ISC](src/assets/fonts/Lucide-LICENSE.txt) |

All three licenses require that the copyright notice and license text
accompany redistribution. Font Awesome's icon artwork is additionally
CC BY 4.0, which asks for attribution, and a subset of Lucide's icons is
MIT (inherited from Feather; both notices are in its license file).
