---
title: "Corpus pinned to v0.1.20"
date: 2026-11-15
kind: pin
---

# Corpus pinned to v0.1.20, 2026-11-15

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.20`. The annotated tag object
is `4a91fbfece8c3046243d9aaf3736a1cddecf14c9`; it peels to commit
`3a389dd8c6d3e287288e1a5cd80ac8c1749e3d19`. The tree hash is
`581f13beef2394b3aa378a30452d5318d7d9d2b509f4c8359d11c288423d1fd1`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The data corpus CI run
[43755724563](https://github.com/phynics/cast-iron-data/actions/runs/43755724563) succeeded for
this exact commit. The tag merges data #70, which removes the stray `p!` import markers from 23
entries and restores the printed wording of the 79 lines that carried one.

The corpus still holds 31,807 entries, 439 publications and 448 books; this pin changes entry
body text only. The library schema stays 4 and the search index format stays 2. The
published-id guard passed for all 42,019 ids: 9,968 legacy PSRD ids and 32,051 `ref/` ids.
`sources/published-ids.txt` does not grow for a text-only corpus change.

`corpus-check` now fails on a left-over `p!` import marker, with the same rule the data
repository's removal script uses, and the pinned corpus passes it. Because
`cargo xtask content` runs that check, the guard and this pin land together.

Local verification, 2026-11-15 (no CI or deployment claim; CI is pending):

| Check | Result |
|---|---|
| `generated/content` | Content version `v046e8483642b667c`, schema 4, index format 2, 31,807 entries, corpus tag `v0.1.20` (was `v9e4cebf228faf94e`) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,807 entries, 439 publications, 448 books, 1 licence |
| `cargo xtask content` twice | Byte-identical (`diff -r`): content tree `bca72dd47d3a14d280e5cab494bb1f8935dd7dddd515b71849f3559c62f822cd`, reports tree `a5c10009545f6d5863a02c1d6a08f171b88d5f06c8bea033ae39e17feeacb08f` |
| `cargo xtask check` | Passed: fmt, clippy for native and WASM, workspace tests, WASM build |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `3b4bbfd362411843`, 11 files; `generated/content` and `dist/content` are identical |
| Chromium: reference | 18 passed |
| Chromium: `#/entry/ref/arc-fighter/two-handed-fighter` | No `p!` in the page text; the four restored lines read "These bonuses increase to +2", "+3", "+4" and "+5" |

**Limitations.** CI for this branch is pending, and no Pages deployment or live check happened
for this pin. Nothing was promoted, so `release` is unchanged.
