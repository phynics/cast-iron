---
title: "Search results page: compact controls"
date: 2026-11-15
status: current
superseded_by: []
issues: [167]
prs: []
area: search
---

# Search results page: compact controls, 2026-11-15

Refs [#167](https://github.com/phynics/cast-iron/issues/167). Layout only: no filter, count or URL behaviour changed. The Search button, the wrapping quick kinds and the full-width `Filters` bar were measured on current `main` at 390 px: the first result row started at 565 px, and the first entry row at 1,170 px, because 45 book-section rows come first.

- **Threshold (accepted by the owner on 2026-11-15).** The first result row starts at most 280 px down without a hint, and at most 330 px with the inactive-books hint (measured 257 and 304). The issue's "about 180 px" cannot hold: the header (61 px), the field, the scope row (44 px each with gaps) and one status line already take about 190 px before any row.
- **One scope row.** The quick kinds scroll sideways and the `Filters` button is fixed at the end of the row, so it stays reachable. The scrollbar is hidden; a focused pill scrolls itself into view, and the pressed kind is scrolled into view when the kinds change (a link, the panel or Back can choose a kind that sits out of sight).
- **`Filters · N`.** N is the existing count (kinds, publications and facets). A kind chosen in the row counts, because the Kind chips in the panel show it as checked too.
- **Panel.** The same panel as before, now following the #193 filter-menu pattern: in flow, `50dvh`, no dialog. It is a named `group` with `tabindex="-1"` and takes focus when it opens, so a screen reader hears its name before the chips.
- **Status area.** The count line is `--step--1`. On a phone the timing ("57 ms") is hidden, because it wrapped the line onto a second row; it stays on wider screens. The hint keeps its sentence, which names the book, and its action shortens to `Turn on` (accessible name `Turn on {Book}`), so the sentence and the action are one line of reading and not a boxed card.
- **Left as it was.** The `Searching in` scope pill row still appears for a kind scope (#164 decision), so a scoped results page has one more row than the figures above.
