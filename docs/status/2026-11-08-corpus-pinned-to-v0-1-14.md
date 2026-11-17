---
title: "Corpus pinned to v0.1.14"
date: 2026-11-08
kind: pin
---

# Corpus pinned to v0.1.14, 2026-11-08

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.14`, commit
`feb184e60563f9ccc60cfc7460140290f87b8219`, tree hash
`60d9706e7e5f3adfd177062a80e431d042c743256ad6ac41a63685d7409a2274`
(`cargo xtask corpus-tree-hash` on the codeload archive; `cargo xtask fetch-sources` verified
it). The pin marks The Elephant in the Room and Tome of Horrors Complete as third party, rebuilds
the book trees to follow the printed chapters (Game Mastery Guide, Core Rulebook, Bestiary, ARG
and others), and carries 34 partial changes plus 76 feat revisions from The Elephant in the Room.

Local verification, 2026-11-08 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v332f238b7f9a7461`, schema 4, 31,961 entries (was `vde9dc21edbc50b07`, 31,851 entries) |
| `cargo xtask content` | Exit 0; the published-id guard passed |
| `cargo xtask check` | Passed |
| `cargo xtask build-site` | Passed; shell `69f22c87fb03ddac`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium` | 186 passed, 1 skipped |

The two marked books are off by default, so the settled-profile tests now derive their counts
from the non-third-party books and the fourth book preset is offered. Feat list and chip counts
exclude the inactive third-party revisions, and the chain-marker expectation filters the relation
graph to the active books.
