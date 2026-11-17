---
title: "Feature anchors on archetypes and class options"
date: 2026-11-10
status: current
superseded_by: []
issues: [251, 252]
prs: []
area: content
---

# Feature anchors on archetypes and class options, 2026-11-10

Refs [#252](https://github.com/phynics/cast-iron/issues/252). Part 1 added stable headings
for 658 class features across all 44 classes in `cast-iron-data`.

- **The typed printed names keep their meaning.** `replaces` and `alters` stay the printed
  feature names in print order. The new optional `replaces_features` and `alters_features`
  lists carry the resolved heading anchors of the base class entry, in the same order.
- **A key must be an anchor of the base class.** An archetype's class is its `archetype`
  relation target. A class option's classes are the `classes` of its group in
  `option-groups.toml`; when a group lists several classes, a key may name a heading in any
  of them. `corpus-check` fails when a key is not such an anchor.
- **Agreement with the printed names is a warning.** Resolution strips case, punctuation,
  and an ability tag (`(Ex)`, `(Su)`, `(Sp)`), then compares the slug with the heading keys.
  A printed name that does not resolve confidently is omitted instead of guessed, so the
  feature list can be shorter than the printed list. A non-empty feature list warns when it
  differs from the confident matches in print order; it does not fail the run while
  per-power trades remain unresolved.
- **The flat list describes a whole-feature trade.** The per-power shape for oaths,
  domains, bloodlines and spirits ([#251](https://github.com/phynics/cast-iron/issues/251))
  will add a separate optional pair beside these fields. This list keeps its meaning, and
  the new field is optional, so `SCHEMA_VERSION` stays 4.
