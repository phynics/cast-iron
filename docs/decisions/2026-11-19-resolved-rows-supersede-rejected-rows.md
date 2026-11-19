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

- A `resolved` row supersedes every earlier `excluded` or `rejected` row with the same `path`. This replaces only the sentence of the 2026-11-18 decision that excludes `rejected` rows; the rest of that decision stands.
- The coverage report lists a resolved path as neither excluded nor rejected (`Exclusions::active_excluded` and `Exclusions::active_rejected`). The old rows stay in the file as history.
- `read_exclusions` still refuses an orphan row (no earlier `excluded` or `rejected` row with that path), a repeated `resolved` path, and a row with no `entries` or an empty `reason`. `corpus-check` still refuses an entry id that does not exist.
- Nothing else changes. The key is still optional and additive, the corpus format and `SCHEMA_VERSION` stay as they were, and the output for a corpus with no `resolved` row is byte for byte the same.
- The corpus repository builds its checker from app `main`, so this change lands before the data pull request.
