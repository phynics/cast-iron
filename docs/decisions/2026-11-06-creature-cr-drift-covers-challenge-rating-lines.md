---
title: "Creature CR drift covers Challenge rating lines"
date: 2026-11-06
status: current
superseded_by: []
issues: [198]
prs: []
area: corpus
---

# Creature CR drift covers Challenge rating lines, 2026-11-06

Refs [#198](https://github.com/phynics/cast-iron/issues/198). The checker is
`crates/corpus/src/drift.rs`.

- **A creature's `cr` field is compared with its `**Challenge rating:**` line, not only a
  stat block `CR` row.** Most creatures state the rating as the bold labelled line and have no
  fenced stat block, so 3,407 of the 3,718 creatures with a `cr` field were not compared
  before this.
- **The comparison parses the rating.** `ChallengeRating::parse` turns both sides into
  24ths, so a fraction such as `1/2` agrees with the same fraction and a changed fraction is
  reported like any other disagreement. A statement that does not parse (`variable`, a
  template adjustment) falls back to comparing the displayed text.
- **Several rating lines are ambiguous.** An entry with more than one line (the elementals,
  ettercap, some vehicle and template entries) usually has no `cr` field, so nothing is
  compared. When a field does exist, the field agrees if any of the statements does, the
  same rule a feat uses for a repeated label.
