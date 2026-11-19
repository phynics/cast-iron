---
title: "Superseding exclusion rows"
date: 2026-11-18
status: partly-superseded
superseded_by: ["2026-11-19-resolved-rows-supersede-rejected-rows"]
issues: [36]
prs: []
area: corpus
---

# Superseding exclusion rows, 2026-11-18

`exclusions.toml` is append-only (corpus `CONTRIBUTING.md`). Data pull request #104 shipped eight records that eight older `excluded` rows still described as "out of scope … tracked in #36" (Thought Thief, the six Ultimate Cantrips and Primitive Human). The rows could not be edited, and a row that says a record is left out while the library ships it misleads a reader of the coverage report.

The project owner chose to keep the append-only rule and to add a superseding row.

## Decision

- `exclusions.toml` gains an optional `[[resolved]]` table with `path`, `entries` and `reason`. `path` is the exact path of an earlier `excluded` row, `entries` the entry ids that now hold the record, and `reason` cites the issue. The old row stays as history.
- The coverage report lists only the `excluded` rows that no `resolved` row supersedes (`Exclusions::active_excluded`). A resolved path is not shown as excluded. The newest word on a path wins.
- `read_exclusions` refuses an orphan row (no earlier `excluded` row with that path), a repeated `resolved` path, and a row with no `entries` or an empty `reason`. `corpus-check` refuses an entry id that does not exist. A `resolved` row never supersedes a `rejected` row.
- The key is additive and skipped when empty, so a file without it reads and writes back byte for byte, and its compiled output does not change (AGENTS rule 5). The corpus format and `SCHEMA_VERSION` do not change: `coverage.json` has the same shape, minus the resolved rows. The corpus format number gates file layout and migrations, and this adds one optional table to an existing file, as `publication_licenses` did for `legal.toml` ([Paizo Blog source bases under the Community Use Policy](2026-11-18-paizo-blog-source-bases-under-cup.md)).
- A tool that predates this change rejects the key, because `Exclusions` denies unknown fields. The corpus repository builds its checker from app `main`, so the app change lands first.

The data repository appends the eight `resolved` rows after this change merges.
