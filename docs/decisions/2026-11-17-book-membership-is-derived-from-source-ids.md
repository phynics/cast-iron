---
title: "Book membership is derived from `source_ids`"
date: 2026-11-17
status: current
superseded_by: []
issues: [394, 420]
prs: []
area: content
---

# Book membership is derived from `source_ids`, 2026-11-17

Closes [#394](https://github.com/phynics/cast-iron/issues/394). This carries out the first bullet of
"Owner decisions from the architecture review, 2026-11-16" and part of H1 of the architecture review.

- **The rule.** An entry belongs to each book in its `source_ids`, in that order. It is visible while
  any of those books is active. Placement in `books.json` (a chapter, a page-ordered list, the
  catch-all) is presentation only: Contents, chapter pages and the reading footer. It no longer owns
  an entry for any other purpose.
- **Where it is computed.** The compiler writes `EntryStub.books` (`EntryStub::encode_books`) from
  `source_ids`: the distinct ids in order, omitted when there is only one. `validate_library` fails
  when a stub differs from its entry. The client never recomputes membership from placement.
- **One client module.** `crates/web/src/membership.rs` (`BookMembership`) answers visibility, the
  per-book counts (`count_by_book`), "is this entry in book X" (`is_member`) and which book a row
  names (`preferred_book`, `BookMembership::row_book`). Discovery's eligible set, `Library::
  is_stub_active`, the Books and book-page counts, the book filter on list pages, the search rows,
  Quick search rows, spell-list rows, the hidden-match hint and the entry meta line all call it. It
  holds no signal, no `RefCell` and no browser call, so native tests cover it (AGENTS.md rule 8).
- **Search.** The index still records one `source` per document (`source_ids[0]`), so
  `INDEX_FORMAT_VERSION` stays 2. `SearchIndex::set_extra_sources` tells the index the other member
  books from the catalog, as `set_source_ranks` already does with ranks. A book filter then keeps a
  document that any of its books matches, and a book chip counts it once under each book. Ranking
  and `Hit::source` still use `source_ids[0]`.
- **A row names one member book.** The first while it is active, else the highest-priority active
  one (the former `preferred_source` rule, moved from `views/book_tier.rs`), else the first. The
  entry header, the search and Quick search rows, the browse rows, the spell-list rows and the
  Recently viewed rows use it. `membership::listed_book` fixes the precedence for a listed row: the
  book that revised the entry (a third-party revision replaces its original in place and keeps
  naming its own book), else the book a book filter chose when the entry belongs to it, else the
  active-books choice above. A search row can now name a third active book, which the old entry
  said needed a format change.
- **Format.** `SCHEMA_VERSION` goes from 4 to 5, because the catalog stub gains `books`. This is a
  "format 5" change: it is on `main`; promotion to `release` waits (see the 2026-11-16 decisions).
  The reader still opens schema 1 to 4: `READABLE_SCHEMAS` in `cast_iron_schema::manifest_check`,
  the validator that `build-site` and the installer share, lists 4 explicitly, so an installed
  schema-4 library keeps validating and opening (rule 7). A schema-4 stub has no `books`, so the entry belongs to its
  `source` alone, and the update route offers the schema-5 library. Rule 5 holds: the field follows
  `source_ids` order and a test pins the serialization. `SelectionMasks` (#429) read the same
  visible-id set and are rebuilt per selection revision, per Library instance.
- **Before promotion.** The previous-library upgrade exercise of the `verify` skill (a real
  schema-4 library installed, then the schema-5 shell: stars and active books kept, offline reload)
  has not run. Native tests and the schema-5 to schema-5 browser updates do not prove it. It is a
  gate of the coordinated format-5 promotion (`docs/releasing.md`), not of this merge.
- **Removed.** `BookLookup::source_for_entry`, `canonical_book_source`, `count_by_source`,
  `Library::canonical_source_for_entry`, `eligible_ids`, `is_listing_chapter` and the
  `Page order`/`Unpaged` title constants (the catch-all title stays, for Contents and reachability).
  `BookLookup` keeps only the book trees.

Superseded:

- "Reference simplification implementation": `Discovery` snapshots eligible IDs using book placement,
  with a catalog-source fallback, and "Search publication filters still describe attribution sources,
  which can differ from book placement". Eligibility is membership, and the search filter and chips
  use membership too.
- "A listing chapter owns an entry only as a last resort, 2026-11-15" (#396): the whole canonical-book
  rule, the listing-chapter ranking and `is_listing_chapter`. A placement cannot own an entry. Its
  "Not changed" paragraph (a typed listing flag, a `corpus-check` rule) is split: the flag is not
  needed; the `corpus-check` rule is [#420](https://github.com/phynics/cast-iron/issues/420).
- "The search row and the entry header use one preferred-source rule over different data": the data
  is now the same (`EntryStub::member_books`).
- The `BookLookup` first-placement map of "Book reading order" ("`BookLookup` now keeps only the
  first-placement map...").

Behaviour a reader sees:

- An entry that two books print stays while either is on. Acrobatic stays with Core Rulebook on and
  Mythic Adventures off, because only Core Rulebook prints it.
- Books and a book's page count an entry under every book that prints it, so the counts add up to
  more than the number of entries. They no longer count an entry under a book that only places it.
- A book page's kind groups and `#/browse/kind/<kind>?source=` list the book's members, not the
  entries it places first.
- Search source chips count an entry once under each member book, and the book filter finds it
  through any of them.
- Under a book filter on the results page, a row names the filtered book (`membership::chosen_book`)
  when the entry belongs to it, so a list filtered by Ultimate Intrigue does not label a row with
  Core Rulebook. Without a filter the rule below applies.
- A row without a filter or a revision names `source_ids[0]` while that book is on. Spell-list rows and Recently viewed rows used
  to name the placement book, and now name this too. The hint "N matches in inactive books (X)"
  names the first member of the hidden entries, not the placement book.
- Contents, chapter pages, the reading footer and previous/next do not change. A Contents row can
  still list an entry under a book that does not print it until the corpus placement rule of #420
  lands; that is presentation, and it hides nothing. Contents counts are not membership counts even
  after #420: Contents also leaves out the hidden kinds and the catch-all, and a placement inside a
  member book does not make the two lists equal.
- Corpus v0.1.24 (#440): every placement lies inside the entry's `source_ids` (39,159 placements, none
  outside, checked on the compiled library). Retired book ids keep working with membership: a
  `source=` filter on a list or search address maps a retired id (`aliased_book_source`) to the book
  that holds its entries, because membership matches the ids in `source_ids`; `?book=` and the
  book routes already did. A one-off tree id stays as typed and finds nothing.
- Interaction with #420: the corpus fix removes placements outside `source_ids`. It does not need
  this change and this change does not need it. Placements then lie inside the member books, which
  is a subset of membership, not equality with it.
