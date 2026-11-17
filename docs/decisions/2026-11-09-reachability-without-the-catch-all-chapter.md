---
title: "Reachability without the catch-all chapter"
date: 2026-11-09
status: current
superseded_by: []
issues: [359, 360, 363]
prs: []
area: content
---

# Reachability without the catch-all chapter, 2026-11-09

Refs [#360](https://github.com/phynics/cast-iron/issues/360), part of
[#359](https://github.com/phynics/cast-iron/issues/359).

- **An entry is reachable when the app gives it a route without the catch-all chapter.** The
  catch-all chapter is the corpus chapter titled `New entries (page order)`. An entry is
  reachable when at least one of these holds: a book tree places it outside that chapter; its
  kind has a list page, which is every kind except Rules; a reachable entry has a body link or
  a typed relation to it; or it has an add-on, variant or revision relation to a reachable
  base. The link and relation routes repeat until nothing changes, so a chain of links counts
  from its first placed or listed entry.
- **The list-page route assumes every book is active.** The compiler has no reader selection, so
  a non-rule entry counts as reachable when its kind has a list page, even if a reader turned
  its book off. Third-party books are off by default, so a non-rule entry that only a
  third-party catch-all chapter places is still reachable in this report. A revision-hidden
  entry also counts: the unfiltered list hides a removed, merged, or revised base only while the
  revision's book is active, and the book's own kind list still shows the entry. The report
  describes the installed library, not one reader's active books.
- **The report is `reachability.json` in the generated reports.** It lists each unreachable
  entry with its kind, source ids and reason, and gives counts per book and kind. The top-level
  `unreachable` total counts distinct entries. `by_book` counts catch-all placements, so its
  rows can add to more than the distinct total when one entry sits in several books; the report
  records that sum as `unreachable_placements` and explains it in `by_book_note`. The report also
  confirms that the search index holds every entry under its exact title. Entries, books and
  kinds are sorted, and the report holds no timestamps, so two compiles produce the same bytes.
- **The report is an error.** `cargo xtask content` prints the count per book and exits
  non-zero while unreachable entries remain, so the content pin cannot land with an entry that
  the app cannot reach. [#363](https://github.com/phynics/cast-iron/issues/363) turned the
  earlier warning into this error after the corpus placed or linked every rule.
