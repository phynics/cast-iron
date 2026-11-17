---
title: "Corpus pinned to v0.1.2"
date: 2026-11-06
kind: pin
---

# Corpus pinned to v0.1.2, 2026-11-06

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.2`, commit
`7739e7add3f2a6ccd1c5232377bd910d38278992`, tree hash
`44f014cb44d2ab033d9069cbfd4c30bad28bfc3e99dae6cd4f3e1e63e16de29f`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). Drift between typed fields and body text is now an error rather than a
warning, so `corpus-check` and `cargo xtask content` fail on any disagreement
([#180](https://github.com/phynics/cast-iron/issues/180)).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v377b3e08a8a0692a`, 31,106 entries (was `vc01cc1c20a652c37`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `6d54314fc71e9ae9`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/relationships.spec.ts` | 9 passed |
