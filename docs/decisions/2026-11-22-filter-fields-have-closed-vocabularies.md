---
title: "Filter fields have closed vocabularies"
date: 2026-11-22
status: current
superseded_by: []
issues: [593, 594]
prs: []
area: corpus
---

# Filter fields have closed vocabularies, 2026-11-22

Refs [#594](https://github.com/phynics/cast-iron/issues/594) and [#593](https://github.com/phynics/cast-iron/issues/593), and phynics/cast-iron-data#169 (owner decision of 2026-11-22). It amends one bullet of [filter menus for the other kinds and classes under headings](2026-11-07-filter-menus-for-the-other-kinds-and-classes-under-headings.md), "Values are shown as the corpus has them ... These are corpus questions".

## Decision

Any field whose values a filter menu or a search facet group shows has a closed vocabulary, and `corpus-check` validates it. The client still does not rename facet values. A value that does not belong to the field is a corpus error, not something the menu shows. `race_group` was the first such field; this entry makes it the rule.

The list lives in `crates/corpus/src/check.rs`, one constant beside the check, with a comment that names the ticket. The corpus repository keeps the print citation of each value.

- **Feat types** (`FEAT_TYPES`, 37 values). Print is the authority: a book abbreviation (`BoA`, `UW`, `UM`, `MC`), a title qualifier (`Taldor Variant`) and a race (`Dhampir`) are not types. A feat that print gives no type tag stays untyped in the corpus, because print does not label it `General`. The compiler gives it the facet `feat_type = General`, so the Type menu reaches every feat without the corpus inventing a label. A feat that the corpus types `General` has the same facet.
- **Creature size** (`SIZE_CATEGORIES`, nine categories from `fine` to `colossal`). The compiler lower-cases `size` for its facet, so the check compares without case; the corpus keeps the case of the printed stat block. A creature that print gives no size (the sample AIs of the Technology Guide) has no `size`.
- **Later:** item slot and affliction types (#171). Item category, creature type facet and trait category get the check when someone next works on them.

## Roll-out

The check starts as a warning with a count per field, the way the uncited book memberships did (#461), and becomes an error when the pinned corpus has none. The pinned corpus v0.1.37 has 9 feat type values and 2 sizes outside the lists; the corpus change of phynics/cast-iron-data#174 clears them. The pin bump that follows it, and the switch from warning to error, are the next steps.

No format changes: the facet value is data, not a format, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
