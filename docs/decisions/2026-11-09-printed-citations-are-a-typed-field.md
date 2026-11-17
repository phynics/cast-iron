---
title: "Printed citations are a typed field"
date: 2026-11-09
status: current
superseded_by: []
issues: [357]
prs: []
area: corpus
---

# Printed citations are a typed field, 2026-11-09

Refs [#357](https://github.com/phynics/cast-iron/issues/357). The data migration is a separate
pull request in `phynics/cast-iron-data`; app rendering is a later step.

- **The corpus gains an optional repeated `[[sources]]` table.** A row is
  `publication = "<publication id>"` plus an optional `page`. The rows are in printed order and
  replace the leading `**Sources:**` paragraph. `corpus-check` requires each `publication` to
  exist in `publications.toml` and to be listed in the entry's `source_ids`.
- **`page` is a string, not a number plus range.** The printed lines carry `221`, `12-14`,
  `263-4` and non-numeric pages such as `inside cover`. One optional string keeps every value
  as printed and makes the migration proof exact; splitting a range would invent structure the
  source does not have.
- **`source_ids` stays.** It is the permanent publication list that active-book filtering uses.
  The new rows add the page, not a second identity for the book.
- **The serialized entry gains an optional `sources` array and `SCHEMA_VERSION` stays 4.**
  Without rows the key is omitted; with rows an older reader ignores an unknown key. This is
  additive, like the optional catalog fields.
- **The corpus format stays 1.** The key is optional, the tool that reads it ships with the
  corpus pin, and the data migration changes only entry files. If a future release must let an
  older tool read a newer corpus, that release adds the format-2 migration step.
