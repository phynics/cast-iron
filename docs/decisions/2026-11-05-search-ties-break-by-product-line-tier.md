---
title: "Search ties break by product-line tier"
date: 2026-11-05
status: current
superseded_by: []
issues: [101, 125]
prs: []
area: search
---

# Search ties break by product-line tier, 2026-11-05

Closes [#125](https://github.com/phynics/cast-iron/issues/125), split out of [#101](https://github.com/phynics/cast-iron/issues/101). It reuses the `Tier` of the two entries above.

- **Relevance first, then tier, then title and id.** `sort_scored` in `crates/search/src/query.rs` orders by score descending. Only an exactly equal score (`f32::total_cmp` equal) consults the source rank, lower first. The old title and id tie-breaks follow, so the order stays total and deterministic. A spell reprinted in the Core Rulebook and in a Player Companion has identical text, so it scores equal and the Rulebook entry comes first. Entries with different titles that happen to score equal are also ordered by tier now, ahead of the alphabet. Browsing by filters with no query text is a title listing with no relevance; it is unchanged.
- **The rank is plain data that the client passes at load time, not part of the index.** `SearchIndex::set_source_ranks` takes `(source id, u8)` pairs and builds a per-document rank array that is not serialized. `crates/web/src/library.rs` calls it with `book_tier::source_ranks(&catalog.sources)` right after decoding `search.bin`. `crates/search` still has no dependency on `schema` or on `Tier`.
- **One table, no drift.** `Tier::CATEGORIES` stays the only mapping from category to tier. `Tier::search_rank` is derived from the tier's position in `Tier::ALL`: Rulebooks 0, Player Companions 1, Campaign Setting 2, Adventures 3. A native test checks that every entry of the table agrees with the rank and that the ranks increase. The Books page, the Publication filter and search ranking therefore move together when a category changes tier.
- **Other is neutral.** `Tier::Other` (Miscellaneous, an unknown category, no category) takes `NEUTRAL_SOURCE_RANK` (255), the value that search gives a source with no rank. A PSRD library has no `RPG` category, so every source is neutral and the ordering is the title and id order that it had before. An index that never receives ranks behaves the same way.
- **`INDEX_FORMAT_VERSION` stays 1, `SCHEMA_VERSION` stays 3, and the compiler and the corpus are unchanged.** The alternative, a rank written into each `DocMeta`, would be a format change. The client would then reject every installed `search.bin` until the library was replaced, which means search would break for installed libraries. Deriving the rank from the catalog's existing optional `category` field keeps an installed library searchable, with the new order, immediately. A library whose catalog has no categories keeps the old order. The cost is one pass over the documents when the index loads.
- **A multi-source entry ranks by its first indexed source.** The rank is looked up from `DocMeta.source`, which the compiler sets to `source_ids[0]`, the first source in page order. That is the same source the search Publication filter and its counts use, but it is not always the book where the Books page places the entry, because the Books page uses book order. For an entry reprinted across tiers, the two "firsts" can disagree. Ranking by the best tier over all of an entry's sources would need every source id in the index, which is a format change and out of scope here.
