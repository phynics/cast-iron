---
title: "Corpus pinned to v0.1.34"
date: 2026-11-21
kind: pin
---

# Corpus pinned to v0.1.34, 2026-11-21

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.34`. The annotated tag object
is `652fcc2467207cca5d10a091c63f08d28acf9a75`; it peels to commit
`724016823d36061b6d53777a53acb31c2e7d53b4`, the head of `main` when the tag was made. The tree
hash is `67e174ab6612cb45b792fcc40d1c41956f784258d260444b017903e8b0f93808`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships two groups of
corpus changes on top of v0.1.33:

- **Class-option powers in the owner-approved layout** (data PRs #136 to #143 and #151,
  phynics/cast-iron-data#88; decisions `2026-11-20-class-option-power-layout`,
  `2026-11-21-class-option-power-counts` and `2026-11-21-class-option-power-owner-cases`). The
  rewrite script and the review queue (#136, #151), the rewrite in six family batches (#137 to
  #142) and the powers resolved from print (#143). The `Black Market Connections (Ex)` spelling
  follows the book (#142). The import residue `@B-5y1-cleric.` is gone from three entries (#143).
  No `id` changed and no file moved.
- **Creature defense blocks in labeled rows** (data PRs #144 to #149, phynics/cast-iron#254). Each
  block reads Armor class, Touch, Flat-footed, AC modifiers, Hit points, Fort, Ref, Will, then the
  other defense rows in their order. 2,897 blocks in 2,848 entries (a-c 553, d-f 567, g-l 641,
  m-r 461, s-z 626 entries); the typed `[fields]` are unchanged.

- **Class-option power check.** `cargo xtask corpus-check` reports 133 of 3,857 class-option powers
  in 122 of 2,080 entries that are not in the canonical form (about 2,035 on the v0.1.33 corpus). By reason: no effect row 65, more
  than one effect row 25, unlabeled paragraph not after the effect 22, out of order 8, legacy
  label 5, legacy upgrade 5, numbered effect label 2, second singleton row 1. All 133 are in the
  review queue of the corpus (`scripts/class-option-power-queue.tsv`). The decision makes the check
  an error at the pin of the rewritten corpus. **It stays a warning in this pin:** an error would
  fail the corpus check on the 133 queued powers. Enforcement waits until the queue is resolved
  (follow-up issue).
- **Corpus-check result.** `ok`, 0 errors. The eight residue guards (#548), the spell-list marker
  check (#539) and the footnote checks passed. 142 warnings by kind: 133 class-option powers (above)
  and one summary line of them; 1 summary of 2,021 book memberships in 695 entries without a
  `[[sources]]` row (#461); 1 summary of 338 unresolved trades in 182 entries with 1 provable match
  (Roughrider's `Armor training 4`, on its own line); 3 mixed third-party and Paizo source
  warnings (the shadow rats).
- **Counts.** 31,825 entries, 443 publications (439 books in the corpus check), unchanged. The
  pin edits existing entries and adds none. Links: 147,298 resolved, 0 out of selection, 0 broken.
  The library schema stays 6 and the search index format stays 2.
- **Share slots.** No new publication. `cargo xtask content` passed the book-slot guard and the
  published-id guard, so `cargo xtask published-books` and `published-ids` were not run.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`:
  only the four pin lines changed. The counts stay 12,484 of 31,825 entries with an AoN path and
  426 of 443 publications with a source page.
- **Specs.** No spec asserts creature defense rows (`Armor class`, `16/13/14`, `Mod:`) or the old
  class-option power text (`Gained:`, `Passive:`, `At Nth Level:`); `grep` in `tests/e2e/tests/`
  finds none. No spec changed.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the
  pinned corpus v0.1.34. No legal claim changed. `notice_pending` is unchanged.
- **Local verification** (nothing deployed), on `origin/main` at `e599eafaa` plus this pin.
  Content version `v24010454fb1d951c` (was `v65e2fb8a7e1ae499`). `build-site --public-url
  /cast-iron/` passed (shell `af83f0b85ee52618`). `size-budget`: library 224,603,695 of
  234,300,000 bytes (95.9%), largest file 69,022,907 of 72,300,000 (95.5%), WASM 3,408,711 of
  3,460,000 (98.5%); no limit raised. Spec `class-options.spec.ts` (the class option spec of the
  feature map) on `--project=chromium` and `--project=mobile-chromium`, one worker: 8 passed on
  each project. The full suite runs in batch CI on `main`.
