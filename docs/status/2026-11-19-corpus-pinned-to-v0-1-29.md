---
title: "Corpus pinned to v0.1.29"
date: 2026-11-19
kind: pin
---

# Corpus pinned to v0.1.29, 2026-11-19

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.29`. The annotated tag object
is `70a66978b6c2b054af93a5529fb72c4ea75d7505`; it peels to commit
`d9bff95d097b0bef28df9cd7fd7071d1a816d964`, the head of `main` when the tag was made. The tree
hash is `c47f192bdfe7fb53430376d9d6b43ec0909c13a84f5d9ba103c66ce686bf916d`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships one corpus
change, PR #111:

- **Feat prerequisites agree with print** (data
  [#110](https://github.com/phynics/cast-iron-data/issues/110), PR #111). On 86 feat entries the
  prerequisites text and the `prerequisite_feats` list disagreed; both now follow the printed
  text. Shield Master and Shield Slam regain Two-Weapon Fighting. Rapid Shot, Far Shot, Manyshot
  and Shot on the Run lose a wrong Precise Shot link. Deflect Arrows loses a wrong Shield Focus
  link. Ledge Walker points at the dwarf racial trait. The corpus also gains
  `scripts/scan-feat-prerequisites.py`, which finds such disagreements.

- **Counts.** 31,825 entries and 443 publications and books, all unchanged: the pin edits
  existing entries and adds none. Reachability: 31,825 reachable, 0 unreachable. Links: 147,942
  resolved (was 147,924), 0 out of selection, 0 broken. The library schema stays 6 and the search
  index format stays 2.
- **Share slots.** No new publication. `cargo xtask published-books` added 0 lines
  (443 books), and `cargo xtask check-book-slots` reported all 443 slots kept.
  `cargo xtask published-ids` is for a published library and was not run; it belongs to the
  release record.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`
  with the new pin lines only: the counts stay 12,484 of 31,825 entries with an AoN path and
  426 of 443 publications with a source page. The `.tsv` did not change.
- **Legal.** `notice_pending` is unchanged.
- **Local verification** (CI not run, nothing deployed), on `origin/main` at `0852e9cdb` plus this
  pin. Content version `vb23c2dd0a96f4c21` (was `vc382c8d199d77e72`). `cargo xtask content`
  passed the published-id guard and the book-slot guard. `cargo xtask check` passed (with
  `check-book-slots` and `aon-coverage --check`). `build-site --public-url /cast-iron/` passed
  (shell `71dac816e7525c7c`; `generated/` and `dist/` manifests identical). `size-budget`:
  library 223,635,065 of 234,300,000 bytes (95.4%), largest file 68,825,389 of 72,300,000
  (95.2%), WASM 3,380,602 of 3,460,000 (97.7%); no limit raised. The WASM grew from 96.9%
  because of client changes already on `main`, not the pin. `scripts/check-publication.py` on
  `dist/content`: 31,825 entries, 443 sources, 0 issues. Spec on `--project=chromium`, one
  worker: `relationships.spec.ts` with the feat patterns of the feature map in the `verify` skill
  (`feat pages|category hub|every feat chain`), 3 passed. The full suite runs in CI.
