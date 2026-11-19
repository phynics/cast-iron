---
title: "Resolved rows supersede rejected rows"
date: 2026-11-19
status: current
superseded_by: []
issues: [36]
prs: []
area: corpus
---

# Resolved rows supersede rejected rows, 2026-11-19

[Superseding exclusion rows](2026-11-18-superseding-exclusion-rows.md) let a `[[resolved]]` row supersede an earlier `excluded` row, and said that a `resolved` row never supersedes a `rejected` row. Data pull request #106 ships 17 cleric domains that the converter had recorded as `rejected` rows (phynics/cast-iron-data#103). `exclusions.toml` is still append-only, so those rows cannot be edited, and `corpus-check` refused the `resolved` row for them with "supersedes nothing".

The project owner's rule of 2026-11-18 is the same as before: keep the file append-only, and the coverage report prefers the newest row for a path.

## Decision

- A `resolved` row supersedes one `excluded` or `rejected` row with the same `path`. This replaces only the sentence of the 2026-11-18 decision that excludes `rejected` rows; the rest of that decision stands.
- Several rows can share a path, and a path match alone would hide the others. The pinned corpus has two `rejected` rows for `json/main04.json#traits_race`, and the Death and Repose domains each have an `excluded` and a `rejected` row for one path. So a `resolved` row must pick its row: when more than one row has the `path`, it carries `supersedes`, the start of the `reason` of the row it resolves, and the start must select exactly one row. A row without `supersedes` for an ambiguous path is a read error that names `supersedes`. When one row has the path, nothing changes, so the eight rows of the pinned corpus stay valid. This is the smallest rule that works: `path` plus `reason` is already the identity of a row (a duplicate pair is a read error), and a prefix keeps the text short.
- The coverage report no longer lists a resolved row as excluded or rejected. It lists it under a new `resolved` key of `coverage.json` and a "Resolved records" section of `coverage.md`: path, kind of the old row (`was`), entry ids and reason. Nothing disappears silently (spec section 2). The old rows stay in the file as history. A report with no `resolved` row has neither the key nor the section.
- `read_exclusions` reads the whole file: an orphan row (no `excluded` or `rejected` row with that path, or a `supersedes` that starts no reason of one), an ambiguous row, a row that resolves a row already resolved, and a row with no `entries` or an empty `reason` are errors. That the `resolved` rows come last is a convention that the reader does not check. `corpus-check` still refuses an entry id that does not exist.
- Nothing else changes. The keys are optional and additive, the corpus format and `SCHEMA_VERSION` stay as they were, and a corpus with no `resolved` row gets the same output byte for byte. A corpus with `resolved` rows (the pinned one has eight, from the 2026-11-18 decision) gets the new list, and no other byte changes.
- The corpus repository builds its checker from app `main`, so this change lands before the data pull request.
