---
title: "Measure pages and the footer start at the wide column's left edge"
date: 2026-11-16
status: current
superseded_by: []
issues: [403, 406]
prs: []
area: ui
---

# Measure pages and the footer start at the wide column's left edge, 2026-11-16

Refs [#406](https://github.com/phynics/cast-iron/issues/406). It finishes the alignment that [#403](https://github.com/phynics/cast-iron/issues/403) began for the header.

- **From 48rem up, `main` is left-aligned in the `--wide` column.** Its inline-start margin is `max(0, (100% - --wide) / 2)`; the end margin is `auto`. A `--wide` main gets the same margin as before, so Home, Search, and entries do not move. A measure page (Settings, Legal, the installer) keeps `max-width: --measure`, so its line length stays at most 44rem, but its left edge, 11rem to the right of the brand at 1280 px, now equals the brand's.
- **The footer follows.** `.app-header` and `.app-footer` share one `--column-gutter` (the gap beside the `--wide` column plus `--space-5`, floored by the safe-area inset). The footer paragraphs keep `max-width: --measure` with no auto margin.
- **Narrow windows.** Between 48rem and 66rem the `--wide` column is the whole window, so a measure page is now left-aligned with a free area on its right, where it was centred before. This is the same left edge the header has there.
- **Phone layout unchanged.** Below 48rem `main` stays centred at its measure and the footer keeps its 16 px padding. `home.spec.ts` and `install.spec.ts` guard both.
