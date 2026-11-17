---
title: "Corpus pinned to v0.1.19"
date: 2026-11-10
kind: pin
---

# Corpus pinned to v0.1.19, 2026-11-10

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.19`. The annotated tag object
is `b4dd9cc634864fc6af6ace24afa3bef1d146287c`; it peels to commit
`a68bc6d0964d394ba2f9498e177680500c22d76c`. The tree hash is
`4949a979638fee1c5f0c98d0fdedef8213819bd272974ae1b136142b0ff60d21`
(`cargo xtask corpus-tree-hash` on a source clone and on the `fetch-sources` cache, which
`cargo xtask fetch-sources` verified). The data corpus CI run
[42753841965](https://github.com/phynics/cast-iron-data/actions/runs/42753841965) succeeded
for this exact commit. The tag merges data #64 (typed Sources), #66 (five collapse marks),
#67 (AoN links), #69 (race groups) and #68 (replaces and alters keys).

The corpus holds 31,807 entries, 439 publications and 448 books. It has 25,895 entries with
29,628 typed source rows, 12,467 entry AoN links and 426 publication AoN links, 75 typed race
groups (7 core, 16 featured, 15 uncommon, 37 other), five collapse marks (Universal Monster
Rules in Bestiary 1 to 4, and Special Abilities in the Core Rulebook), and 3,917
replaces/alters keys in 1,144 entries (3,099 replaces keys in 1,058 entries and 818 alters
keys in 518 entries). The library schema stays 4 and the search index format stays 2.

The published-id guard passed for all 42,019 ids: 9,968 legacy PSRD ids and 32,051 `ref/`
ids. `sources/published-ids.txt` does not grow until a library is published.

Local verification, 2026-11-10 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v9e4cebf228faf94e`, schema 4, index format 2, 31,807 entries, corpus tag `v0.1.19` (was `v17a954e5c0f1d11c`) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,807 entries, 439 publications, 448 books, 1 licence |
| `cargo xtask content` twice from clean output | Byte-identical (`diff -r`): content tree `359372b0d4c804e6972542d27b6b1000fe10933e2add190e5bfc36f323a8b28f`, reports tree `a5c10009545f6d5863a02c1d6a08f171b88d5f06c8bea033ae39e17feeacb08f` |
| `cargo xtask check` | Passed: fmt, clippy for native and WASM, workspace tests, WASM build |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `ae0dda3243618194`, 11 files; `generated/content` and `dist/content` are identical |
| `python3 scripts/check-publication.py` | Exit 0; 0 issues; 1,686 named-material entries. The script states its screen is finite |
| Chromium: browse-groups, reference, books, search, active-books, continue-reading, relationships, class-options, accessibility-reading | 145 passed |
| Chromium: changes, mythic | 7 passed |
| mobile-chromium: class-options | 8 passed |
| mobile-chromium: mythic | 4 passed |
| mobile-chromium: mobile | 22 passed |
| Chromium: source-variants | 1 passed |

The Races spec no longer skips on a corpus without `race_group`. It fails unless the compiled
corpus carries the four groups, and its expected counts come from the compiled catalog and the
active-book set.

**Real browser upgrade, 2026-11-10.** Isolated Chromium profile on one local origin. The run
served the published Pages artifact from
[42683286107](https://github.com/phynics/cast-iron/actions/runs/42683286107), release `6365dba`
with shell `5464362442e5568f`, content `v17a954e5c0f1d11c`, schema 4 and corpus `v0.1.17`. It
installed that library, starred Abjuring Step and turned Ultimate Magic off. It then served this
build with shell `ae0dda3243618194`, content `v9e4cebf228faf94e`, schema 4 and corpus `v0.1.19`,
and used the app's own controls: app update, Reload, then the separate library download and
activation. The star, the inactive book and the bookmark survived.

Abjuring Step changed from a leading `Sources: Ultimate Combat pg 221` paragraph to a
description first and a typed `Sources: Ultimate Combat p. 221` line last. The Races filter
showed four chips with active-book counts 7, 16, 15 and 23. Other races is 37 corpus-wide; the
list counts active books. Bestiary Universal Monster Rules collapsed to one row with 89. An
offline reload repeated the entry, the citation, the star and the four race chips. The script is
`.cache/upgrade-check.mjs` (local, not committed).

**Limitations.** No PSRD library artifact was preserved, so the tag's PSRD upgrade script did
not run. The 9,968 legacy ids are covered by the published-id guard, which passed, and the
browser check covers the published canonical baseline only. Physical devices and the
accessibility check ([#12](https://github.com/phynics/cast-iron/issues/12)) remain open. CI is
pending for the pull request. No Pages deployment or live check happened for this pin. `release`
still serves `6365dba` and content `v17a954e5c0f1d11c`.
