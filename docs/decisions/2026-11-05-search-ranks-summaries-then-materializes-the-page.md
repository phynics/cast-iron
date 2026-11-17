---
title: "Search ranks summaries, then materializes the page"
date: 2026-11-05
status: current
superseded_by: []
issues: [84]
prs: []
area: search
---

# Search ranks summaries, then materializes the page, 2026-11-05

Settles #84. The search screen requests every match because it groups duplicate titles and reports the true total, so it passed `usize::MAX` as the limit. The engine then built a highlighted title and a snippet for every match; each snippet tokenized up to 6 KB of body text. On the canonical 29,515-entry library that was about 95% of query time, and it grew with the number of matches rather than the rendered page.

- **Ranking and display are separate engine calls.** `SearchIndex::search_summaries_eligible` returns `HitSummary` rows (id, title, kind, source, score) in final order. `SearchIndex::highlighter` builds the query match pattern once, and `SearchIndex::materialize_hit` fills `title_matches` and `snippet` for the rendered groups only. The web view groups summaries, takes the page, materializes that page, and advances the same way for "Show more results". `search` and `search_eligible` keep their old signature and behavior for native callers and tests; they materialize the returned hits only.
- **Ranking is unchanged.** The golden top-50 ids for the 50 `tests/e2e/measure-library.mjs` queries came from the `origin/main` engine at `404c530` on content `vbf0523074eb3de80`; `crates/search/tests/canonical_ranking.rs` compares them when that library is generated. A synthetic test also checks bounded limits against the full ranking and summaries against full hits. `INDEX_FORMAT_VERSION` stays 1; the serialized index did not change.
- **The rest of the query path is allocation-light.** Each query token uses a document-indexed match table instead of a tree, prefix document frequency uses one reusable seen array, and a bounded limit selects the best rows before the final sort. None of these changes the comparator or the score formula.
- **`data-search-ms` covers ranking and page materialization.** It no longer includes filter-count work, facet lookups, or rule-heading reads, which were already outside the old measurement. Wall time from typing to results is a separate, larger figure.
