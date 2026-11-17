---
title: "Full Pf Data corpus second pass"
date: 2026-11-05
kind: record
---

# Full Pf Data corpus second pass, 2026-11-05

A second local pass over pin `08ea71f24375908d0190c822fd1fd080797667a9` removed
the largest remaining conversion causes on the canonical main:

| Measure | Before | After |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 28,496 | 29,024 |
| Findings | 1,579 | 858 |
| High findings | 1,244 | 523 |
| Medium findings | 335 | 335 |
| Excluded records | 1,489 | 961 |
| Mandatory fixtures | 6/6 | 6/6 |

The deliberate `::aff block has no name` rejection remains 332 medium findings.
The remaining high findings are smaller families, led by the inline `@list`
token, unresolved ability anchors such as `#/fetch`, and a few unsupported
containers. Two full `cargo xtask data-import` runs produced byte-identical
`full.json`, `full-search.bin`, `inventory.json`, `books.json`,
`book-coverage.json`, `migration-full.json`, and `migration-full.md`.
`cargo xtask data-report` reports 0 findings and 6/6 fixtures. Two canonical
`cargo xtask content` runs produced a byte-identical `generated/content` tree,
and `python3 scripts/check-library-upgrade.py generated/psrd generated/content`
exited 0 with an empty alias-problem and dangling-reference set.

The default canonical library moves from 29,515 entries at
`vd7596846128e349b` to 30,000 entries at `v46b91fe4b0c71e28`, a net gain of 485
entries. The PSRD carry-over library built with `cargo xtask content --psrd`
remains 9,968 entries at `vdb43853600b8ed8b`. Schema 3 and search format 1 are
unchanged. This is local evidence; the full migration is still blocked and not
shipped. See [decisions](../decisions/2026-11-05-full-pf-data-corpus-second-root-cause-pass.md).
