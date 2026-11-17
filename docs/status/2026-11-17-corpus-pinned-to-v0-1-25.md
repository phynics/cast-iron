---
title: "Corpus pinned to v0.1.25"
date: 2026-11-17
kind: pin
---

# Corpus pinned to v0.1.25, 2026-11-17

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.25`. The annotated tag object
is `d293dc6ea9c24dc80bd67423aaee5e102ab2afed`; it peels to commit
`17a24be58973da097bdff73500a182b60b122ec7`. The tree hash is
`057d5b51d022bc97edc27c52ee06246510e402a2eee9922844607e2a8b54d8ce`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The tag holds data #87 (explicit anchors on the three Core Rulebook headings that Quick
reference links to, [#439](https://github.com/phynics/cast-iron/issues/439)), data #89 (the
Bloodrager's spell save DC uses Charisma, [#435](https://github.com/phynics/cast-iron/issues/435))
and data #90 (the 28 Bestiary 2 Universal Monster Rules placed in its printed chapter). Corpus
#91 and #92 (page restores) are not in this pin.

- **Counts.** 31,800 entries, 439 publications, 439 books and 39,159 placements, all unchanged
  from v0.1.24. Reachability: 31,800 reachable, 0 unreachable. Links: 147,603 resolved, 0 broken.
  The library schema stays 5 and the search index format stays 2.
- **Bloodrager.** The class card's Spellcasting row now reads `Arcane, spontaneous · Charisma`;
  `casting_prose` needed no change.
- **Quick reference.** The three heading anchors are explicit in the corpus with the ids that
  `quick.rs` already uses, so no URL changed.
- **Bestiary 2.** Contents lists `Universal Monster Rules` after Monsters A to Z, with the 28
  carried rules and Lycanthropic Empathy.
- **Local verification** (CI not run, nothing deployed). Content version `v46f539381d0000c3`
  (was `vf0fd727fee7b7e7a`). `cargo xtask content` passed the published-id and book-slot guards;
  `corpus-fmt --check`: 31,808 files, 0 unformatted; `corpus-check` passed. `cargo xtask check`
  and `build-site --public-url /cast-iron/` (shell `00cca45a2690a0c0`) passed. `size-budget`:
  library 223,062,475 of 234,300,000 bytes, largest file 68,677,002 of 72,300,000, WASM
  3,269,780 of 3,280,000 (no raise). Specs on `--project=chromium`, one worker: `class-card` and
  `books` (45 passed, including the new Bestiary 2 test and the Bloodrager assertion). The full
  suite, including `quick-reference`, runs in CI.
