---
title: "Spell row density: Compact and Wide"
date: 2026-11-07
status: partly-superseded
superseded_by: [2026-11-22-spell-row-density-is-a-text-toggle]
issues: [153, 167, 182, 185]
prs: []
area: ui
---

# Spell row density: Compact and Wide, 2026-11-07

Refs [#182](https://github.com/phynics/cast-iron/issues/182). The owner decision keeps this on spell-list pages only: search results (#167) and Books (#153) are separate issues, and Home rows stay one line (#185).

- **Two densities, one hook.** Wide is the #179 default: name and school, then casting time · save · gist · book, with the gist on its own line under the facts on a phone (three lines) and one line from 40 rem. Compact is two lines at every width: name and school, then one fact line that includes the gist and the book and ends in an ellipsis when it overflows. The list container carries `data-density`, and CSS selects the layout under each value; the rendering logic does not fork.
- **The choice is a device preference.** `spell_density` in the `meta` store holds `compact` or `wide`, next to the theme. Absent or unknown means Wide, the layout every existing profile already had. The app context holds the live value, so a toggle on one spell list changes the next one and the Settings row without another read.
- **Two Filter-role pills, 44 px.** Compact and Wide are radio inputs inside `.chip` labels, so the selected one is filled and carries a check (the state never relies on colour alone). The group sits beside the school filter and is named "Spell row density". Settings repeats the same choice as an Appearance radio group so a reader can set it before opening a list.
