---
title: "Filter menus for the other kinds, and classes under headings"
date: 2026-11-07
status: partly-superseded
superseded_by: ["2026-11-22-filter-fields-have-closed-vocabularies", "2026-11-22-races-are-a-grouped-list"]
issues: [193, 195, 196]
prs: [2, 3, 4]
area: ui
---

# Filter menus for the other kinds, and classes under headings, 2026-11-07

Refs [#193](https://github.com/phynics/cast-iron/issues/193) and [#195](https://github.com/phynics/cast-iron/issues/195), PR 3 of 5. The feats menu of PR 2 now serves every kind that had a lens, except creatures (PR 4) and races (no group data, [#196](https://github.com/phynics/cast-iron/issues/196)).

- **Menus by kind.** Items `Category: All categories` (57 values, with the find field), Traits `Category: All categories` (13), Archetypes `Class: All classes` (with the find field), Spells `School: All schools` (10 values, so no find field). Each reuses `FilterMenu`, filters in place (`?facet=item_category:Rods`) and names the kind in the count line (`340 of 8,151 items · Show all`). The pill walls, `N more` disclosures and the spell-list `.lens-page` card are gone; the spell lists are a `Spell lists` page link in the filter row. `Lens::Page` and its CSS are removed.
- **Classes are grouped, not filtered.** The council planned inline pills `All · Core · Base · NPC` for classes. The corpus now has seven groups, and eight pills do not fit one row at 320 px, which the council's own rule requires; a menu was ruled out for a group this small. The list is 49 classes, so it shows every group at once under an `h2` heading with its count. A filter would only hide the other groups. Prestige classes keep their own category (the class lens already skipped `prestige`). The groups come from the search index (`class_type`), so they follow the active books; a heading with no active class is not shown, and a class without a group is listed last under `Other`. An old link `?facet=class_type:core` on the classes page shows the grouped list.
- **The scope of an item category drops the kind word.** A category is already a plural noun, so the header field reads `Search Rods`, not `Search Rods items`; the results line is `4 results in Rods`. The other kinds read `Search Evocation spells`, `Search Combat traits`, `Search Wizard archetypes`.
- **Values are shown as the corpus has them.** The client does not rename facet values beyond the existing label rules. Oddities seen while building: the spell school menu lists `Multiple` (1 spell, from `multiple (see text)`), and 60 items have no category, so they appear only under All categories. These are corpus questions.
- **Browse still links to search for races and creatures** (`FacetLens`), until PR 4 and #196.
