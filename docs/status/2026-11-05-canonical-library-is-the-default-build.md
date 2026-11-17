---
title: "Canonical library is the default build"
date: 2026-11-05
kind: record
---

# Canonical library is the default build, 2026-11-05

`main` now builds the canonical Pf Data library by default. `cargo xtask content`
writes `generated/content/`, and `cargo xtask build-site` packages that directory
into `dist/`. PSRD-Data remains the carry-over input and the previous published
library: `cargo xtask content --psrd` writes `generated/psrd/`, and
`cargo xtask build-site --psrd` packages it. `--canonical` is still accepted as the
explicit spelling of the default.

**This is a default switch on `main`, not a release.** The `release` branch and the
live GitHub Pages site still publish the PSRD library (`vdb43853600b8ed8b`, 9,968
entries). Promotion needs the owner's approval and follows `docs/releasing.md`:
publish and verify the alias-capable shell on the PSRD release first, then promote
the commit that makes canonical the default (design section 6.5). Merging this
switch publishes nothing.

Local verification on 2026-11-05 against the pinned PSRD-Data and Pf Data commits:

| Check | Result |
|---|---|
| Canonical default, two runs | Byte-identical: `vd7596846128e349b`, 29,515 entries, 206,972,160 bytes; content tree hash `eb8d2e60a947ca9b70c14b0e3bff9f251705a38f5026c0b4d0dc8b6a10d6846c` |
| PSRD `--psrd`, two runs | Byte-identical: `vdb43853600b8ed8b`, 9,968 entries; content tree hash `4425ab68f1a668e926be7017457be6d4397cb597f41e7be68b82aaf5ddd46b07` |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0: 9,968 old ids, 0 missing, 0 unresolvable, 0 alias problems, 0 unexpected title changes, 0 wrong class categories, 0 dangling references; 5,710 direct, 1,898 shared, 115 derived, and 2,245 carried |
| `check-publication.py` on `dist/content` | Exit 0: 29,515 entries, 397 publications, 0 issues |
| Full browser suite | 110/110 passed: chromium 54, webkit 54, mobile-chromium 2; the `Licensing` case passed in both desktop projects |
| `cargo xtask check` | Passed (format, Python tests, native and WASM clippy, workspace tests, WASM build) |
| `cargo xtask data-report` | Passed; no change to the tracked `reports/` files |

Schema version 3 and search format 1 are unchanged. The default library's legal page
lists both dataset pins and all 951 Section 15 notices plus the reviewed
notice-pending list. This is local evidence only;
no CI, deployment, or live verification is claimed here.

### Notice coverage, 2026-11-05

Three Section 15 lines from the owner's copies of the books and eight reviewed
notice-pending publications are now pinned. The 453 in-scope publications split
into 434 matched, 8 notice-pending, and 11 comic labels without a notice. The
canonical library has 31,106 entries at `vf32215920f3724aa`; the legal screen
rejects 21 records, every one naming only a comic. Local verification:
`cargo xtask check` passes, two canonical compilations are byte-identical,
`cargo xtask data-report` reproduces the committed reports,
`check-publication.py generated/content` and
`check-library-upgrade.py generated/psrd generated/content` both exit 0, and the
`Licensing` browser test passes on chromium and webkit. No CI, deployment, or
live check is claimed.

### Search facet-chip latency, 2026-11-05

The canonical search view renders 479 chips: one per publication, kind, and facet value.
Chips waited on three repeated full-index passes. Every search rebuilt the discovery
snapshot, scanned all 29,515 documents once per facet field for the value lists, and
counted each chip with a cloned `Filters` plus lowercase allocation for every document
and facet pair. `SearchIndex` now builds per-value document lists once when it decodes the
index. `filter_counts_eligible` evaluates each query-matched document against the active
filter groups once and answers every chip from its failure mask, and Search caches facet
values and the discovery snapshot per selection revision. The lists use metadata that
search format 1 already carries, so nothing changes in `search.bin`.

`tests/e2e/measure-library.mjs` now records these times. Local run, fresh browser profile,
distribution server on localhost, canonical content `vd7596846128e349b`:

| Measure (ms) | Chromium before | Chromium after | WebKit before | WebKit after |
|---|---:|---:|---:|---:|
| Open Search: first chip visible | 225 | 153 | 236 | 174 |
| Open Search: every chip count settled | 235 | 164 | 249 | 188 |
| Select Spells: School and Spell level chips | 178 | 145 | 201 | 146 |
| Toggle one school: counts settled | 146 | 97 | 135 | 88 |
| Search engine p50 / p95 | 53.9 / 207.3 | 51.5 / 195.6 | 51.0 / 211.0 | 47.0 / 188.0 |

The "before" numbers already fit under a second on this machine, so the 60-second waits
were a cutover safety margin, not a reproduced local stall. They are gone from
`search.spec.ts`, which now uses the suite's normal 15-second expectation timeout; no
assertion was weakened. The remaining cost is DOM work for the 479 chips, not counting.
