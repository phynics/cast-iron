---
title: "Starred uses the category card columns"
date: 2026-11-16
status: current
superseded_by: []
issues: [404]
prs: []
area: ui
---

# Starred uses the category card columns, 2026-11-16

Closes [#404](https://github.com/phynics/cast-iron/issues/404).

- **Starred is a grid like the cards.** `.home-bookmarks` shares the `.cats` columns and column gap: one column below 36rem, two from 36rem, three from 64rem. Each cell keeps its hairline row (title, muted kind, 44 by 44 star), so its edges match the card below it. DOM order is the reading order (row-major). Reason: at 1280 px the `--measure` list ended at 44rem while the cards ran to 63rem, which left each star far from its title.
- **The preview shows six stars before "See all".** `STARRED_PREVIEW` goes from 5 to 6. Reason: six fills two rows of three, so the preview does not end on a short row. This is the owner's decision of 2026-11-16.
- **The breakpoint is the cards' 36rem, not 48rem.** The issue text names 48rem for the two-column step, but `.cats` switches at 36rem. Matching the cards is the acceptance (edges equal), so Starred follows them. Changing the cards' breakpoint would be a separate decision.
- **A partly filled row stays partly filled.** With one or two stars on a wide window, only those cells draw a hairline; the cell width is one card column. The list's own top rule applies only in the one-column layout.
