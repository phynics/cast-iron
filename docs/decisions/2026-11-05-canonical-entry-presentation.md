---
title: "Canonical entry presentation"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Canonical entry presentation, 2026-11-05

- **The earliest printed attribution is the primary publication.** A canonical entry's `source_ids[0]` now follows the first `‹SOURCE Book/page›` marker (or `source="Book/page"` directive) in the screened record, which `Converted.pages` already preserves in first-mention order. The upstream `sources` array is not reliable for this: it agrees with the earliest attribution on 94% of multi-book records, and the disagreements usually put a later mythic or reprint printing first. Entries with no attribution keep the upstream array order. `pf_data::publication_names` and `canonical::derive_publications` dedupe by first occurrence instead of sorting, so that fallback is stable. The full publication set is unchanged; alias fit, book chips, and per-book membership still use every publication.
- **A leading heading that repeats the entry title is dropped.** Pf Data often opens a record with `## Title` or `::fh[Title]`, while the entry page already renders the title as its h1. The converter removes only the first content heading when its text equals the title; a generated spell stat block before it does not count as content. Any later heading, including one that repeats the title inside a section, stays. PSRD carry-over entries are not rewritten.
- **Each Section 15 notice appears once, verbatim.** `SourceInfo.copyright` joins the distinct matched notices with `; ` and does not add a title prefix to each one. The title is already `SourceInfo.title` and the book page heading; the old `title. notice` form repeated it whenever the notice text itself began with the book title. Several printings of one book remain distinct notices and each is kept.
