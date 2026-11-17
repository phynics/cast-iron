---
title: "Corpus pinned to v0.1.17"
date: 2026-11-09
kind: pin
---

# Corpus pinned to v0.1.17, 2026-11-09

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.17`, commit
`b91db5f280cebaa9832dfc53b661c6f6d66ab575`, tree hash
`9475234881b753039ec5e8a27f6b1d3027ed8b821555e52a6ccf6c97dc51ef93`
(`cargo xtask corpus-tree-hash` on the codeload archive; `cargo xtask fetch-sources` verified
it). The tag drops 151 duplicate records, whose ids now resolve through `aliases.toml`
(data #57, #59, #60), places the residual Advanced Race Guide chapters (data #58), and adds
the race hubs: the typed `ref/race/*` records stay and the carried ARG chapters alias to
them (data #60). It also adds anchors to 658 class-feature headings (data #61). The
published-id guard passes and the entry count falls by 151, from 31,958 to 31,807. The
coverage report records each drop as a duplicate exclusion.

Local verification, 2026-11-09 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v17a954e5c0f1d11c`, schema 4, 31,807 entries, corpus tag `v0.1.17` (was `v3023e551a457c6eb`) |
| `cargo xtask content` | Exit 0; the published-id guard passed |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `d3186470177a1899`, 11 files; `generated/content/manifest.json` and `dist/content/manifest.json` match |
| `npm --prefix tests/e2e test -- --project=chromium tests/relationships.spec.ts tests/class-options.spec.ts tests/books.spec.ts` | 38 passed; no test changes needed |
