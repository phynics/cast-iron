---
title: "Browse rows wrap between words"
date: 2026-11-17
status: current
superseded_by: []
issues: [443, 444]
prs: [467]
area: ui
---

# Browse rows wrap between words, 2026-11-17

Closes [#444](https://github.com/phynics/cast-iron/issues/444). Part of [#443](https://github.com/phynics/cast-iron/issues/443).

- **Cause.** A kind-list row (`.entry-list li`) was a no-wrap flex line with `overflow-wrap: anywhere`. The title link and the muted source (`.row-source`) both had `min-width: 0` and could shrink, and `anywhere` also makes a word's minimum width one letter, so the two shared the line in proportion to their text. A long source took half of a 310 px column and left the title `Abominatio|n` or `Alien Consciousn|ess`.
- **Rule.** A row wraps (`flex-wrap: wrap`). The title keeps its natural width (`flex: 1 1 auto`), so it breaks only between words. The source sits beside it when both fit and drops to its own right-aligned line when they do not. Both use `overflow-wrap: break-word`: a single word wider than the row still breaks as a last resort, and the page never scrolls sideways. Do not use `anywhere` or `break-all` on a flex item that shares a line with another.
- **Cost.** A row with a long source is now two lines tall in a narrow column, where it was one line with a split title. The 44 px target and the one-line truncation of Home's starred rows (`.home-row-title`) are unchanged.
- **Checked elsewhere.** Search results, Quick search rows and Books rows stack the title above the muted line, so they never share a line and did not split words; the spec covers kind lists.
