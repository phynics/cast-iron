---
title: "Pf Data class conversion"
date: 2026-11-05
kind: record
---

# Pf Data class conversion, 2026-11-05

Local results against the same pin after the class-conversion fixes:

| Measure | 2026-11-04 | 2026-11-05 |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 27,853 | 28,496 |
| Findings | 2,174 | 1,579 |
| High findings | 1,831 | 1,244 |
| Medium findings | 343 | 335 |
| Mandatory fixtures | 6/6 | 6/6 |

Every class and prestige-class description record now converts, including the
core classes and the derived class-archetype rules that previously skipped when
their host was missing. The remaining causes are non-class: 161 records without
name or publication metadata, 152 `::trait` metadata records, and the
`::room`, `::haunt`, `::trap`, `::drug`, and `::spelllist` families. The
`::prereq` and `::elephant` families are resolved. Representative mode still
reports 0 findings. Two full runs produced byte-identical reports and generated
output, and `cargo xtask content` remains byte-identical to `origin/main`.
Schema 3, search format 1, and production content are unchanged. This is local
evidence; the full migration remains blocked and unshipped.
