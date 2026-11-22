---
title: "Entry link lists fold on phones"
date: 2026-11-22
status: current
superseded_by: []
issues: [572, 594]
prs: []
area: ui
---

# Entry link lists fold on phones, 2026-11-22

Refs [#572](https://github.com/phynics/cast-iron/issues/572), part of the survey [#594](https://github.com/phynics/cast-iron/issues/594). It refines `2026-11-17-class-pages-summary-card-and-folded-references`, which folded References and Referenced by on class pages only and kept the open panels on every other kind "until a design decision extends the fold". This is that decision.

- **Below 64 rem every entry folds both lists.** Measured at 390 px, the two lists and the long `Leads to` list made up most of a feat page (Power Attack 11,363 px, the Acrobatics rule 17,459 px, Dwarf 17,822 px), and the citations, the reading footer, the licence line and Provenance sat below all of it. Without a side panel the lists have no place beside the body, so they take the class-page form: closed `details.ref-block.ref-fold` rows after the reading footer and before the entry footer, each with the chevron, the name and the count.
- **From 64 rem an entry other than a class shows the same `details` open in the side panel.** The open state follows `matchMedia("(min-width: 64rem)")`, read in Rust on load and on change. A reader who closes a row keeps it closed until the width crosses 64 rem. A class or prestige-class page stays folded and one column at every width.
- **Folded or open, Referenced by drops its `N entries link here` line.** The summary holds the count, so the panel is the same element at every width.
- **A relationship list shows 8 rows, then `Show all {N}`.** `Leads to` on a feat shows the first 8 rows. `Racial traits` on a race applies the same cap to each group (`Standard racial traits`, `Alternate racial traits, {book}`), because the panel is a set of book groups and a cap on the whole panel would hide group headings. The row is the `details.backlink-more` that Referenced by groups already use, and rows past 8 are built only when it opens (`views/backlinks.rs`, `show_all`). A list of 8 rows or fewer has no row. The grouped Archetypes panel of a class is not capped; the issue names only these two lists.
