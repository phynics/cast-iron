---
title: "Quick reference page and the Table tab"
date: 2026-11-17
status: current
superseded_by: []
issues: [421, 437, 439]
prs: []
area: ui
---

# Quick reference page and the Table tab, 2026-11-17

Closes [#437](https://github.com/phynics/cast-iron/issues/437). Part of [#421](https://github.com/phynics/cast-iron/issues/421), whose design comment holds the target list. The owner decided on the same day to add a fourth tab and to leave Home without a chip row.

- **Four primary destinations: Home, Books, Table, Settings.** This replaces the rule that the destinations are fixed at three. Table is a tab so the list is one tap from every page, mid-play, without opening Home or a search. The bottom bar sizes its columns by the tab count (`grid-auto-flow: column`, `grid-auto-columns: 1fr`, in the phone rules after "Reader navigation and home"), so four tabs need no new rule. An earlier `repeat(5, 1fr)` in the same bar was overridden by those rules and never applied; it is removed. Reason for a tab over a Home section: Home is for what a reader starred and the categories; the lookups a table repeats every session are a different job and want a stable place.
- **The page is one Rust constant of (label, entry id, optional anchor), in four groups.** It holds no text from the rules, so nothing can drift from the corpus. A chip whose id the library does not hold is not drawn. Labels are the page's own words, not entry titles, so `Broken` can open `Broken and Fragile` and `AC modifiers` can open the middle of a table.
- **Two chips can share an entry.** `Grappled` and `Pinned` both open `ref/rule/grapple-conditions`, because the corpus holds them as two bold paragraphs in one entry, with no heading to anchor. The native test names that pair as the only allowed repeat. Ability damage and ability drain are one chip, `Ability damage and drain` (one entry, `ref/rule/ability-damage-drain`), which makes the 35 conditions of the design.
- **Anchors are derived, not frozen.** Three chips carry a `sec-` anchor that the schema derives from the heading text (`sec-table-8-6-armor-class-modifiers`, `sec-actions-that-provoke-attacks-of-opportunity`, `sec-table-8-7-two-weapon-fighting-penalties`). A corpus edit of one of those headings would move the target; [#439](https://github.com/phynics/cast-iron/issues/439) freezes them. The Playwright spec checks that each anchor element exists, so a drift fails there.
- **Icon: a die.** A rounded square with five pips, in the existing set's stroke and grid. The tab is about the table at play, and a book or list icon would repeat Books.
- **Header alignment (#403, #409) is unchanged.** The header keeps the `--wide` column edge; the fourth tab only adds one more nav link to the right-aligned group. The spec asserts the brand's left edge and the last control's right edge are the same with the tab as without it.
