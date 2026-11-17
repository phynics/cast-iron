---
title: "Corpus pinned to v0.1.15"
date: 2026-11-09
kind: pin
---

# Corpus pinned to v0.1.15, 2026-11-09

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.15`, commit
`99a907d5b314bde448532ee6f817b61345cda87e`, tree hash
`e0756b1734602daaae0dbf5e9c337ca6728b7de598b85bfef5a1b820b51fa0ba`
(`cargo xtask corpus-tree-hash` on the codeload archive; `cargo xtask fetch-sources` verified
it). The tag changes only `books.toml`: nested placement checked against print for the Core
Rulebook, APG, ARG, Bestiary 1–4, Game Mastery Guide, Monster Codex, NPC Codex and Mythic
Adventures. It adds no entries and changes no ids.

Local verification, 2026-11-09 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `vb2a40ecd1f8a8665`, schema 4, 31,961 entries (was `v332f238b7f9a7461`) |
| `cargo xtask content` | Exit 0; the published-id guard passed |
| `cargo xtask check` | Passed |
| `cargo xtask build-site` | Passed; shell `0f3fb98f3d6df9dd`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium` | 191 passed, 1 skipped; no test changes needed |
