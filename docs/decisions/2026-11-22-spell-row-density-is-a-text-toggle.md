---
title: "Spell row density is one text toggle"
date: 2026-11-22
status: current
superseded_by: []
issues: [182, 447, 594, 595]
prs: []
area: ui
---

# Spell row density is one text toggle, 2026-11-22

Refs [#595](https://github.com/phynics/cast-iron/issues/595) and the survey [#594](https://github.com/phynics/cast-iron/issues/594). Refines `2026-11-07-spell-row-density-compact-and-wide`: the choice stays in place on the list and in Settings, and its in-place form changes from two Filter pills to one text button. The owner decided this on 2026-11-22.

- **Why.** The two pills narrow nothing, so they borrowed the Filter role that DESIGN.md "Chip roles" keeps for choices that narrow a list. They also took a row of about 60 px on the most crowded list head in the app: at 390 x 844 only one spell row fitted on the first screen.
- **One control.** A `.text-btn` (accent text, weight 600, no border, no fill, 44 px hit area), the form of `Done` and `Undo`. No pill, no check mark, no chevron.
- **The name states the action.** `Compact rows` while the list is Wide, `Wide rows` while it is Compact. It is a plain button with a changing name, not `aria-pressed`. A visually hidden live line (`role="status"`) says `Compact rows` or `Wide rows` after a press, naming the layout that is now shown, because the button's new name describes the opposite.
- **Place.** The last item of the level bar's row, after the highest level link, at the right end (`margin-left: auto`). When the last row is full it wraps to a line of its own, right-aligned. The separate density row is gone. Wide screens use the same control in the same place.
- **Behaviour.** A press switches the rows in place and saves the choice at once. It does not navigate, the address does not change, the scroll position stays and focus stays on the button. The button renders with the list head, before the spells load, on every class spell list and level (an empty narrowed list too), and not on the spell hub.
- **Unchanged.** `spell_density` in the `meta` store, the `data-density` hook on `.spell-list`, the Wide default, and Settings' `Spell row density` radio group.
- **Filter role.** DESIGN.md adds "a display choice such as row density" to the Never column of the Filter row.
