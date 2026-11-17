---
title: "Corpus pinned to v0.1.3"
date: 2026-11-06
kind: pin
---

# Corpus pinned to v0.1.3, 2026-11-06

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.3`, commit
`46e7cf538d3d2b8bc49850b112ba1acb446c3d18`, tree hash
`7d625e7b4a8ebe07b51046010f777109dacf75a8c03801a399e91cfe5e43f7fe`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). This pin supplies the facet data behind the class, item and creature lenses
([#195](https://github.com/phynics/cast-iron/issues/195)), and the CR drift check now compares
a creature's `cr` field with its `**Challenge rating:**` line
([#198](https://github.com/phynics/cast-iron/issues/198)).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v57b3e42733317ed3`, 31,106 entries (was `v377b3e08a8a0692a`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books, 1 licence; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `50ff6058afafd3d9`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/browse-groups.spec.ts tests/search.spec.ts` | 35 passed |

The lenses read from the search index in the built site:

| Lens | Values | Entries |
|---|---|---|
| Class, `class_type` (prestige skipped) | 7: Alternate 3, Base 10, Core 11, Hybrid 10, NPC 5, Occult 6, Unchained 4 | 49 |
| Item, `item_category` | 57, from Adventuring Gear 49 and Weapons 541 to Wondrous Items 2,614 and Miscellaneous Equipment 1,747 | 8,091 |
| Creature, `creature_type` | 26 chips for 13 types, because the corpus spells some types in two cases (`aberration` 182 and `Aberration` 2); `Intelligence` appears as a type | 3,241 |
| Creature, `cr` | 35, from `1/8` 14 through `1` 199 and `5` 256 to `30` 13 | 3,718 |
