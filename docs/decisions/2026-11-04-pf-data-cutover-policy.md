---
title: "Pf Data cutover policy"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Pf Data cutover policy, 2026-11-04

- **Identity preservation is waived for the cutover.** The product owner accepted that some PSRD-era bookmarks may stop resolving. `ref/` ids become the canonical public ids at cutover. The alias map from Phase D remains the path for every legacy id that has a reviewed counterpart, and an unmatched legacy id resolves to its `[retired]` reason, never to a guessed article. After cutover, `ref/` ids are permanent under AGENTS.md rule 3; the waiver covers this one renumbering only.
- **The content target is maximum OGC coverage.** Derive everything the pinned Pf Data corpus holds. Carry over Open Game Content that Pf Data lacks from the PSRD snapshot, so that no OGC is lost. Retire the PSRD pipeline only when nothing remains to carry over; otherwise it stays as a carry-over input.
- **The full corpus ships.** The measured full-input run is about 209 MB of canonical content plus 53 MB of search index (today 62 MB + 20 MB). The size is accepted. Offline acceptance still has to pass at that size. Install time, storage quota, and search latency are measured and recorded, not used as a reason to cut scope.
