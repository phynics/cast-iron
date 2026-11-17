---
title: "Reviewed supplemental source import"
date: 2026-11-04
kind: record
---

# Reviewed supplemental source import

Local branch `data/18-pf1e-corpus-migration` adds a native adapter for the pinned
`jasontankapps/pathfinder-data-1-e` snapshot and imports one reviewed record:
`pf-data/rule/magic`, "Magic and Spellcasting". It is the missing Core Rulebook
page 206 opening passage and is placed first in the existing Core Rulebook Magic
article. Its permanent ID uses the native protocol namespace; all earlier IDs and
titles are unchanged. Every other primary-source record stays in coverage as
`record shape ... is not approved`, not in the library.

Content version `vff56b0e6310710c1` has 9,968 entries (was 9,967), schema 3 and
search format 1. Raw content is 62,123,015 bytes (was 61,956,172); the search
index is 20,330,948 bytes (was 20,329,611). Resolved links are 1,310, including
the six reviewed links from the new passage. The legal page lists the new dataset
pin and appends the pinned source's complete Section 15 notices. Supplemental
coverage rows carry a `pathfinder-data-1-e:` prefix, so they cannot be read as
PSRD-Data paths.

Local verification: `cargo xtask check` passes. Two `cargo xtask content` runs
produce the same tree hash. `scripts/check-library-upgrade.py dist/content
generated/content` reports no missing IDs, no changed titles or kinds, and no
dangling references. The offline Chromium check `Reviewed primary-source passage
appears under the Core Rulebook Magic article` passes against the real `dist/`
build. `cargo xtask data-report` is reproducible and its tracked reports are
committed; the full-corpus experiment remains blocked and is not shipped.
No CI, deployment, or full browser-matrix result is claimed for this content
change. Apollyon Clasp and the other unreviewed comic and blog material remain
excluded. Licensing decisions are tracked in
[#34](https://github.com/phynics/cast-iron/issues/34). Two PF2 publications are
registered in the pinned source (`Monster Core`, `Bestiary 3 (2nd Edition)`);
eight tagged records, the `main.json#ogc` ORC notice, and the untagged PF2
*Player Core* description in `gnome_flick_mace` are explicitly out of scope and
cannot enter the PF1e path. Comics are ignored permanently; blog records are
deferred to [#36](https://github.com/phynics/cast-iron/issues/36). The PSRD-Data
dataset-license risk is overridden for now and tracked in #34.
