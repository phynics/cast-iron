---
title: "Corpus pinned to v0.1.32"
date: 2026-11-20
kind: pin
---

# Corpus pinned to v0.1.32, 2026-11-20

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.32`. The annotated tag object
is `82279dcc4186ea357f32b47eec1fe54c32ec1d72`; it peels to commit
`c265325ece61ef4fde220631a6865beef629472b`, the head of `main` when the tag was made. The tree
hash is `83560f950b7c6d8327a17fe5c9a69106af22507b3e17fb4a8c20b317ab963556`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships two corpus
changes on top of v0.1.31, both from the footnote and front-matter clean-up:

- **HTML entity references in front matter replaced** (data PR #126, issue #125). Of the 328 HTML
  entity references in 302 files, 249 were removed with their empty `weight` keys and 79 were
  replaced with plain characters (`&times;` to `×`, `&mdash;` inside a longer value to `—`). The
  typed weight field is now empty instead of the six characters `&mdash;`. Three item titles
  change (`Telescope (×10)`, `×50`, `×250`).
- **The last orphan footnote markers restored** (data PR #127, issue #124). 13 Bonus-feat legends
  that held `^B|param^` now hold `^B^`; four idols got `^S^` and Old-Mage Jatembe got `^F^`; and
  the `footnote-e` note is removed from Detect Chaos, Detect Fiendish Presence, Detect Good and
  Detect Law. The v0.1.32 tag annotation misstates the reason for the four removals, which
  [data PR #127](https://github.com/phynics/cast-iron-data/pull/127) records as follows:
  `footnote-e` copied Detect Evil's printed table note (*Core Rulebook* p. 266) into four spells
  that have no table for it to annotate. Each removal has an exclusion row.

- **Counts.** 31,825 entries and 443 publications and books, all unchanged: the pin edits
  existing entries and adds none. Reachability: 31,825 reachable, 0 unreachable. Links: 147,298
  resolved (unchanged), 0 out of selection, 0 broken. The library schema stays 6 and the search
  index format stays 2.
- **Share slots.** No new publication. `cargo xtask published-books` is not needed
  (`cargo xtask content` passed the book-slot guard), and `cargo xtask check-book-slots`
  reported all 443 slots kept. `cargo xtask published-ids` is for a published library and was not
  run; it belongs to the release record. `cargo xtask content` passed the published-id guard.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`
  and `.tsv`: the pin lines and the list hash changed, and the three telescope titles in the `.tsv`
  lost their `&times;`. The counts stay 12,484 of 31,825 entries with an AoN path and 426 of 443
  publications with a source page.
- **Corpus-check warnings.** `cargo xtask corpus-check` on the pinned corpus ends `ok` with
  20 warnings (was 42): 0 footnotes without a marker in the entry body (the orphan warning of
  #521; was 22), 0 front-matter entity warnings, 16 duplicate trade-feature warnings, 3 mixed
  third-party and Paizo source warnings and the one trade-resolution summary. #531 can now turn
  the footnote warnings into errors; this pin does not do that.
- **Spec change.** `footnotes.spec.ts` used Detect Law, which has no footnote after data #127. Its
  first test now uses the feat Expanded Summon Monster (*Monster Summoner's Handbook* p. 28), whose
  printed note ("This creature is summoned with the celestial template ...") is cited by markers in
  the lists of creatures. The assertions keep their intent: the Notes group closes the body, the
  note is smaller and in another colour than the rules text, the rules paragraphs stay in the body,
  and each marker links to the note and the note links back to each marker. The other tests
  are unchanged. Every other spell with a kept note has its marker in a table cell (Nightmare,
  Detect Evil, Major Creation, Polymorph Any Object, Prismatic Wall, Sacrifice, Call Spirit).
  A trial on Nightmare rendered the marker as a plain link without a back link, because the
  renderer gives a marker its note link and id only in paragraph text
  (`Renderer::paragraph_content`), so the spec does not use those entries. Refs #534.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the
  pinned corpus v0.1.32; the removal of the editor-added notes is dated to v0.1.31. No legal claim
  changed.
- **Legal.** `notice_pending` is unchanged.
- **Local verification** (nothing deployed), on `origin/main` at `622eaa6e8` plus this
  pin. Content version `vac36ce2cc1ac05cb` (was `v4192151b29702eab`). `cargo xtask content`
  passed the published-id guard and the book-slot guard. `cargo xtask check` passed (with
  `check-book-slots` and `aon-coverage --check`). `build-site --public-url /cast-iron/` passed
  (shell `d06317e4b655275e`). `size-budget`: library 223,246,449 of 234,300,000 bytes (95.3%),
  largest file 68,766,167 of 72,300,000 (95.1%), WASM 3,380,944 of 3,460,000 (97.7%); no limit
  raised. Spec on `--project=chromium`, one worker: `footnotes.spec.ts`, 7 passed. The full suite
  runs in batch CI on `main`.
