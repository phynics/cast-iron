---
title: "A relation label is an enum"
date: 2026-11-17
status: current
superseded_by: []
issues: [251, 449]
prs: []
area: content
---

# A relation label is an enum, 2026-11-17

Issue #449, from the #251 design pass. `Relation.label` and the fourth element of `RelationEdge` were an untyped `Option<String>` with seven values. They are now `Option<RelationLabel>` (`alternate`, `changes`, `merged`, `mythic`, `or`, `removed`, `stamina`).

- **The wire form is unchanged.** Each variant serializes as its old lowercase string, so `SCHEMA_VERSION` stays. The compiled library is byte-identical before and after (`diff -r` of two `cargo xtask content` runs).
- **The variants are in alphabetical order on purpose.** `RelationEdge` sorts by its derived `Ord`. Alphabetical declaration keeps the order that the strings had, so an edge list sorts as before. A schema test pins it.
- **`RelationKind::allows(Option<RelationLabel>)` is the closed table.** `corpus-check` reads it instead of its own `matches!`. A test lists every accepted pair and checks that the other pairs of the six kinds and eight label choices are refused.
- **An unknown label is now a parse error.** Before, the reader accepted any string and `corpus-check` reported it. The entry file now fails to read, and the error names the file and the allowed values. The client matches on the enum; a label that is not an add-on label has an explicit arm.
