---
title: "Page-order books are derived from [[sources]] (corpus format 3)"
date: 2026-11-17
status: current
superseded_by: []
issues: [425]
prs: []
area: corpus
---

# Page-order books are derived from `[[sources]]` (corpus format 3), 2026-11-17

Refs [#425](https://github.com/phynics/cast-iron/issues/425). It carries out the owner decisions of 2026-11-17 on the step 1 report of that issue.

- **What goes away.** 423 of the 439 books were a list that the `[[sources]]` pages already determine: `Page order`, then `Unpaged`, with nesting by the entry's `parent`. They were 2.5 MB of the 4.4 MB `books.toml`. Only the 16 books with a printed chapter structure are stored, one `books/<publication-id>.toml` each (1.8 MB with their catch-all rows removed). The rest is derived by `book_index` in `crates/corpus`, which `cargo xtask content` and `corpus-check` both call. [`docs/corpus-format.md`](../corpus-format.md#books-format-3) has the rules.
- **Same-page order is kind, then title, then id** (owner). It reproduces 96.5% of today's same-page runs. Title alone gives 89.0% and id alone 80.0%. The order of entries on one printed page is not evidence; this is a convention, and the evidence note says so.
- **The revision rows of The Elephant in the Room lose their suffix** (owner). The book authored `Agile Maneuvers (The Elephant in the Room)` for a revision entry titled `Agile Maneuvers`. A derived node is the entry title, so 76 Contents rows read without the suffix. The entry page still names its book.
- **The catch-all chapter is generated for the 16 curated books too** (owner). `New entries (page order)` holds the members the curated tree does not place, in the derived order. The app never shows it, so the row order is not user-visible; reachability and `BookLookup` read its membership, and the entry set equals the stored one in all 16 books. 159 KB of stored rows go away.
- **Corpus format 3, with a `migrate` step** (owner). Format 2 is taken by the typed trades (#450). The layout change is format 3, because a reader that does not know it would read a new corpus and build a library with no page-order books. The reader accepts formats 1, 2 and 3. The registered step from format 2 cuts `books.toml` and writes `format = 3`. `cargo xtask corpus-migrate-layout` is the cut alone and writes `format = 3` only from format 2: a format 1 corpus still has the trades migration (#451) to run. The reader therefore accepts the split layout below format 3 with a warning, and refuses `books.toml` at format 3. The library format does not change: `books.json` keeps its shape, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
- **Book order.** Curated books first, then derived ones, each group by the title folded to lowercase ASCII letters and digits, then id. The stored order had the Game Mastery Guide, a curated book, among the derived books; it now sits with the curated ones (position 123 to 7). No other book moves relative to the rest.
- **A derived book takes the publication title.** Stored titles of 14 books were the publication id (`carrion-crown-players-guide`) or a spelling that differs from `publications.toml` (`GameMastery Guide`). The title now matches the catalogue.
- **`[[sources]]` pages have a grammar.** `N`, `N-N`, a comma list of those, or `inside cover`. In a split corpus a page that is none of them is a `corpus-check` problem, because the book order reads the leading number. Before format 3 it is a warning: corpus v0.1.25 holds three such strings (Autumn Blast, the Colour-Blighted template, the Shadow template) that cast-iron-data #92 fixes.
- **The pin is not bumped here.** The pinned v0.1.25 predates cast-iron-data #91 (875 restored pages) and #92 (page corrections), and it is format 1 with a single `books.toml`, so this change compiles it to a byte-identical library. The mechanical corpus PR that runs the migration needs a corpus tag that holds #91 and #92, and then the pin bump.
