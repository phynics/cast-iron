---
title: "Wide entry grid: one flexible slack row"
date: 2026-11-17
status: current
superseded_by: []
issues: [446, 443]
prs: []
area: ui
---

# Wide entry grid: one flexible slack row, 2026-11-17

The audit of 8081e0f saw about 180 px of blank space between the header and the first sentence of Longsword at desktop width. It reproduced on main at 01b1216 (1280×900, light and dark): the body started 209 px under the header on Longsword, 173 px on Dagger and 218 px on Chain shirt. At 390×844 the gap was 12 px, so only the wide grid was affected. It was not a capture artifact and not the item statistics.

**Cause.** The References panel spans the rows from `contents` to `book`. A spanning grid item distributes the height it still needs equally among the auto rows it crosses, empty rows included. The panel is taller than a short entry's text, so the empty `contents` row grew to 197 px above the body (and the empty `aon` and `book` rows grew too).

**Decision.** The grid has one more row, `slack`, after `book`, and the panel spans through it. `slack` is the only `1fr` row, so it takes the extra height and the empty auto rows stay at 0. Header-to-body gap is now 12 px on the three item entries, which is the same as at 390 px. The folded class layout has no panel and keeps `grid-template-rows: none`.

A later grid row that the panel crosses must be `auto` and may be empty only when `slack` stays the sole flexible row.
