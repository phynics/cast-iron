---
title: "Corpus pinned to v0.1.21"
date: 2026-11-16
kind: pin
---

# Corpus pinned to v0.1.21, 2026-11-16

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.21`. The annotated tag object
is `79a404c531c20bd7508a4063f1212134a4feb993`; it peels to commit
`3b74a89da8a2985bd5e541c8f149129373bbb3de`. The tree hash is
`0f1f86f00e491060550ce23c793b5202d90a7ebd11e15963d623478c1b0b01d5`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The data corpus CI run
[43771444322](https://github.com/phynics/cast-iron-data/actions/runs/43771444322) succeeded for
this exact commit. The tag merges data #71, #72 and #73. #72 aliases the seven Core Rulebook race
chapters (Dwarves, Elves, Gnomes, Half-Elves, Half-Orcs, Halflings, Humans) to the typed
`ref/race/*` records, merges the one sentence only a chapter held (the Common sentence in
Human), makes each typed core record cite the Core Rulebook first, and resolves the Bestiary
lycanthrope section, so every entry of kind race carries a `race_group`. #71 and #73 restore the
printed verb in 98 "At Nth level:" rows.

The corpus holds 31,800 entries (was 31,807: the seven chapters are aliases now), 439
publications and 448 books. The Races list holds 75 entries in four groups: core 7, featured 16,
uncommon 15, other 37. The library schema stays 4 and the search index format stays 2. The
published-id guard passed: the seven chapter ids still resolve through `aliases.toml`.
`sources/published-ids.txt` does not grow until a library is published.

`corpus-check` now fails on a race without a `race_group`. Because the pin must pass that
check, the guard and this pin land together.

Local verification, 2026-11-16 (no CI or deployment claim; CI is pending):

| Check | Result |
|---|---|
| `generated/content` | Content version `v0b7dcae17bf56e8f`, schema 4, index format 2, 31,800 entries, corpus tag `v0.1.21` (was `v046e8483642b667c`) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,800 entries, 439 publications, 448 books, 1 licence |
| `cargo xtask content` twice | Byte-identical (`diff -r`): content tree `c5aeaf4188819701af0de5a76635e8acd3d5dee0a1b50164a94590aa1ca277c7`, reports tree `7cca55a00bf498fe93f8cbc63197d6935ea5cd2c9dbd8af2bca39e032354bf44` |
| `cargo xtask check` | Passed: fmt, clippy for native and WASM, workspace tests, WASM build |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `47039171cb10038d`, 11 files; `generated/content` and `dist/content` are identical |
| Chromium: reference, browse-groups | 50 passed |
| Chromium, one-off script (not committed): `#/browse/kind/race` | The four group chips add up to 75 (7 + 16 + 37 + 15) and the count line reads 75 races. Each of the seven core races appears once in the A to Z list |
| Chromium, one-off script: `#/entry/ref/carried/core-rulebook/races/dwarves` | Resolves to `#/entry/ref/race/dwarf`; heading Dwarf; meta line "Race · Core Rulebook · +16 more"; sources line "Core Rulebook p. 21; Inner Sea Races p. 236; Advanced Race Guide p. 10" |

**Limitations.** CI for this branch is pending, and no Pages deployment or live check happened
for this pin. Nothing was promoted, so `release` is unchanged.
