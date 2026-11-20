---
title: "Corpus pinned to v0.1.33"
date: 2026-11-21
kind: pin
---

# Corpus pinned to v0.1.33, 2026-11-21

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.33`. The annotated tag object
is `12e3222284e714acc3cb8bcf4a2d63e3998c0d16`; it peels to commit
`dc3e2020ff439f1d1894de5ff3f9313d25f381f0`, the head of `main` when the tag was made. The tree
hash is `39cc196f8693bdd57cfa59307203fa47844b674218c0f61ef0923fee9400958b`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships three groups of
corpus changes on top of v0.1.32:

- **Monster spell-list markers written as the printed wording** (data PR #128, closes data issue
  #79). 396 shorthand markers (`&Q spell` 204, `&S level N; ...` 165, `&EM spell` 21, `&EX spell` 6)
  and 8 plain prefixes (`&maximized`, `&reach` and four more) in the spell-like ability and
  spell lists of 317 monster files now read `quickened spell`, `summon (level N, ...)` and so on.
- **First pin in corpus format 2: trades migrated and the review queue settled** (data PRs
  #129 to #133, phynics/cast-iron#452). `corpus.toml` says `format = 2`. 1,308 archetype and
  class option files hold one flat `trades` list. Of 5,905 trades, 5,567 name a class feature and
  338 in 182 entries stay unresolved, each with a documented reason (the step convention is in
  `docs/corpus-format.md`, #541). The review queue has 1,366 rows: 841 resolved, 525 explicit
  exceptions, 0 open. Exceptions by code: variant-only 187, base-is-not-a-class 151,
  class-statistic 51, free-text-part 40, composite 39, repeated-item 19, overlaps-earlier-claim 16,
  no-class-heading 7, not-a-class-feature 7, single-slot-loss 3, levels-overlap 2,
  conditional-replacement 1, print-ambiguity 1, print-contradicts-class 1. No `id` changed and no
  file moved.
- **Book memberships the books do not print** (data PR #135, phynics/cast-iron#461 batch 4).
  114 memberships removed in 110 entry files (97 of them an entry's first book: spells such as
  Communal Air Walk listed under the Core Rulebook, which cannot print them; the next book now
  comes first) and one more turned into an `expanded_in` credit (Distant Shores p. 12).
  `books.toml` loses the matching chapter lines.

- **Format 2 end to end.** The reader accepts it (`READABLE_FORMATS` is 1, 2 and 3). `cargo xtask
  content` took the strict path for format 2 (`attached::trades`), lowered every trade, and wrote
  `trades.json` (581,702 bytes): 1,308 entries, 5,905 trades, 5,567 with a `feature`, 338 without.
  The entry shards hold no trades. The UI reads trades only for the printed `Replaces ...` line of
  an archetype or class option (`class_options::replaces_line_of`, `views/related.rs`), which needs
  no resolved feature, so the 338 unresolved trades show their printed names. Schema stays 6; no
  code assumed format 1 for this to compile and render.
- **Counts.** 31,825 entries and 443 publications, unchanged (439 books in the corpus check). The
  pin edits existing entries and adds none. Reachability: 0 unreachable. Links: 147,298 resolved
  (unchanged), 0 out of selection, 0 broken. The library schema stays 6 and the search index
  format stays 2.
- **Share slots.** No new publication. `cargo xtask content` passed the book-slot guard, so
  `cargo xtask published-books` is not needed. `cargo xtask published-ids` is for a published
  library and was not run; it belongs to the release record. `cargo xtask content` passed the
  published-id guard.
- **AoN coverage.** `cargo xtask check` reported the report stale; `cargo xtask aon-coverage`
  regenerated `docs/research/aon-path-coverage.md`: only the four pin lines changed. The `.tsv` is
  unchanged. The counts stay 12,484 of 31,825 entries with an AoN path and 426 of 443 publications
  with a source page.
- **Corpus-check warnings.** `cargo xtask corpus-check` on the pinned corpus ends `ok` with 6
  warnings and no error: one summary of 2,021 book memberships in 695 entries without a
  `[[sources]]` row (was 2,040 in 701; #461), one summary of 338 unresolved trades in 182 entries
  with 1 provable match (Roughrider's repeated `Armor training 4`, listed on its own line), and 3
  mixed third-party and Paizo source warnings (the shadow rats). No unresolved-footnote, entity
  or class-option-power warning.
- **Specs.** No spec asserts a membership that no longer exists: none of the 110 changed entries,
  nor the removed `books.toml` chapter entries, appears by id or title in `tests/e2e/tests/`. No
  spec changed.
- **Known issue before the release that carries this pin.** phynics/cast-iron#543: the Continue
  Reading point can go stale after a membership removal, so a reader whose saved point is in a
  removed membership gets a stale point. Fix it before this pin is promoted.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the
  pinned corpus v0.1.33. No legal claim changed.
- **Legal.** `notice_pending` is unchanged.
- **Local verification** (nothing deployed), on `origin/main` at `27fb428f9` plus this pin.
  Content version `v65e2fb8a7e1ae499` (was `vac36ce2cc1ac05cb`); `cargo xtask content` passed the
  published-id guard and the book-slot guard. `build-site --public-url /cast-iron/` passed (shell
  `935ae8004d9dc2d0`). `size-budget`: library 223,325,364 of 234,300,000 bytes (95.3%), largest
  file 68,767,438 of 72,300,000 (95.1%), WASM 3,381,262 of 3,460,000 (97.7%); no limit raised.
  Spec on `--project=chromium`, one worker: `class-sections.spec.ts` (the archetype and class page
  spec of the feature map; its archetype list is built by `views/related.rs`, which loads
  `trades.json`), 29 passed; it is not configured for `mobile-chromium`. No spec asserts a trade
  or the `Replaces` line. The full suite runs in batch CI on `main`.
