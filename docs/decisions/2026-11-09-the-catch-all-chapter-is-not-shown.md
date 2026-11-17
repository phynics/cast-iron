---
title: "The catch-all chapter is not shown"
date: 2026-11-09
status: current
superseded_by: []
issues: [359, 363]
prs: []
area: ui
---

# The catch-all chapter is not shown, 2026-11-09

Closes [#363](https://github.com/phynics/cast-iron/issues/363), part of
[#359](https://github.com/phynics/cast-iron/issues/359). Pins corpus
[`v0.1.18`](https://github.com/phynics/cast-iron-data/releases/tag/v0.1.18), which carries the
placement of the 21 catch-all-only rules (data #63), the Game Mastery Guide book merge (data
#62), and the third-party contamination fixes (data #65).

- **The catch-all chapter has no reader route.** The corpus chapter titled
  `New entries (page order)` is not a Contents row, has no chapter page, and is left out of the
  previous/next order. `views::book::chapter_label` and the `More from this book` label are
  removed: nothing renames it because nothing shows it.
- **A book with no other chapter lists what it can.** `Contents` is empty; the book page keeps
  `In this book` and its list-page kind links, and shows `This book has no entries to show.`
  only when it has no list-page kind either.
- **The schema owns the rule.** `cast_iron_schema::books::shows_in_contents` returns false for
  the catch-all, and `contents_roots` no longer lifts its rows to the top level. The compiler's
  book-section documents and the client's chapter rows both come from those functions, so they
  cannot disagree.
- **Reachability is now a gate.** `cargo xtask content` exits non-zero when any entry has no
  route without the catch-all, instead of warning. On corpus `v0.1.18` the report shows 0
  unreachable entries of 31,807, so the pin passes.
