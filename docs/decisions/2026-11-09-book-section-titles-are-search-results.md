---
title: "Book section titles are search results"
date: 2026-11-09
status: current
superseded_by: []
issues: [32]
prs: []
area: search
---

# Book section titles are search results, 2026-11-09

Refs [#32](https://github.com/phynics/cast-iron/issues/32). Owner decision in the issue on
2026-11-09: book section titles become their own search results, and `INDEX_FORMAT_VERSION`
increases.

- **One document per rendered Contents node.** The compiler indexes the chapter row and every
  nested node that the chapter page renders, with the book source, the permanent chapter id from
  #346, the row's anchor, its title, and the book title. Entry leaf rows are included too: a
  matching entry title and its book-section row are different results, and neither hides the
  other. The section anchor is the same `toc-<path>` id the Contents tree already uses, and the
  chapter page now puts that id on its `h2` and list rows.
- **The compiler and the client share the tree rules.** `crates/schema/src/books.rs` holds
  `chapter_ids`, `chapter_rows`, the Contents visibility rule, the catch-all label, and the new
  `sections` walk. `crates/web/src/graph.rs` and `views/book.rs` keep thin wrappers for their
  existing call sites. The compiler uses the same functions with the catalog's kinds, so every
  indexed section is a row that its chapter page actually renders.
- **Results are separate.** Search asks the index for book-section documents after the entry
  results and renders them under a `Book sections` heading with `Book section · {book}`. A
  section row opens `#/browse/book/<source>/<chapter>?a=<anchor>`; Back returns to the search.
  Quick search keeps showing entries and relations only, because its rows open entry pages.
- **Only active books contribute.** A section document is accepted by its kind and its book's
  active state, not by the catalog entry-id set. Turning a book off removes its sections; the
  inactive-book hint still counts entries only.
- **Format 2.** `DocMeta` gains `chapter` and `anchor`. `INDEX_FORMAT_VERSION` is 2, so an
  installed format-1 library is rejected with `UnsupportedVersion` and search shows the existing
  error with the Settings link, the same version mechanism as before. The library remains
  installable and replaceable.
- **Size.** On corpus v0.1.16, `search.bin` grows from 69,570,748 bytes to 70,960,128 bytes
  (+1,389,380 bytes) and the entry count is unchanged at 31,958.

Decisions that refine `docs/spec.md` and that a later reader cannot derive from the code.
