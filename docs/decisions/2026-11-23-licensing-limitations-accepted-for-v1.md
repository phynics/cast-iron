---
title: "Licensing: two standing limitations accepted for v1"
date: 2026-11-23
status: current
superseded_by: []
issues: [34]
prs: []
area: licensing
---

# Licensing: two standing limitations accepted for v1, 2026-11-23

Closes [#34](https://github.com/phynics/cast-iron/issues/34). The owner decided on 2026-11-23 to accept
the two open items of #34 as standing limitations, with no further search before v1. Counts are from the
corpus `main` at `2599cc4b`.

- **Three publications keep `notice_pending`.** *Hodgepodge from the Grand Lodge!* (2 archetypes), the
  *Kingmaker Player's Guide* (7 traits) and the *Serpent's Skull Player's Guide* (10 traits) have no
  Section 15 notice in any pinned source. Their 19 entries, each a member of that publication only, keep
  shipping as assumed Open Game Content with the general Paizo attribution, as decided in
  [owner-copy notices and notice-pending publications](2026-11-05-owner-copy-notices-and-notice-pending-publications.md).
  The legal page keeps the list under "Section 15 notice pending".
- **The PSRD-Data dataset has no license of its own.** 1,905 entries under `ref/carried/` derive from it.
  They rest on Paizo's Open Game Content designation of the text, not on a grant from the dataset's
  repository ([licensing audit](../research/licensing-audit.md), finding 1). Only a replacement of those
  entries from another source would remove this limitation, and none is planned.
- **Not chosen.** The owner did not exclude the 19 entries, and did not ask for one more search for the two
  guides' notices. A Community-Use-only basis for *Hodgepodge from the Grand Lodge!* (rule 12 of
  `AGENTS.md`) was not evaluated.
- **What reopens this.** A Section 15 line from the publication itself resolves a pending row: add it as
  rule 12 says and remove the title from `notice_pending` in the corpus `legal.toml`. A request from a
  rights holder, or a change in the licensing of a dataset, needs a new issue.
- **Unchanged.** Rules 10 to 12 of `AGENTS.md`. Comics stay excluded, and ORC-licensed PF2 material stays
  out of scope.
