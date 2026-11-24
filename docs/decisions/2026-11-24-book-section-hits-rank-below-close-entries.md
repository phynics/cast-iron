---
title: "Book-section hits rank below entries with a close score"
date: 2026-11-24
status: current
superseded_by: []
issues: [691]
prs: []
area: search
---

# Book-section hits rank below entries with a close score, 2026-11-24

Closes [#691](https://github.com/phynics/cast-iron/issues/691). Owner ruling of 2026-11-24 for v1: a book-section hit ranks below an entry hit whose score is close.

- **The problem.** A contents row such as `Summon` or `Burn` in a bestiary matched the one-word query by title exactly and took the exact-title boost (1000). It outranked the spells and feats the reader meant, such as `Summon Monster` and the feat `Burn! Burn! Burn!`, which only start with the query (500).
- **The rule.** In `score_docs`, a book-section document takes 0.4 of the title boost that an entry with the same title match would get (`BOOK_SECTION_BOOST_SCALE` in `crates/search/src/query.rs`). An exact section title is worth 400, a title prefix 200, all words in the title 80. The text score is unchanged.
- **What stays.** An exact chapter title still ranks above every entry whose title merely holds the words (200) and above every body match, so a reader who types a chapter name finds the chapter. An exact entry title (1000) still ranks above an exact section title.
- **Why a scale, not a flat penalty.** One constant, no new tier, and it keeps the order among sections: they all scale alike.
- **Runtime only.** The score is not stored: `INDEX_FORMAT_VERSION` and the search bytes do not change, and the compiler output is byte-identical.
- **Golden.** `canonical-top50.txt` changes in 40 of the 50 queries. Book sections in the top 10 fall from 53 to 5 over those queries, and no top-1 result changes.
