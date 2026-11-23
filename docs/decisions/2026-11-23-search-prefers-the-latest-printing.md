---
title: "Search prefers the latest printing of a rule kept in several books"
date: 2026-11-23
status: current
superseded_by: []
issues: [619]
prs: []
area: search
---

# Search prefers the latest printing of a rule kept in several books, 2026-11-23

Refs [#619](https://github.com/phynics/cast-iron/issues/619). Browse-list order of same-name entries is a separate issue, [#637](https://github.com/phynics/cast-iron/issues/637).

- **The rule.** Results that tie exactly on relevance and on product line order by the newest `publication_date` among the books that print the entry (its member books, `source_ids`). The order of keys is: score, product-line rank, printing recency (newest first), title, id. A book without a date counts as undated and sorts after every dated one; undated entries tie on this key and fall back to title and id.
- **Each entry ranks by its own books.** A book that revises a rule prints a different entry, so its date does not count for the base entry.
- **Where it shows.** Ranked results, Quick search, and the lead row of a same-name result group, which is the first hit in ranked order (`grouped_hits`). The group needed no change. The empty-query (filter-only) list still sorts by title.
- **Runtime only.** The recency is not serialized: `SearchIndex::set_printing_recency` takes `(entry id, recency)` pairs at library load, as `set_source_ranks` takes its ranks, so `INDEX_FORMAT_VERSION` and the search bytes do not change. The client derives the values from the catalog (`printing_recency` in `crates/web/src/views/book_tier.rs`): each distinct date is a dense rank, and an entry takes the highest rank among its member books. Dates are ISO 8601, so they compare as text.
- **Why recency ranks below the product line.** A rulebook printing stays ahead of an adventure's copy of the same text. Recency only decides between printings of one product line, such as a rule that a later rulebook reprints.
