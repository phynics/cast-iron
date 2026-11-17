---
title: "Corpus pinned to v0.1.16"
date: 2026-11-09
kind: pin
---

# Corpus pinned to v0.1.16, 2026-11-09

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.16`, commit
`55aca627334297e9df7a5cad46ba949cac9bcbc9`, tree hash
`44c732b4faadd2c488a231d25103e86becd031a98240deeb2d918605a810fafb`
(`cargo xtask corpus-tree-hash` on the codeload archive; `cargo xtask fetch-sources` verified
it). The tag adds the Adventures series fields to `publications.toml` and rebuilds the book trees
for the Technology Guide, Ultimate Campaign, Ultimate Combat, Ultimate Equipment and Ultimate
Magic. It drops three duplicate entries: `ref/rule/hierophant`, the carried Technic Scavenger and
`ref/tech-misc/silverdisk`. Each dropped id resolves through `aliases.toml`, so the published-id
guard passes and the entry count falls by 3, from 31,961 to 31,958. The coverage report records
each drop as a duplicate exclusion.

Local verification, 2026-11-09 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v3023e551a457c6eb`, schema 4, 31,958 entries (was `vb2a40ecd1f8a8665`) |
| `cargo xtask content` | Exit 0; the published-id guard passed |
| `cargo xtask check` | Passed |
| `cargo xtask build-site` | Passed; shell `df303149a7421e0e`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium` | 196 passed, 1 skipped; one new test opens `#/entry/ref/tech-misc/silverdisk` and lands on Battery |
