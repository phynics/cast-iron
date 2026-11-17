---
title: "Plain-text corpus, step 3: the build reads the corpus"
date: 2026-11-06
kind: record
---

# Plain-text corpus, step 3: the build reads the corpus, 2026-11-06

`cargo xtask content` now compiles the canonical library from the pinned corpus
(`phynics/cast-iron-data` tag `v0.1.0`, commit `0e7c3abe620b80293e0a6382e2ab7a4b3372f8e4`,
pin in `sources/corpus.toml`). The pin has since moved to `v0.1.2`; the measurements in
this section are from `v0.1.0`. `--from-upstream` keeps the previous build for comparison.
Decision: [the pinned corpus is the source of truth](../decisions/2026-11-06-the-pinned-corpus-is-the-source-of-truth-for-the-canonical.md).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| Pinned corpus | Tree hash `d103cc70e91293ebdd7026d0ac01013dfa3edfba6395ca8b4bbfb7d27a6df373`, 31,136 files, verified by `fetch-sources`; a hand-edited cache fails with the expected and actual hash |
| `generated/content` from the corpus vs `--from-upstream` | Byte-identical: `v6aa587a686a3caf0`, 31,106 entries, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Two corpus builds from scratch | Byte-identical over all of `generated/` (content and reports): `dfba02a16b4fa38f807933032b91f2d89e2998a0a7a63915672805c7f62107d8` |
| `dist/content` after `cargo xtask build-site` | Same tree hash as `generated/content` (`2c6005d6...`) |
| Reports | `validation.json` and `coverage.md` differ from the upstream build (one corpus note; no upstream survey or `link-pass.json`); `coverage.json` is in the content tree and identical |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0 (`--psrd` unchanged: `vcdd730994d863e50`, 9,990 entries) |
| `check-publication.py` | Exit 0 |
| `cargo xtask data-report` | Exit 0; tracked `reports/` files unchanged |
| `npm --prefix tests/e2e test -- reference search install --project=chromium` | 35 passed (a sample, not the full suite) |
| `cargo xtask check` | Passed |
