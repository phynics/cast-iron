---
title: "A racial trait carries its race as a search facet"
date: 2026-11-22
status: current
superseded_by: []
issues: [577, 594]
prs: []
area: search
---

# A racial trait carries its race as a search facet, 2026-11-22

Refs [#577](https://github.com/phynics/cast-iron/issues/577) and the survey
[#594](https://github.com/phynics/cast-iron/issues/594).

- **The compiler emits `racial_trait_race`.** The value is the last segment of the id that the
  trait's `part` relation targets (`dwarf` for `ref/race/dwarf`), as an archetype's
  `archetype_class` comes from its `archetype` relation. Only a racial trait gets it. It is
  additive, like `group` in `2026-11-22-class-option-group-in-the-search-index`: the index layout
  is unchanged, so `INDEX_FORMAT_VERSION` stays 2 and `SCHEMA_VERSION` does not change.
- **The list page has one menu, `Race`.** It follows the filter menu rules, with a find field
  because the 37 races with a trait exceed 10 values. The route is
  `#/browse/kind/racial_trait?facet=racial_trait_race:dwarf`. Quick search takes the race as its
  scope (`Search Dwarf racial traits`), and the search filter panel offers `Race` for the kind.
- **Labels are the catalog's race titles.** The slug does not print as the title (`half-elf` is
  `Half-Elf`, `being-of-ib` is `Being of Ib`), so the client registers the titles of the catalog's
  races with the facet labels when the library opens.
- **Old libraries.** A library compiled before this has no `racial_trait_race` values. The menu
  has no value and is hidden, as any filter menu whose facet is empty; the list shows all racial
  traits.
- **Not in this change.** The `Alternate` start of an alternate trait's muted line and the
  dropped race on that line belong to the row lines of #575, which this change leaves alone.
