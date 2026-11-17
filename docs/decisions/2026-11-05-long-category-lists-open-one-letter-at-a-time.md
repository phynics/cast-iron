---
title: "Long category lists open one letter at a time"
date: 2026-11-05
status: current
superseded_by: []
issues: [100]
prs: []
area: ui
---

# Long category lists open one letter at a time, 2026-11-05

Refs [#100](https://github.com/phynics/cast-iron/issues/100).

- **A kind with more than 200 entries groups by first letter.** The threshold keeps small categories (`class`, `race`, `prestige_class`, `skill`, `trap`) as the plain list they were. Digits and symbols share the `#` group, which sorts last, matching the existing jump bar.
- **The selected letter is route state.** `#/browse/kind/spell?l=F` is shareable and adds a history entry, so Back returns to the letter that was open. A missing, malformed, or absent letter opens the first group. The route helpers are `Route::browse_kind` and `Route::browse_kind_letter`.
- **Only the selected group holds entry links.** Every group still renders its heading and count; the other groups render one link on the whole heading row instead of their rows. That link's accessible name is "B, 142 entries". This keeps the DOM small on a phone while leaving the group counts visible.
- **The jump bar sits directly below the count line.** It comes before the facet lenses, so a phone does not have to scroll past every lens to reach it.
- **Focus lands on the selected heading.** The route frame focuses `<main>` one animation frame after a route change, so the browse view waits one more frame and focuses the group's `<h2 tabindex="-1">`. A bare category URL does not steal focus. An unknown `?l=` value falls back to the first group and replaces the URL with the bare kind route.
- **The jump bar is one scrolling row on phones and wraps on desktop.** Below 48rem it keeps the 44 px targets in a horizontally scrolling row, and the open letter is centred in that row, so the sticky bar stays 48 px tall. Above 48rem it wraps to at most two rows, and the group scroll margin clears both. The letter heading is no longer sticky.
