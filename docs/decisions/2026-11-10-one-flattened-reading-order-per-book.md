---
title: "One flattened reading order per book"
date: 2026-11-10
status: partly-superseded
superseded_by: ["2026-11-17-book-membership-is-derived-from-source-ids"]
issues: [347]
prs: []
area: ui
---

# One flattened reading order per book, 2026-11-10

Refs [#347](https://github.com/phynics/cast-iron/issues/347).

- **One cached order serves the whole reading footer.** `reading::BookReading` flattens a
  book's Contents-visible entries once, with an entry-position map and the chapter rows that a
  crossing names. `Library::reading(book)` builds it lazily and keeps the `Rc` next to the
  cached `BookLookup`, so every `Library` clone shares the value. Candidate membership, the
  breadcrumb trail and previous/next all read that one value instead of walking the book again.
- **The cache key is the library instance, not the active-book set.** Contents visibility comes
  from the immutable catalog kind (`hidden_from_contents`) and the chapter tree does not change
  while a library is active. Activating or deactivating a book only changes which book the footer
  chooses; it cannot change a book's flattened order. A new library instance has a new
  `BookLookup` and an empty cache, so a cache from an old content version is never reused. No
  selection revision key is needed, and there is no global cache.
- **The first Contents occurrence of an entry wins.** A later placement of the same entry in one
  book is ignored, including its path, title and chapter, so previous/next never steps onto a
  duplicate. Contents rendering still shows the corpus tree as written; only navigation takes the
  first occurrence.
- **Hidden kinds, the catch-all and collapsed hubs keep their rules.** A hidden kind is skipped
  at any depth, the catch-all chapter is neither a stop nor walked, and a collapsed hub still
  contributes every child in book order. Chapter crossings and labels come from the cached
  chapter rows, so the footer still names the chapter it enters.

`BookLookup` now keeps only the first-placement map that answers an entry's canonical book; the
raw reading-order cache it held was not Contents-visible and is removed. The cache proof is
native: a test counts builder runs and visibility calls, and repeated membership, trail and
neighbour queries neither rebuild the order nor re-evaluate the book.
