---
title: "Corpus pinned to v0.1.30"
date: 2026-11-20
kind: pin
---

# Corpus pinned to v0.1.30, 2026-11-20

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.30`. The annotated tag object
is `f801c2e708cf3ecf3cc3c31ccb1e4a99498a0cf5`; it peels to commit
`fb77db5d9e7ee4bb7cedde53b4b295095d560044`, the head of `main` when the tag was made. The tree
hash is `5d3c0cecbe745cf0a1f8f13ce772310210403b961e2fa2e9c5edcde1ab827a1e`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships two corpus
changes, both from the editorial footnotes audit
([data #109](https://github.com/phynics/cast-iron-data/issues/109)):

- **Page citations become sources** (data
  [#112](https://github.com/phynics/cast-iron-data/issues/112), PR #113). 438 page-citation
  footnotes in 70 entries are now structured `[[sources]]` rows, and the citation blocks and
  their markers are gone: 323 new rows, 49 existing rows extended with a page, 79 rows that
  already carried the page. 4 citations cannot be checked (Pathfinder #30 and #43 are not
  available for a print check) and are recorded in `exclusions.toml`.
- **Notes that no source carries are removed** (data
  [#114](https://github.com/phynics/cast-iron-data/issues/114), PR #115). 81 footnote blocks and
  60 editorial paragraphs that neither Archives of Nethys nor the PRD carries are removed, each
  with an exclusion row (143 rows). 3 blocks with converter residue are split into one block per
  note, 27 duplicate footnote anchors in 6 entries are renamed, and 20 dangling markers are
  stripped. Summon Monster II no longer has notes.

- **Counts.** 31,825 entries and 443 publications and books, all unchanged: the pin edits
  existing entries and adds none. Reachability: 31,825 reachable, 0 unreachable. Links: 147,420
  resolved (was 147,942, because the removed notes held links), 0 out of selection, 0 broken. The
  library schema stays 6 and the search index format stays 2.
- **Share slots.** No new publication. `cargo xtask published-books` added 0 lines
  (443 books), and `cargo xtask check-book-slots` reported all 443 slots kept.
  `cargo xtask published-ids` is for a published library and was not run; it belongs to the
  release record.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`
  with the new pin lines only: the counts stay 12,484 of 31,825 entries with an AoN path and
  426 of 443 publications with a source page. The `.tsv` did not change.
- **Corpus-check warnings.** `cargo xtask corpus-check` on the pinned corpus ends `ok` with
  130 warnings: 110 footnotes without a marker in the entry body (the orphan warning of #521), 0
  converter-residue warnings, 16 duplicate trade-feature warnings, 3 mixed third-party and Paizo
  source warnings and the one trade-resolution summary. The 110 orphans are notes the corpus
  keeps (mostly monster stat-block notes, plus the Detect spells, Minor Creation and Type G);
  they render in the Notes group without a marker to link back to.
- **Spec change.** `footnotes.spec.ts` tested the missing-marker case on Summon Monster II,
  whose notes #115 removed. It now tests Detect Law, which keeps two printed notes without
  markers. The assertions are the same: two notes in the Notes group, no markers or back links,
  smaller type and a different colour than the rules text, the rules text unchanged. The labels
  are "Note 2" then "Note 1", the order and numbers the printed notes have.
- **Legal.** `notice_pending` is unchanged.
- **Local verification** (CI not run, nothing deployed), on `origin/main` at `6a188226d` plus this
  pin. Content version `v867a87d7bb3ce27d` (was `vb23c2dd0a96f4c21`). `cargo xtask content`
  passed the published-id guard and the book-slot guard. `cargo xtask check` passed (with
  `check-book-slots` and `aon-coverage --check`). `build-site --public-url /cast-iron/` passed
  (shell `d06317e4b655275e`; `generated/` and `dist/` manifests identical). `size-budget`:
  library 223,287,433 of 234,300,000 bytes (95.3%), largest file 68,777,534 of 72,300,000
  (95.1%), WASM 3,380,944 of 3,460,000 (97.7%); no limit raised. `scripts/check-publication.py`
  on `dist/content`: 31,825 entries, 443 sources, 0 issues. Spec on `--project=chromium`, one
  worker: `footnotes.spec.ts`, 7 passed. The full suite runs in CI.
