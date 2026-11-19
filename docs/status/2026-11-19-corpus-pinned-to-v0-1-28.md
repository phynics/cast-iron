---
title: "Corpus pinned to v0.1.28"
date: 2026-11-19
kind: pin
---

# Corpus pinned to v0.1.28, 2026-11-19

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.28`. The annotated tag object
is `4961330461d9b7823fae5c3465759a13eda0f13a`; it peels to commit
`24d393a23a8d4edb45f2382f12cbf9fe40c831f2`, the head of `main` when the tag was made. The tree
hash is `a53c9b3f583a27f6f203f29c5c0df0527588b68f844f32cd8a4521ba803ee17f`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive, which `cargo xtask fetch-sources`
verified). The pin ships the corpus changes #106 and #108:

- **17 cleric domains** (data
  [#103](https://github.com/phynics/cast-iron-data/issues/103), PR #106). Artifice, Chaos,
  Community, Darkness, Death, Evil, Glory, Good, Knowledge, Law, Liberation, Magic, Nobility,
  Repose, Rune, Strength and Trickery ship as `ref/domain/*` entries, checked against print. The
  converter had rejected them. Nineteen `[[resolved]]` rows supersede their rejected rows, and the
  Death and Repose rows also supersede the excluded Pharasma-Friendly adjustments from the Golarion
  Day blog post, which now ship inside those two domains. The 13 subdomain records that point to
  these domains stay out as their own entries and keep their exclusion rows.
- **Protection and Animal** (data [#107](https://github.com/phynics/cast-iron-data/issues/107),
  PR #108). The usage text of the two domain powers matches the printed rounds per day.

- **Counts.** 31,825 entries (was 31,808), 443 publications and 443 books (unchanged).
  Reachability: 31,825 reachable, 0 unreachable. Links: 147,924 resolved (was 147,789), 0 out of
  selection, 0 broken. The library schema stays 6 and the search index format stays 2. Only the
  `class_option` kind changes, 2,802 to 2,819 (+17); `ref/domain/*` entries go from 18 to 35.
- **Share slots.** No new publication. `cargo xtask published-books` added 0 lines
  (443 books), and `cargo xtask check-book-slots --base origin/main` reported all 443 slots kept.
  `cargo xtask published-ids` is for a published library and was not run; it belongs to the
  release record.
- **Coverage report.** The report gains a "Resolved records" section with 27 rows (8 from #36,
  19 from #103). The 19 domain rows are 17 rejected rows and the 2 excluded Death and Repose rows.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`:
  12,484 of 31,825 entries with an AoN path (was 12,467; the 17 domains have one), 426 of 443
  publications with a source page. The `.tsv` did not change.
- **Legal.** `notice_pending` is unchanged: Hodgepodge, Kingmaker Player's Guide, Serpent's Skull
  Player's Guide.
- **Local verification** (CI not run, nothing deployed), on `origin/main` at `2a0f1e463` plus this
  pin. Content version `vc382c8d199d77e72` (was `v6bd1da0aee54f66d`). `cargo xtask content`
  passed the published-id guard and the book-slot guard. `cargo xtask check` passed (with
  `check-book-slots` and `aon-coverage --check`). `build-site --public-url /cast-iron/` passed
  (shell `5153cf3265a651a6`; `generated/` and `dist/` manifests identical). `size-budget`:
  library 223,617,626 of 234,300,000 bytes (95.4%), largest file 68,825,139 of 72,300,000
  (95.2%), WASM 3,353,197 of 3,460,000 (96.9%); no limit raised. `scripts/check-publication.py`
  on `dist/content`: 31,825 entries, 443 sources, 0 issues. Spec on `--project=chromium`, one
  worker: `class-options.spec.ts`, 8 passed (the spec the feature map of the `verify` skill selects
  for class-option content). The full suite runs in CI.
