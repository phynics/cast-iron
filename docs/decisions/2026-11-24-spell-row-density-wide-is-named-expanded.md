---
title: "The Wide spell row density is named Expanded"
date: 2026-11-24
status: current
superseded_by: []
issues: [651]
prs: []
area: ui
---

# The Wide spell row density is named Expanded, 2026-11-24

Refs [#651](https://github.com/phynics/cast-iron/issues/651). Renames a name in `2026-11-07-spell-row-density-compact-and-wide` and `2026-11-22-spell-row-density-is-a-text-toggle`; their text keeps the old word. The owner decided this on 2026-11-23: "Wide" is a misnomer for the layout.

- **What reads differently.** The layout that was `Wide` is `Expanded`. The list's toggle reads `Expanded rows` (and `Compact rows`), and the Settings radio under `Spell row density` reads `Expanded`. The default is still this layout.
- **The stored value stays `wide`.** `Density::Expanded.slug()` is `wide`, so `spell_density` in the `meta` store, the `data-density="wide"` hook and its CSS selectors are unchanged. A reader who chose the layout before the rename keeps the choice, and no migration or second read path exists. Only the visible label changed.
