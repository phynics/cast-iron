---
title: "Corpus pinned to v0.1.23"
date: 2026-11-16
kind: pin
---

# Corpus pinned to v0.1.23, 2026-11-16

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.23`. The annotated tag object
is `44c5989e80f3d28b953a5089ca3517ff4ac97784`; it peels to commit
`0087039684076b7c55ef5e6fee6f1b816612714a`. The tree hash is
`f5abe7f25c09a7930db09cecbd2f80fc260a67767850dc45ac0a2845b53b431f`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The data corpus CI run
[43953279910](https://github.com/phynics/cast-iron-data/actions/runs/43953279910) succeeded for
this exact commit. The tag holds data #83 and data #84.

- Data #83 corrects 228 spell entries whose levels and other fields had been copied from their
  `variant` target (Summon Nature's Ally II to IX, Summon Monster II to IX and others). Each
  spell now sits at its own printed level on the class spell lists and in the base spell's
  Variants panel (#402).
- Data #84 is the last batch of #390. The verbless-row scan
  (`scripts/scan-verbless-level-rows.py` in the data corpus) finds 0 rows in 0 files on this
  commit (1,321 rows in 278 files when batch 2 began; the issue's 1,220 rows in 256 files came from an earlier scan that missed rows whose value ends in a unit and rows ending in `+2d6` or `20%`, and the fixed scan also dropped 65 rows that already carry a verb).

The corpus still holds 31,800 entries, 439 publications and 448 books; this pin changes entry
levels, fields and body text only. The library schema stays 4 and the search index format stays
2. The published-id guard and the reachability check passed in `cargo xtask content`.
`sources/published-ids.txt` does not grow for this change.

Local verification, 2026-11-16 (no CI or deployment claim; CI is pending):

| Check | Result |
|---|---|
| `generated/content` | Content version `v65d35c593e7620bd`, schema 4, index format 2, 31,800 entries, corpus tag `v0.1.23` (was `va53c5afc9bcdf057`). Links: 147,603 resolved, 0 broken |
| `python3 scripts/check-publication.py` | 0 issues; 439 sources, 31,800 entries, 1,686 entries that name Community Use material |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,800 entries, 439 publications, 448 books, 1 licence |
| `cargo xtask corpus-fmt --check .cache/sources/cast-iron-data` | Exit 0: 31,808 files, 0 unformatted |
| Verbless-row scan on the pinned corpus | 0 rows in 0 files |
| `cargo xtask check` | Passed (exit 0): fmt, clippy for native and WASM, workspace tests, WASM build |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `64ecde687d84d9fe`, 11 files; `generated/content` and `dist/content` manifests are identical |
| Chromium: `relationships` (with the new #402 test), `variants`, `class-options`, `reference`, `search`, `journeys`, `browse-groups` | 115 passed before `main` merged #414; after the merge, `relationships`, `journeys` and `reference`: 29 passed |
| mobile-chromium: `class-options`, `variants` | 11 passed |

The first browser run used a build of `main` at `2446d8c` plus this change. After Muse's review, `main` (with #414) was merged into the branch, the site was rebuilt (shell `485cb6305dbc98cf`) and `relationships`, `journeys` and `reference` ran again.

The new test in `relationships.spec.ts` ("numbered spell series list each member at its own
level") checks that Summon Nature's Ally II is on the Druid list at level 2 and not at level 1,
that V is at level 5 and IX at level 9, that Summon Monster II is on the Wizard list at level 2 and not at level
1, and that the Variants panel of Summon Nature's Ally I shows each variant's own levels. No
existing spec asserted an old wrong level, so no assertion changed.

**Limitations.** CI for this branch is pending, and no Pages deployment or live check happened
for this pin. Nothing was promoted, so `release` is unchanged.
