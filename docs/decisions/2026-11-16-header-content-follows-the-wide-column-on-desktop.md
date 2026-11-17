---
title: "Header content follows the wide column on desktop"
date: 2026-11-16
status: current
superseded_by: []
issues: [403, 406]
prs: []
area: ui
---

# Header content follows the wide column on desktop, 2026-11-16

Refs [#403](https://github.com/phynics/cast-iron/issues/403).

- **From 48rem up, the header's content shares the `--wide` column.** The header's inline padding is `max(max(0, (100% - --wide) / 2) + --space-5, safe-area inset)`, the same edge `main` uses. The brand then sits at the left edge of the Home heading, and the last header control sits at the right edge of the category grid. The header background and border stay full-bleed. Below 48rem the header is unchanged. Reason: the brand at 16 px and the content at about 128 px (at 1280 px) had no shared edge.
- **The install screen's header follows the same rule.** `installer.rs` uses `.app-header`, so at 1280 px its brand sits at 136 px while the install card, a `--measure` column, starts at 312 px. It shows only the brand: no nav, search or action. It looks like Settings does, with a wider header over a narrower column. [#406](https://github.com/phynics/cast-iron/issues/406) covers it with the other measure pages.
- **The header aligns to `--wide` on every route.** Settings and Legal use the `--measure` column, so there the header is wider than the content. [#406](https://github.com/phynics/cast-iron/issues/406) decides how measure pages align. Reason: one header position on all routes, and no route-dependent header.
