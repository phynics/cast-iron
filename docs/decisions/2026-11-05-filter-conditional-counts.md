---
title: "Filter-conditional counts"
date: 2026-11-05
status: current
superseded_by: []
issues: [15, 41]
prs: []
area: ui
---

# Filter-conditional counts, 2026-11-05

Settles #41 (split from #15). Search and Browse used to show the same active-book totals, so a tile could say "768 spells" while the active filter showed 12. The two views now have separate meanings.

- **Search counts follow the query and the other filters.** A kind chip, publication chip, or facet value shows how many eligible entries match the query text and every active filter except the chip's own group. Standard faceted-search rule: the group you are editing ignores its own selection, so alternatives stay visible and a chip's number is the result size you get by choosing it. With no query and no filters the counts are the active-book totals. The count is a number of entries; when several publications print one article, the result line already reports "N results · M publication entries", and chips count the M. `SearchIndex::filter_counts_eligible` computes it from the same scoring path as `search_eligible`. It reads the stored kind, source, and facets of each matching document, so `INDEX_FORMAT_VERSION` and the compiler are unchanged.
- **Browse and Books counts are library totals, and say so.** Category tiles and category pages are labelled "Entries in your active books", and Books is labelled "Entries in each book, whether or not it is active". Both stay unconditional: they describe what a book or category holds, not a search. The home page applies the same label in its own change.
- **Counts do not flicker.** The counts change when a result arrives, together with the result list. While a query is debounced or a search is running, the chips keep the previous numbers. Chips keep a fixed minimum count width, and a chip with zero matches stays in place (dimmed) instead of disappearing, so the layout does not move.
