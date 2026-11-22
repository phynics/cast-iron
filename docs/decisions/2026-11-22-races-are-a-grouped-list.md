---
title: "Races are a grouped list"
date: 2026-11-22
status: current
superseded_by: []
issues: [193, 196, 576, 594]
prs: []
area: ui
---

# Races are a grouped list, 2026-11-22

Refs [#576](https://github.com/phynics/cast-iron/issues/576), part of the UI/UX survey [#594](https://github.com/phynics/cast-iron/issues/594). The owner accepted the design on 2026-11-22. It closes the last line of [the filter-menus decision of 2026-11-07](2026-11-07-filter-menus-for-the-other-kinds-and-classes-under-headings.md): "Browse still links to search for races and creatures (`FacetLens`), until PR 4 and #196". Creatures have had their menus since PR 4, and `race_group` is now on every typed race (#196).

- **One list under headings, no filter.** `#/browse/kind/race` is the form Classes use (DESIGN.md, "Grouped lists"): the `h1`, the count line (`62 races`), then `h2.letter-head` headings in the order of the Advanced Race Guide's race chapters, each with its count: Core races, Featured races, Uncommon races, Other races. The four group pills wrapped to three rows at 390 px and had no `All` pill; the Filter pill is only for a group whose options fit one row at 320 px. The seven core races were scattered through an A to Z list.
- **Labels.** The headings drop the `(Advanced Race Guide)` suffix of the search facet label, because the order already names the book. The search facet `Race group` in the filter panel keeps its labels.
- **Nothing is lost.** A group that no active book holds shows no heading. A race without a group (a carried legacy race) is listed in its A to Z place under `Other races`. A book-filtered list (`?source=`) stays one A to Z list, as for Classes.
- **Old links.** `#/browse/kind/race?facet=race_group:core_race` opens the grouped list and the address bar replaces the facet away (no history entry). The same applies to `class_type` on Classes.
- **Code removed.** `FacetLens`, `Lens::Facet`, the race arm of `lenses()` and the `.lens-chip`, `.lens-more` and `.lens-wait` styles. The `.lenses` wrapper stays, because the filter row of the other kinds sits in it. The list order is the `RACE_GROUP_ORDER` constant beside `CLASS_GROUP_ORDER` in `views/categories.rs`, and `grouped_by` returns `race_group` for races.
