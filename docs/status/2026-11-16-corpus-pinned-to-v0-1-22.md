---
title: "Corpus pinned to v0.1.22"
date: 2026-11-16
kind: pin
---

# Corpus pinned to v0.1.22, 2026-11-16

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.22`. The annotated tag object
is `a864a53d22653752c4139f2c89dd869452b2d775`; it peels to commit
`bd16c5642fc7dc98237b86065c1fcb460e34a07b`. The tree hash is
`057c5d29f4c1ac5f14a9510c952fd45b3a5ceb7f02c108362be166770ebeef73`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The data corpus CI run
[43883672996](https://github.com/phynics/cast-iron-data/actions/runs/43883672996) succeeded for
this exact commit. The tag holds the batches 3 to 9 of #390 (data #74, #75, #76, #77, #78, #80
and #82) and #81. The batches restore the printed verb in the verbless "At Nth level:" rows,
with corrections from print, and the summon monster and summon nature's ally rows now name the
spell and link to it. #81 repairs 1,629 broken HTML entity fragments.

The corpus still holds 31,800 entries, 439 publications and 448 books; this pin changes entry
body text only. The library schema stays 4 and the search index format stays 2. The
published-id guard and the reachability check passed in `cargo xtask content`.
`sources/published-ids.txt` does not grow for a text-only corpus change.

Local verification, 2026-11-16 (no CI or deployment claim; CI is pending):

| Check | Result |
|---|---|
| `generated/content` | Content version `va53c5afc9bcdf057`, schema 4, index format 2, 31,800 entries, corpus tag `v0.1.22` (was `v0b7dcae17bf56e8f`). Links: 147,599 resolved, 0 broken |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,800 entries, 439 publications, 448 books, 1 licence |
| `cargo xtask content` twice | Byte-identical content tree `de23261d33b6b66513f11349d263e3dad54895220bb99f57d0f97478415f4a01`. Reports tree of the second, clean run `51b3065fcd642d71e3e57159c36cab09642f5f8a49db0baece572d668354968a` (the first run's reports also held the `publication.json` that the screen below wrote) |
| `python3 scripts/check-publication.py` | 0 issues; 439 sources, 31,800 entries, 1,686 entries that name Community Use material |
| Entity fragments | `grep -E "&quot|&#[0-9]+;?"` over the compiled entry shards finds no match |
| `cargo xtask check` | Passed (exit 0): fmt, clippy for native and WASM, workspace tests, WASM build |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `87958f8ff9f4f2c3`, 11 files; `generated/content` and `dist/content` are identical |
| Chromium: `class-options`, `relationships`, `reference`, `search`, `journeys`, `browse-groups`, `variants` | 114 passed |
| mobile-chromium: `class-options`, `variants` | 11 passed |

No spec asserted the old broken text, so no assertion changed. A search of `tests/e2e/tests` for
`&quot`, `&amp`, `&#`, "At Nth level" and the summon spell names found nothing.

**Scope of the "75 races" count.** The v0.1.21 entry above counts every entry of kind `race` in
the whole compiled library, with every book active (no book is switched off). The 75 races have
11 canonical books, none of them third party, so the count does not change with the third-party
flag. By canonical book category, 27 are in Rulebooks (Core Rulebook 7, Bestiary 1, Bestiary 5 8,
Bestiary 6 5, Planar Adventures 3, Ultimate Wilderness 3), 38 in Campaign Setting (Inner Sea Races
35, Isles of the Shackles 1, Occult Bestiary 2), 9 in Player Companion (Blood of the Sea) and 1
in an Adventure Path (Dreams of the Yellow King). The first-run default (`fresh_install_disabled`)
turns on the Paizo Rulebooks tier only, so a new profile that keeps that default lists 27
races, not 75. The count is unchanged by this pin.

**Limitations.** CI for this branch is pending, and no Pages deployment or live check happened
for this pin. Nothing was promoted, so `release` is unchanged.
