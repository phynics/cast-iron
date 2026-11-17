---
title: "Pf Data Section 15 notice matching"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: licensing
---

# Pf Data Section 15 notice matching, 2026-11-04

- **Notice coverage is evidence, not publication.** `cargo xtask data-legal` reads the pinned `OPENGAMECONTENT.md`, matches every publication referenced by an in-scope record, and writes deterministic `reports/migration-legal.json`. It does not edit the shipped legal page or import content. `cargo xtask data-report` also writes the report and records the counts under `legal` in `reports/migration-coverage.json`, so CI compares the same evidence.
- **Matching order is exact, reviewed alias, then normalizer.** Exact means the same ASCII-alphanumeric token sequence. The normalizer removes known Pathfinder and GameMastery line prefixes, volume/scenario words, short module codes, and a leading article after an adventure number; it also treats `and` and `&` as the same connector. When several Section 15 entries share a title key, every distinct notice applies: the Core Rulebook carries both its 2009 and 2010 printing notices. No candidate is dropped in favour of the first. A source title is never matched by similarity score.
- **Aliases are a reviewed allowlist with hard targets.** `sources/pathfinder-data-1-e/notice-aliases.json` maps an exact publication title to an exact Section 15 title. An alias whose publication is absent or whose target is missing fails the report, so a pin change cannot leave a stale alias silently passing.
- **The gate is separate from the report.** `pf_legal::gate` fails a later production build while any publication with in-scope records has no matched notice. The report command itself stays successful so a blocked migration still writes its evidence and CI can compare the committed snapshot.
- **Out-of-scope publication labels stay visible.** Publications named by `out-of-scope.json` are not matched or gated, but their record counts remain in the report under `excluded_publications`. Records excluded by the inventory stay out of all counts. This follows the rule that excluded material stays in coverage rather than disappearing.
