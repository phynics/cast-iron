---
title: "A listing chapter owns an entry only as a last resort"
date: 2026-11-15
status: superseded
superseded_by: ["2026-11-17-book-membership-is-derived-from-source-ids", "2026-11-17-placement-membership-and-retired-book-routes"]
issues: [394]
prs: []
area: content
---

# A listing chapter owns an entry only as a last resort, 2026-11-15

Refs [#394](https://github.com/phynics/cast-iron/issues/394). The review of `phynics/cast-iron-data#72` found that `ref/race/dwarf` hid when the Advanced Player's Guide was off and the Core Rulebook on, although the Core Rulebook places Dwarf in a curated chapter.

- **The rule.** An entry's canonical book (`BookLookup::source_for_entry`, so `canonical_book_source`, `Library::is_stub_active`, discovery counts and the `Book · Kind` lines) is the first book in book order that places it in a printed chapter. A listing chapter counts only for an entry that no printed chapter of any book places; then the first book whose listing places it owns it. A listing chapter (`is_listing_chapter`, `crates/schema/src/books.rs`) is the compiler's `New entries (page order)` catch-all or one of the corpus's page-ordered lists, `Page order` and `Unpaged`, whose `books.toml` basis says "this is a page-ordered list, not a chapter structure".
- **Why the page-ordered lists too.** The first version skipped only the catch-all. Unchained Monk is printed in five books and placed in the Advanced Player's Guide catch-all, in `Unpaged` lists of Blood of the Beast, Chronicle of Legends and The Elephant in the Room, and in the `Page order` list of Pathfinder Unchained. No book prints it in a chapter. The first version made Blood of the Beast the owner, so with only the Advanced Player's Guide, Pathfinder Unchained and The Elephant on, the base showed `Inactive` (`changes.spec.ts`, full CI on #396). With the lists ranked like the catch-all, the owner is the Advanced Player's Guide again, as before this fix. An entry changes owner only when some book prints it in a chapter.
- **Why.** Contents never shows the catch-all, and the page-ordered lists are evidence for order, not for where a book prints an entry. Such a placement must not decide which book an entry belongs to, or whether turning a book off hides it, when another book prints the entry in a chapter. Book order still decides between two printed placements, and between two listings.
- **Meta line.** The meta line (`preferred_source`) works from the entry's `source_ids`: it names the first one while active, else the best active one. Hiding works from the canonical book above. They agree whenever the canonical book and the preferred source are both active. After cast-iron-data #72 (corpus v0.1.21), `source_ids[0]` of the core races is the Core Rulebook, which is also their first curated placement, so the two name the same book for them. They can still differ for an entry whose canonical book is not in its `source_ids`, or is not its first one. This slice does not change that. No serialized shape change: `BookNode`, `BookIndex` and `books.json` are unchanged, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay (the schema crate gains only the title constants and `is_listing_chapter`). No compiler or search-format change; the compiler computes no book placement of its own (its stub `source` is `source_ids[0]`).
- **Not changed.** The compiler's reachability report (`reachability.rs`) still treats only the catch-all as outside the printed chapters, so its "only placement" list and this rule differ for page-ordered lists. A typed listing flag on `BookNode` and a `corpus-check` rule that a renamed list title fails loudly belong to the membership redesign (#394).
