---
title: "Canonical PSRD book order"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Canonical PSRD book order, 2026-11-05

- **The sixteen PSRD books keep their curated chapters in the canonical library.** `books::build_canonical_with_psrd` walks the PSRD `BookIndex` that `pipeline::build_canonical` already builds for carry-over and rewrites each placed legacy id through the same final alias resolution that ships in `aliases.json`. Titles, chapter order, nesting, and curated review notes stay those of `book-order.toml`; the canonical build only substitutes ids.
- **Shared targets, retirements, and new content have one rule each.** A canonical target claimed by several legacy ids appears once per book, at the first placement, and later duplicates are counted and named in notes. A legacy id with no installed target is dropped from the tree and reported with its alias retirement reason. Canonical entries of the publication that the PSRD tree does not place go into one final `New entries (page order)` chapter, ordered by the same page key as the page-only build; they are never interleaved into curated chapters by guess.
- **Other publications are unchanged.** Every publication outside the sixteen PSRD books keeps `build_canonical`'s page-ordered `Page order`/`Unpaged` index. The PSRD default build is untouched, and no schema or search-format version changes because the shape of `BookNode` and `BookIndex` does not change.
- **Coverage is recorded per book.** `CanonicalBookStats.curated` holds chapters, placed, appended, dropped, duplicate, dropped-id, and duplicate-id counts; the canonical build adds one note per book to `validation.json` and `coverage.md`. The per-publication coverage table reports every canonical member of a PSRD book as placed, because the curated tree or the appended chapter holds it.
