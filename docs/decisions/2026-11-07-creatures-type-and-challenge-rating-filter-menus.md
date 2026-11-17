---
title: "Creatures: type and challenge rating filter menus"
date: 2026-11-07
status: current
superseded_by: []
issues: [193, 196]
prs: [2, 4]
area: ui
---

# Creatures: type and challenge rating filter menus, 2026-11-07

Refs [#193](https://github.com/phynics/cast-iron/issues/193), PR 4 of 5. Creatures move from the old pill lens to the filter menus of PRs 2 and 3; races keep their lens until [#196](https://github.com/phynics/cast-iron/issues/196).

- **Two menus narrow the one list together.** `Type: All types` (`creature_type`, 13 values) and `CR: All CRs` (`cr`, 35 values). Each is a link to `#/browse/kind/creature?facet=<field>:<value>`, and a second choice adds the other field instead of replacing it (`?facet=creature_type:dragon&facet=cr:10`). The page intersects the index ids for every applied field, so the count line shows the part of the whole (`17 of 3,753 creatures · Show all`). A repeated value for one field still keeps the first, as on every list page.
- **Challenge ratings sort numerically, not as text.** `sort_facet_pairs("cr", …)` reads each value with `ChallengeRating::parse` and orders by twenty-fourths: `1/8, 1/6, 1/4, 1/3, 1/2, 1, 2, … 30`. Alphabetical order would put `10` before `2` and the fractions in the wrong sequence. Both menus have a find field because each group has more than ten options.
- **Creature type labels are title case.** The index values stay lowercase (`dragon`, `ooze`) because routes and filters use them; `facet_label` only changes the display, so the button reads `Type: Dragon`, the scope reads `Search Dragon creatures`, and the empty state reads `No Dragon creatures in your active books.` The camelCase corpus values keep their explicit labels (`Magical Beast`, `Monstrous Humanoid`).
- **A choice focuses only the menu that changed.** The focus request across a route change now carries the menu's field, not a bare flag, so on a page with two menus the chosen button takes focus and the other does not race it. Before, both buttons asked for focus after their data loaded and the later one won.
