---
title: "Adventure series fields stay optional"
date: 2026-11-09
status: current
superseded_by: []
issues: [159]
prs: []
area: content
---

# Adventure series fields stay optional, 2026-11-09

Refs [#159](https://github.com/phynics/cast-iron/issues/159). Owner decisions recorded in the
issue on 2026-11-09. The data lives in `phynics/cast-iron-data`; this repository owns the format,
the checker and the schema.

- **Only three groupings.** Adventure Paths group by path name and volume, Modules are one line
  ordered by publication date, and Pathfinder Society groups by season and scenario. Companion
  and Campaign Setting sub-lines are out of scope until asked for.
- **The fields are optional in `publications.toml`.** A publication without `series_kind` belongs
  to no series. The data comes from a reviewed script that parses the titles and lists every
  Adventure it cannot parse for review, so the corpus never guesses a grouping.
- **The shape is typed per kind.** `series_kind` is `adventure-path`, `module` or `society`.
  An Adventure Path needs `series` and a positive integer `series_number`; a Module carries no
  other series field; Society needs `season` and, when present, a `<season>-NN` scenario number.
  `corpus-check` rejects the other combinations, and the canonical writer puts the fields after
  `publication_date` and before `third_party`.
- **No `SCHEMA_VERSION` bump.** The fields are additive optional `SourceInfo` members, absent
  from an older catalog, so schema 4 still reads them. The grouping UI is a later change.
