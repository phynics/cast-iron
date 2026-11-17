---
title: "Canonical page-ordered book index"
date: 2026-11-05
kind: record
---

# Canonical page-ordered book index, 2026-11-05

[#18](https://github.com/phynics/cast-iron/issues/18) now extracts page attribution
from the screened Pf Data `‹SOURCE Publication/page›` lines into the canonical
converter and builds a page-ordered `books.json` for the `ref/` library. The full
pinned corpus (pin `08ea71f24375908d0190c822fd1fd080797667a9`, 27,853 converted
entries) yields:

| Measure | Full corpus | Representative `data-report` |
|---|---:|---:|
| Books | 454 | 49 |
| Entries | 27,853 | 42 |
| Placed by page | 24,129 | 37 |
| Unpaged | 3,724 | 5 |
| In more than one book | 4,872 | 9 |
| Without publication attribution | 0 | 0 |
| Placements | 35,700 | 82 |
| Paged placements | 29,738 | 47 |
| Unpaged placements | 5,962 | 35 |

The Core Rulebook is ordered by page: `Casting Spells` at page 206 follows
`How Combat Works` at page 178 in the flattened book. Two `build_canonical` runs
over one full conversion are byte-identical, and `cargo xtask data-report`
reproduces the tracked representative reports. The canonical index has no curated
chapters yet; `book-order.toml` matches PSRD urls, so its chapter list remains a
later refinement. At that time the default `cargo xtask content` produced
`vdb43853600b8ed8b` with 9,968 entries, byte-identical to the baseline production
build; that library is now `cargo xtask content --psrd`. Schema 3 and search format
1 are unchanged. This is local evidence; the full migration remained blocked and
was not shipped at that time.
