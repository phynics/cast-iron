---
title: "List rows name their parent, and the fold joins only printings of one rule"
date: 2026-11-22
status: current
superseded_by: []
issues: [575, 594]
prs: []
area: ui
---

# List rows name their parent, and the fold joins only printings of one rule, 2026-11-22

Refs [#575](https://github.com/phynics/cast-iron/issues/575) and the survey
[#594](https://github.com/phynics/cast-iron/issues/594). The design is in DESIGN.md, Views > Repeated
titles. These are the choices that the code does not show.

- **The fold key is kind, normalised title and parent.** Where the parent comes from decides when a
  list can know it: a racial trait's race (`part`) and an archetype's class (`archetype`) come from
  `relations.json`, a class option's group from `option-groups.json`, an affliction's parent from the
  catalog stub. The rows render at once with the book; the parent joins the line, the folds and the
  order when the lookup arrives. No compiler, index or schema change, so no format version moves and
  no `EntryId` changes.
- **A fold names its parent in its summary.** The issue states the fold key and the row line, not the
  fold's own line. After the change 49 racial-trait folds hold one race's trait printed in two books;
  without a parent they would read as the same `{title} · 2 publication entries` as the folds of other
  races. The summary therefore carries the same muted text a row carries, without the book.
- **`Alternate` starts the muted line of an alternate racial trait** (`Alternate · Dwarf · Advanced
  Race Guide`), as `2026-11-22-racial-trait-race-facet` deferred to this change. It is read from the id
  (the `alternate-racial-traits` segment, the rule the race page uses), so it shows in a list that the
  Race menu narrows too (`Alternate · Advanced Race Guide`), where the race is left out.
- **A page in a variant link is read on the first open.** The catalog stub has no citation page, so a
  fold with two printings in one book reads their shards when it first opens, and only then. In the
  pinned corpus no fold has two printings with one primary book, so the path is covered by native tests of the
  label rules, not by a browser test.
- **The prestige-class requirements panel keeps `{Book}, {Kind}`.** It lists feats and skills, so the
  page does not name the kind.
