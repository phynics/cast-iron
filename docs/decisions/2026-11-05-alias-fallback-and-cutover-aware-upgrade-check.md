---
title: "Alias fallback and cutover-aware upgrade check"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Alias fallback and cutover-aware upgrade check, 2026-11-05

- **An unemitted canonical target falls back to carry-over.** The `aliases.json` row still resolves to the planned canonical id when the converter emitted it. When the legal screen did not exclude that target but conversion failed, the build now carries the legacy PSRD entry under `ref/carried/<legacy-id>` and rewrites the alias row to it with basis `carried`. The carried entry keeps its body, fields, provenance, and license refs, so no OGC is lost. On the pinned corpus this moves 29 rows out of `unrepresentable` and leaves zero retirements. A target excluded for rights stays a `rights` retirement; a legacy id the PSRD library does not publish stays retired with its own reason.
- **Fallback carry-over is reported, not silent.** The carry-over pass resolves only map rows whose target exists in the pinned catalog, is absent from the emitted id set, is not legally excluded, and names a legacy entry the PSRD build published. The count appears under `alias-fallback-carry-over` in `coverage.json`, the build note, and `carry_over` counts; the client never sees an alias target outside the installed catalog.
- **The upgrade check resolves like the client.** When the new library ships `aliases.json`, `scripts/check-library-upgrade.py` resolves each old id in one step through the new catalog, one alias row, or the retirement list. It fails on a missing or unresolvable id, a missing alias target, a chain, an old id in both lists, an unexpected title change, a wrong class category, or a dangling relation/list/book target.
- **Title equivalence reuses the fit gate, and merges are reports.** The check compares titles with the alias fit gate's `title_fit`, so documented comma inversions and parenthetical labels pass. A title change on a `shared` or `derived` row is reported under `reported_title_changes` rather than failed, because those rows are documented merges; `direct`, `carried`, and `renamed` rows still fail on a change. Books may place one entry in every publication, so a repeated placement is reported as a count, not a failure. The script also checks that every catalog entry is placed or in `book_order.unplaced`, which is the canonical index's accounting rule.
