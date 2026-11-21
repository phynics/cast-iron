---
title: "Contents addresses follow the row, not its position"
date: 2026-11-21
status: current
superseded_by: []
issues: [546, 550]
prs: [553]
area: content
---

# Contents addresses follow the row, not its position, 2026-11-21

Closes [#550](https://github.com/phynics/cast-iron/issues/550). It finishes what
`2026-11-21-book-section-anchors-follow-the-row` left under "Not covered".

**Problem.** `#/browse/book/<id>?c=3.2` named a Contents row by its 1-based path in the book tree. A pin
that adds or removes a row moves every later row, so a saved or shared address marked another row. The
trail above an entry writes `?c=` from the loaded tree, so only addresses that someone kept drifted.

**Scheme.** A Contents row has the identity that a chapter page already gives it.

- A chapter row is `?c=~<chapter id>`: the permanent id of its chapter page (`chapter_rows`).
- A row below a chapter is `?c=~<chapter id>&a=<row anchor>`: the chapter id and the stable anchor that
  `books::section_anchors` gives the row on that page. The anchor is the one search results and `?a=` on
  the chapter page use, so one name serves all three addresses.
- The `~` marks the identity form. A chapter id that reads as a position (`2`) stays an id after it.
  `cast_iron_schema::books::ContentsRow` holds both forms; `contents_row` makes the stable one from a
  path and `resolve_contents_row` finds the row again in the tree that is loaded. A row that moved to
  another chapter is still found, as section anchors are.
- The trail above an entry writes the stable form. The DOM ids of Contents rows stay positional
  (`toc-3-2`): they are not an address, and the page computes them from the tree it has loaded.
- The Contents page scrolls to the marked row itself once the address has resolved. The route-level
  scroll used to compute the DOM id from the position in the URL, which cannot be done for an identity.

**Old addresses.** `?c=3.2` still parses to `ContentsRow::Position`. The client resolves it through
`legacy-sections.json`, the table that #549 added, and then replaces the address with the stable form of
the row it found. `legacy-sections.json` held the rows below chapter level (`toc-2-26`, the `books` map),
which is what `?c=2.26` means. The chapter rows themselves (`?c=2`) were not in it, so the table gains one
additive, optional map, `chapters`: book, then `toc-<n>`, then the chapter id that the position held in the
last published tree. A chapter id is permanent, so this map does not go stale either.

- A position below chapter level goes through `books[book]["toc-…"]` to a stable anchor, and the row is
  looked up in the current tree, preferring the chapter that held the position.
- A position of a chapter row goes through `chapters[book]["toc-n"]` to a chapter id.
- A row that left the book, a position that no published tree made, and a table that lists no such row
  open the top of Contents: the address is replaced by the plain book address.
- A library without `legacy-sections.json` (an older install) reads the position against its own tree,
  which made it. A table that exists but cannot be read is an empty table, never absent: an unreadable
  table does not fall back to position in the new tree. The `replace` is skipped when the reader has
  moved on while the table loaded.

**Regeneration.** The table needs no regeneration after any pin: it holds stable anchors and chapter ids,
which a pin does not move. It was regenerated once, now, with
`cargo xtask legacy-sections` from the `books.json` and `catalog.json` of the live library at corpus
v0.1.31 (content `v4192151b29702eab`), the tree that the old addresses were made against. The `books` map
is byte-for-byte what #549 committed; the file only gains `chapters` (413 chapter rows in 244 books).
`cargo xtask content` checks the new map (top-level positional anchors, non-empty ids) and notes how many
chapter ids still name a chapter.

**Format impact.** `SCHEMA_VERSION` stays 6 and `INDEX_FORMAT_VERSION` stays 2. `legacy-sections.json`
keeps `format: 1`; `chapters` is a new optional key that a reader without it ignores. An older app shell
reads a new library and meets `?c=~…` only from a new address, which it does not mark; nothing else
changes for it.

**Known limits.** The limits of #549 hold: a numbered anchor (`-2`, `-3`) of a repeated heading renumbers
when an earlier repeat leaves its chapter. A chapter whose title slug collides with another chapter's takes
`-2` in tree order, so such a chapter's id can move when the earlier one leaves the book. An address for a
row that Contents collapses into its hub marks nothing.
