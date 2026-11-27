---
title: "Class features in print order, set as heads (layout A)"
date: 2026-11-27
status: current
superseded_by: []
issues: [739]
prs: []
area: ui
---

# Class features in print order, set as heads (layout A), 2026-11-27

The owner chose how a converted class feature looks (#739, part 1). It refines the head of
`2026-11-24-feature-heads-and-search-aliases`; the data and the rule for "which heading is a feature"
do not change.

## Decision

- **Layout A: print order, each feature a head.** A feature reads as its printed paragraphs under one
  head, in the book's order. Layout B (features grouped by level) is rejected: the book does not
  group them, and a grouping would hide the order that "This ability replaces ..." sentences rely on.
- **The head.** The name in the display serif, bold. The ability tag, `(Ex)`, `(Su)`, `(Sp)` or a
  combination such as `(Ex or Sp)`, is its own element after the name: body type at the body size,
  italic, muted ink, not bold. `Level N` follows a dotted leader. A feature with no printed level shows
  neither leader nor level.
- **The tag is split for display only.** The heading text, its id and anchor, `?a=` links, the search
  aliases and `headings::feature_of` stay as they were. `headings::split_heading_tag` (schema crate, on
  top of `power::split_ability_tag`) does the split; the text of name and tag, joined by one space, is
  the printed heading, so the accessible name is unchanged.
- **Separator.** A hairline rule (`--line`, 1px) sits between one listed feature and the next. There is
  none above the first listed feature, above a heading that the corpus does not list, or inside a
  feature. In CSS: the top border of a `.feature-head` that has an earlier feature head in the body.
- **No invented rows.** The printed text carries any scaling. A per-level scaling line is not part of
  this design.

The Options section for selectable abilities is part 2 of #739 and is not decided here.
