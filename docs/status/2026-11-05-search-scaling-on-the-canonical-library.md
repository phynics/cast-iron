---
title: "Search scaling on the canonical library"
date: 2026-11-05
kind: record
---

# Search scaling on the canonical library, 2026-11-05

Refs [#84](https://github.com/phynics/cast-iron/issues/84). The measurements below were
taken on content `vd7596846128e349b`: 29,515 entries and `search.bin` 65,725,435 bytes,
format 1 (unchanged). Merging current main first moved the canonical build to
`vbf0523074eb3de80` (29,880 entries, `search.bin` 67,372,004 bytes); the same engine path
measures p50 3.55 ms and p95 9.52 ms natively there, and the browser numbers below were
repeated on that build. The search view asks for every match because it groups duplicate
titles and reports the true total, so the engine materialized a highlighted title and a
snippet for each match. Each snippet tokenized up to 6 KB of body text, which made query
time scale with matches rather than with the rendered page.

Native measurement uses `cargo run --release -p cast-iron-search --example
bench_library --features profile -- generated/content/vd7596846128e349b/search.bin web`.
It loads the compiled index and runs the 50 queries from
`tests/e2e/measure-library.mjs`, three times each. Per-phase p50/p95 in milliseconds:

| Phase | Before (all hits materialized) | After (summary rank + 50-row page) |
|---|---:|---:|
| Tokenize | 0.00 / 0.00 | 0.00 / 0.00 |
| Posting lookup | 0.02 / 0.08 | 0.02 / 0.08 |
| Scoring | 1.26 / 3.42 | 1.45 / 4.18 |
| Sort | 0.04 / 0.19 | 0.05 / 0.27 |
| Result materialization | 0.32 / 1.92 | 0.00 / 0.00 |
| Snippet/title building | 50.91 / 199.66 | 1.21 / 3.38 |
| Total | 52.21 / 205.86 | 3.11 / 7.52 |

These figures come from a quiet host. "Before" is the bench's `full` mode, which builds a
snippet for every match as the old path did, with the new scoring code. The unmodified
engine at `404c530`, timed over the same 50 queries with no other work running, measures
p50 53.7 ms and p95 187.3 ms (max 264.5 ms). Snippet building was about 95% of the query.
The largest result sets are 4,054 matches at p95 and 7,665 at the maximum.

Browser acceptance on the same `dist/`, run locally:

| Measure | Chromium | WebKit |
|---|---:|---:|
| `Search operates entirely offline` p95 (20 queries) | 6.4 ms (max 9.8) | 13 ms (max 13) |
| `measure-library.mjs` engine p50 / p95 (50 queries) | 4.0 / 10.0 ms | 4.0 / 11.0 ms |
| `measure-library.mjs` input-to-results p50 / p95 | 131 / 185 ms | 127 / 162 ms |

Ranking is unchanged. `crates/search/tests/canonical_ranking.rs` compares the top 50 ids
for all 50 queries against a golden captured from the `origin/main` engine at `404c530`
on content `vbf0523074eb3de80`, and passes. A synthetic test compares bounded limits with
the full ranking and checks that summary materialization matches full hits.
`INDEX_FORMAT_VERSION` and the serialized layout are unchanged.

Local verification: `cargo xtask check` passes, and the full Playwright suite passes
114/114 (chromium 56, webkit 56, mobile-chromium 2) against the canonical `dist/` in
14.0 minutes. Nothing here is deployed, and the 250 ms smoke bound is unchanged.

After merging main again, the canonical build moved to `v3767562a585b5ce4`, so the pinned
golden skips. The baseline engine at `404c530` and the current engine were run over the
same `search.bin` and the same 50 queries: all 2,550 lines (top 50 ids and scores) are
identical. On that build `measure-library.mjs` reports engine p50 / p95 of 4 / 14.8 ms in
Chromium and 4 / 10 ms in WebKit, and the three search specs pass in both browsers. This is
local verification.
