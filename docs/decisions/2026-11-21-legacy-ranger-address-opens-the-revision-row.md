---
title: "One legacy section address is edited by hand: The Elephant in the Room `toc-1-1`"
date: 2026-11-21
status: current
superseded_by: []
issues: [425, 546]
prs: [564]
area: content
---

# One legacy section address is edited by hand: The Elephant in the Room `toc-1-1`, 2026-11-21

Owner decision of 2026-11-21, taken with the pin of corpus v0.1.36 (corpus format 3).

**Problem.** `sources/legacy-sections.json` maps the positional address of every row that the last
published tree rendered to that row's stable anchor (see
[Book section anchors follow the row](2026-11-21-book-section-anchors-follow-the-row.md)). In the
published tree of The Elephant in the Room (`the-elephant-in-the-room-feat-taxes-in-pathfinder`), `toc-1-1`
was the stored `Ranger` heading above the revision `Ranger`; its stable anchor is
`row-h-unpaged--h-ranger`. Format 3 derives the book from each entry's `[[sources]]` pages, and a derived
book has no grouping rows, so that anchor no longer exists. By the rule of the decision above, the old
address `?a=toc-1-1` then opens the chapter with no anchor. It is the one row of the table that format 3
makes lose its row (6,794 of 6,795 rows opened their row on v0.1.33; 6,793 would on v0.1.36).

**Decision.** `toc-1-1` of that book maps to `row-ref-rule-class-ranger-ranger-teitr`, the row of the
Ranger revision. The child address `toc-1-1-1` already mapped to it, and the grouping row held only that one
child, so both old addresses of the same text open the same row. After the edit 6,794 of 6,795 rows open
their row on v0.1.36, as on v0.1.33. This is the only change to the table.

**The table is no longer purely the published tree.** The other decision says the table was made once, by
`cargo xtask legacy-sections`, from the `books.json` and `catalog.json` of the live library at corpus
v0.1.31, and that a pin never changes it. This edit is the one exception: one value of one key was changed
by hand. Two consequences follow.

- `cargo xtask legacy-sections --check` compares the committed file with a table built from a published
  library. It reports a difference at this one row. Nothing in `cargo xtask check` or CI runs the
  command, because it needs the published `books.json` and `catalog.json`.
- Do not regenerate the file with `cargo xtask legacy-sections`: it would put `row-h-unpaged--h-ranger`
  back. Regenerate only when a reason exists to rebuild the table, and apply this edit again.

The file carries no hash or manifest entry of its own; the compiler writes its bytes into the library as
`legacy-sections.json`, so the library's own manifest hash covers the edit.

**Test.** `the_old_ranger_grouping_address_of_the_elephant_in_the_room_opens_the_ranger_revision_row` in
`crates/content-compiler/src/legacy_sections.rs` reads the cached pinned corpus, builds its books, and
resolves `toc-1-1` and `toc-1-1-1` through `resolve_legacy_anchor`, the function the client uses. Both
open `row-ref-rule-class-ranger-ranger-teitr`. CI fails when the cache is missing; a local run without
it skips the test.

**Format impact.** None. `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay as they are, and the table
format stays 1.
