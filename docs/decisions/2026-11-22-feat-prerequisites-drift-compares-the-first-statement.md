---
title: "Feat prerequisites drift compares the first printed statement"
date: 2026-11-22
status: current
superseded_by: []
issues: [615]
prs: []
area: corpus
---

# Feat prerequisites drift compares the first printed statement, 2026-11-22

The drift check compares a feat's typed fields with the labelled lines of the feat's normal part.
It used to accept a field that matched any statement of the same label.

- **`prerequisites` matches the first statement only.** A feat has one prerequisites line of its
  own. A later `**Prerequisites:**` line in the normal part belongs to a sub-option, and structure
  cannot tell the two apart. A field that equals only a later statement is now drift.
- **`benefit`, `normal` and `special` keep any-match.** The corpus has feats whose field
  correctly matches a later statement: `ref/feat/virtuous-creed` (Benefit, statement 7 of 7),
  `ref/feat/advanced-weapon-training` (Special, 2 of 2) and `ref/feat/draconic-heritage`
  (Special, 4 of 4).
- **A printed label with no typed field is a warning.** The check compares only fields that
  exist. A feat whose normal part prints `**Prerequisites:**` or `**Special:**` but has no such
  field is reported once per run, with a count and examples. It becomes an error when the corpus
  has none.

On corpus v0.1.39, which carries the Beast Speaker fix (data `be498d7`), the first-statement rule changes nothing: total drift stays 0. On v0.1.38 it reported the one known Beast Speaker drift, so this rule merged after the v0.1.39 pin.
