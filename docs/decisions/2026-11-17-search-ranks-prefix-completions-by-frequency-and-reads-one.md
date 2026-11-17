---
title: "Search ranks prefix completions by frequency and reads one eligibility mask per selection"
date: 2026-11-17
status: current
superseded_by: []
issues: [422]
prs: []
area: search
---

# Search ranks prefix completions by frequency and reads one eligibility mask per selection, 2026-11-17

Closes [#422](https://github.com/phynics/cast-iron/issues/422). The search format does not change: `INDEX_FORMAT_VERSION` stays 2, and a break would wait for format 5 (owner decision of 2026-11-16).

- **A short prefix keeps its 512 most frequent completions.** `expand` used to keep the first 512 completions in alphabetical order, so "ma" lost "magic" to terms that sort before it. It now keeps the 512 terms with the highest document frequency. Frequency is the posting-list length that the index already stores per term (`dfs`), so no format change is needed. Equal frequencies keep the alphabetically earlier term, so the choice never depends on the sort algorithm. The kept terms go back to alphabetical order before scoring, so the floating-point sums keep the order they had. Only a prefix with more than 512 completions changes; every other query, and the 50 queries of the canonical golden, rank as before. In the 56-query comparison on the v0.1.23 library (the 50 golden queries plus "ma", "m", "c", "sp", "char" and "bl"), only "ma", "m", "c" and "sp" change their top 50. The benchmark's aggregate for "ma" across its four passes (entry, section and unfiltered hits plus the filter-count total, with overlapping documents counted in each) rises from 47,022 to 54,354 because the kept completions cover more documents. It is not the query's hit count.
- **The mask is built once per book selection.** `SearchIndex::eligible_mask` evaluates the selection for each document once and returns an `EligibleMask` (a flag per document ordinal). `search_summaries_masked`, `search_masked`, `filter_counts_masked` and `suggest_correction_masked` read it by ordinal. The closure APIs stay and give the same results; `filter_counts_eligible` now builds a mask and calls the masked one. The mask logic is native and tested (rule 8).
- **The client caches three masks per selection revision** in `DiscoveryCache`, next to the eligible id set: entries, book sections, and both (for a correction that includes sections). A snapshot from an older revision builds its own and leaves the cache alone. `filter_counts_counted` narrows the entry mask with the caller's own test, which runs only for eligible documents.
- **The ranking golden has a synthetic fixture.** `crates/search/tests/synthetic_ranking.rs` indexes `tests/fixtures/synthetic-corpus.txt` (invented text, 31 documents, plus a generated crowd of 612 documents that pushes the prefix "zq" past the cap) and compares the top ids of the queries in `synthetic-queries.txt`, with all books and with one book off through a mask, against `tests/golden/synthetic-top.txt`, and asserts the correction for "magic missle" and that the masked correction equals the closure one. It needs no generated library, so `cargo xtask check` always runs it. Only `CAST_IRON_BLESS=1` rewrites the golden after a deliberate ranking change, and it refuses to run when `CI` is set. `canonical_ranking.rs` stays as an optional extra: it still skips when the content version differs.
- **Timings** (native, release, v0.1.23 library, all four passes of one keystroke, median of 7) are in the pull request. `crates/search/examples/bench_keystroke.rs` reproduces them.
