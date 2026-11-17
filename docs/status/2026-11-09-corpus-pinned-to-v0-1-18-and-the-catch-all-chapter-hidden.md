---
title: "Corpus pinned to v0.1.18, and the catch-all chapter hidden"
date: 2026-11-09
kind: pin
---

# Corpus pinned to v0.1.18, and the catch-all chapter hidden, 2026-11-09

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.18`, commit
`592d0ac2445036464a4a4c7ed386181b9a4ba339`, tree hash
`e9dc5532ac0972427fbea6a8a0a66d1ee993e396ec2e1ace9ffe5728d1f05dfc`
(`cargo xtask corpus-tree-hash` on the codeload archive; `cargo xtask fetch-sources` verified
it). The tag merges the duplicate Game Mastery Guide record into `game-mastery-guide` and drops
the `gamemastery-guide` tree (data #62), places or links the 21 rules that only the catch-all
chapter reached (data #63), and carries the third-party contamination fixes (data #65). The
stored-id migration is active for the first time. The published-id guard passes, and the entry
count stays 31,807.

The app no longer shows the corpus chapter `New entries (page order)`: it is not a Contents row,
has no chapter page, and is left out of previous/next (#363). `cargo xtask content` now fails
when the reachability report finds an entry with no route without that chapter. On this pin the
report shows 0 unreachable entries of 31,807, so the compile passes.

Local verification, 2026-11-09 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v96888807160b6179`, schema 4, 31,807 entries, corpus tag `v0.1.18` (was `v17a954e5c0f1d11c`) |
| `cargo xtask content` | Exit 0; the published-id guard passed; reachability 0 of 31,807 |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `7d43eb0e1ce6ed18`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/books.spec.ts tests/search.spec.ts tests/continue-reading.spec.ts tests/active-books.spec.ts tests/reference.spec.ts tests/relationships.spec.ts` | 107 passed |
| `npm --prefix tests/e2e test -- --project=webkit tests/continue-reading.spec.ts tests/active-books.spec.ts` | 20 passed |
