---
title: "Corpus pinned to v0.1.4"
date: 2026-11-07
kind: pin
---

# Corpus pinned to v0.1.4, 2026-11-07

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.4`, commit
`cd8f9b3656bba069f3a1a4728649d1a585056442`, tree hash
`77ef95c353e25d342d55e5a7405554f58beabf2d558aa7b718a36ab48035827e`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). The pin normalises every creature's `creature_type` to one of the 13 PF1e
spellings, so the Creatures lens shows 13 chips with readable two-word labels, and carries
the markup, spell-link and raw-link fixes in corpus v0.1.4
([#211](https://github.com/phynics/cast-iron/issues/211)).

Local verification, 2026-11-07 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v2189dcc7f6c8b730`, 31,106 entries (was `v57b3e42733317ed3`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books, 1 licence; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `3f17489e52a0c2ac`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/browse-groups.spec.ts tests/search.spec.ts` | 49 passed |
| Creature lens on the built site | 13 chips, including “Magical Beast” and “Monstrous Humanoid”; no `magicalBeast`, `monstrousHumanoid`, or `Intelligence` |
