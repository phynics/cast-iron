---
title: "List folds order their printings like search"
date: 2026-11-23
status: current
superseded_by: []
issues: [637]
prs: []
area: ui
---

# List folds order their printings like search, 2026-11-23

Closes [#637](https://github.com/phynics/cast-iron/issues/637). It extends `2026-11-23-search-prefers-the-latest-printing` to the browse lists, without editing it. Linking the printings of one rule to each other is a separate, post-v1 issue, [#650](https://github.com/phynics/cast-iron/issues/650).

- **The rule.** The links of a fold (browse lists and the requirements panel of a prestige class) run in the order search gives equal results: product-line rank of the entry's primary book, then the newest printing (the newest-dated member book; a book with no date last), then book title, then id. Every entry stays; none is merged or removed.
- **One definition.** `PrintingOrder` in `crates/web/src/views/book_tier.rs` holds the rank and the recency. Search's recency values (`printing_recency`) and the fold's sort both read it, so the two cannot drift. The library builds its per-book table once.
- **The fold summary** takes the spelling of its first link's title, so the fold leads with the printing that it lists first. Its place in the list is unchanged: rows still sort by title, parent title and id.
- **Filter-only search.** A search with filters and no text puts the latest printing first inside a group of equal normalized titles, ahead of the id. Titles still order the list, so no other order changes. Lists, ranked search and filter-only search now agree.
- **Example.** *Tent* listed Advanced Player's Guide, Core Rulebook, Ultimate Equipment (by book title). It now lists Ultimate Equipment first.
- **No format change.** Nothing new is serialized: `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
