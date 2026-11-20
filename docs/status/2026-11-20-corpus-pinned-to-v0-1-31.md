---
title: "Corpus pinned to v0.1.31"
date: 2026-11-20
kind: pin
---

# Corpus pinned to v0.1.31, 2026-11-20

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.31`. The annotated tag object
is `0941cd2df17dd4c9bd9f953e9edad18c11f81eaa`; it peels to commit
`4ec8d26683ea09b7aaba2336935dcddd179ea264`, the head of `main` when the tag was made. The tree
hash is `5962f73867b220a5c0f8b5601213a48020282b19d649a63e5edff86fe5b6fbe9`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships four corpus
changes, all from the editorial footnotes audit
([data #109](https://github.com/phynics/cast-iron-data/issues/109)) and the owner decision that
[a footnote stays when AoN, the PRD or the printed book carries it](../decisions/2026-11-20-printed-footnotes-stay.md):

- **LICENSE.md names the notes that stay** (data PR #119). The Pathfinder Data notes that stay and
  their basis are named in the corpus `LICENSE.md`, which no longer says that none of the dataset's
  application content was copied.
- **Markers of kept footnotes are restored** (data PR #120, issue #116). It also removes the note
  on Detect Law that was a copy of a note from the detect evil table.
- **Placeholder Range line dropped** (data PR #122). Detect Relations keeps `Range: N/A`;
  Torrent of Elemental Rage loses its `Range` line.
- **Editorial footnotes removed** (data PR #123, issue #117). 60 footnotes that only the dataset's
  editor added (40 pointers, 15 commentary notes, 5 compiled lists) and 15 editorial paragraphs
  are removed, each with an exclusion row. No note of unclear origin is removed: those notes were
  restored as printed.

- **Counts.** 31,825 entries and 443 publications and books, all unchanged: the pin edits
  existing entries and adds none. Reachability: 31,825 reachable, 0 unreachable. Links: 147,298
  resolved (was 147,420, because the removed notes held links), 0 out of selection, 0 broken. The
  library schema stays 6 and the search index format stays 2.
- **Share slots.** No new publication. `cargo xtask published-books` added 0 lines
  (443 books), and `cargo xtask check-book-slots` reported all 443 slots kept.
  `cargo xtask published-ids` is for a published library and was not run; it belongs to the
  release record. `cargo xtask content` passed the published-id guard.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.md`
  with the new pin lines only: the counts stay 12,484 of 31,825 entries with an AoN path and
  426 of 443 publications with a source page. The `.tsv` did not change.
- **Corpus-check warnings.** `cargo xtask corpus-check` on the pinned corpus ends `ok` with
  42 warnings (was 130): 22 footnotes without a marker in the entry body (the orphan warning of
  #521; was 110), 16 duplicate trade-feature warnings, 3 mixed third-party and Paizo source
  warnings and the one trade-resolution summary.
- **Spec change.** `footnotes.spec.ts` expected two notes on Detect Law. #120 removed the note that
  began "Some characters who are not clerics", so the entry keeps one printed note ("Excludes
  undead and outsiders"). The spec now expects one note, labelled "Note 1". The other assertions
  are the same: the Notes group, no marker or back link, smaller type and a different colour than
  the rules text, the rules text unchanged.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` described
  the pinned corpus as still holding the editor-added notes and the old `LICENSE.md` sentence;
  they now describe the v0.1.31 state and link the decision.
- **Legal.** `notice_pending` is unchanged.
- **Local verification** (CI not run, nothing deployed), on `origin/main` at `030dc8893` plus this
  pin. Content version `v4192151b29702eab` (was `v867a87d7bb3ce27d`). `cargo xtask content`
  passed the published-id guard and the book-slot guard. `cargo xtask check` passed (with
  `check-book-slots` and `aon-coverage --check`). `build-site --public-url /cast-iron/` passed
  (shell `d06317e4b655275e`). `size-budget`: library 223,249,835 of 234,300,000 bytes (95.3%),
  largest file 68,766,596 of 72,300,000 (95.1%), WASM 3,380,944 of 3,460,000 (97.7%); no limit
  raised. Spec on `--project=chromium`, one worker: `footnotes.spec.ts`, 7 passed. The full suite runs in CI.
