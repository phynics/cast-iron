---
title: "Race groups"
date: 2026-11-10
status: current
superseded_by: []
issues: [196]
prs: []
area: content
---

# Race groups, 2026-11-10

Refs [#196](https://github.com/phynics/cast-iron/issues/196). The corpus already carried a
`race_group` facet label in the client, but no canonical record held the value.

- **One optional `race_group` field on the entry.** It is `core_race`, `featured_race`,
  `uncommon_race`, or `other_race`. The first three are the Advanced Race Guide's printed race
  chapters; `other_race` is a race printed outside them. `corpus-check` requires the field on
  every typed `ref/race/` entry once any typed race carries it, so the corpus pinned before the
  data landed still compiles, and it rejects the field on every other kind. A carried legacy
  race keeps its published id without a group.
- **The group is entry metadata, not a typed field.** A race still has `[fields] kind = "none"`.
  The field is additive: an older library omits it, and an older reader ignores the unknown key,
  so `SCHEMA_VERSION` stays 4 and the corpus format stays 1. This follows the printed-citation
  decision.
- **The compiler derives the facet.** `canonical_facets` reads the entry field and emits
  `race_group`, so the existing Races filter shows its groups and counts. No client state or
  search index format changes.
- **The seven Core races take the Advanced Race Guide's Core group.** Their records also appear
  in the Core Rulebook, but one record has one printed group; the ARG chapter is the grouping
  the filter follows. `other_race` replaces the old `monster_race` label for the races outside
  the three ARG chapters, because many of them are playable races from other books. The old
  `standard_race` and `monster_race` labels stay readable for an already installed library.
