---
title: "The Quick reference row in the empty Quick search"
date: 2026-11-17
status: current
superseded_by: []
issues: [421, 438]
prs: []
area: search
---

# The Quick reference row in the empty Quick search, 2026-11-17

Closes [#438](https://github.com/phynics/cast-iron/issues/438). Part of [#421](https://github.com/phynics/cast-iron/issues/421).

- **A row in the list, not a button or a chip row.** The empty layer already offers destinations as rows (Recent searches, Recently viewed), so Quick reference is one more row in the same form: `Quick reference`, then the muted `Conditions · Actions · Combat modifiers`. It reuses the highlight, the option ids and `aria-activedescendant`, so Down then Enter reaches it and no second focus model exists.
- **First, in a group of its own.** The mid-play lookup is the reason the page exists, and a row that moves with the reader's history would not be where the thumb expects it. It is a group (`GroupKind::Reference`) with a screen-reader heading and no visible heading, as the suggestion popup does, because one row needs no label above it. It shifts every highlight index of the empty list by one, so the specs that count rows were updated.
- **Hidden while a scope is set.** A scope means the reader opened the layer from a list to search that list (`Search feats`). A page link at the top of a scoped list would be an answer to a different question. Removing the scope brings it back, since the layer re-derives the list from the scope signal.
- **A row that is not a search.** Opening it navigates to `#/quick` through `go`, not through `search`/`run`, so nothing is written to Recent searches. The diagnostics sample (#436) is recorded only by the suggestion lookup for non-empty text, which the row never reaches, so it adds no sample.
- **The muted line is fixed text.** It names the page's first three groups and does not follow the page's constant. The page's fourth group, `Every session`, is left out for length.
