---
title: "Full Pf Data corpus root-cause findings"
date: 2026-11-04
kind: record
---

# Full Pf Data corpus root-cause findings, 2026-11-04

[#18](https://github.com/phynics/cast-iron/issues/18) now has a full-corpus run
that reports root causes instead of reference cascades. Local 2026-11-04 results
against pin `08ea71f24375908d0190c822fd1fd080797667a9`:

| Measure | Before | After |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 24,610 | 27,853 |
| Findings | 49,035 | 2,174 |
| High findings | 47,605 | 1,831 |
| Medium findings | 1,430 | 343 |
| Mandatory fixtures | 1/6 | 6/6 |

The largest previous causes were 42,611 unresolved references, of which 39,207
pointed at records that were selected but had failed conversion. Those links are
now `reference-to-failed-record` coverage under the target's own conversion
finding. The remaining high findings are distinct conversion causes: 161 records
without name or publication metadata, 152 `::trait` metadata records, 147
`::prereq` records, and smaller families such as `::elephant`, `::room`,
`::haunt`, `::trap`, and `::drug`. The deliberate `::aff block has no name`
rejection stays counted as 332 medium findings.

Representative mode still reports 0 findings and 6/6 fixtures, and the tracked
`reports/migration-test.*` and `reports/migration-coverage.json` are unchanged.
Two `cargo xtask data-import` runs produced byte-identical reports and generated
full-mode output. `cargo xtask content` produced the same `vff56b0e6310710c1`
content version and a byte-identical tree to `main`, so production PSRD content
is unchanged. Schema 3 and search format 1 are unchanged. This is local evidence;
the full migration is still blocked and not shipped.
