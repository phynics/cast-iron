---
title: "Class options in the app"
date: 2026-11-08
status: partly-superseded
superseded_by: ["2026-11-22-class-option-group-in-the-search-index"]
issues: [146, 246]
prs: []
area: ui
---

# Class options in the app, 2026-11-08

Refs [#146](https://github.com/phynics/cast-iron/issues/146),
[#246](https://github.com/phynics/cast-iron/issues/246). The app half of slice 4; corpus
v0.1.8 holds the 2,802 entries in 49 groups.

- **Groups come from `option-groups.json`, not the search index.** The search index has no
  `group` field, so the list page's group menu and its filtering read the derived index and the
  active-book snapshot. `?facet=group:<slug>` stays the route. Search and Quick search read the
  same file to print `{Group singular} · {book}`; the result scope keeps the kind and drops the
  group, because a group facet would match nothing in the index. No index or serialized format
  changed.
- **A shared group carries `?facet=class_scope:<class id>`.** The class page adds it only when
  the group's `classes` list has more than one class. The list page shows it as the only
  `.scope-pill` there, and removing the pill keeps the group. The value does not narrow the
  entries: a group's options are shared by every class it serves.
- **Class options are a Home category after Prestige classes.** They are character-building
  material, so they sit with Classes rather than in More categories. Books lists them in
  `In this book` at the same position, and Contents leaves them out like the other linked kinds
  (#158).
- **The star kind snapshot follows the library.** `Library::load` rewrites a bookmark's stored
  `kind` when the catalog disagrees, so a star made while class options were `rule` moves to
  `class_option`. The snapshot remains the fallback for an entry the library no longer holds.
- **Existing labels stay.** The list page rows read `{book}` under a group filter and
  `{Group singular} · {book}` in the whole list; an option's own page names its group and links
  its class only when the group has exactly one. Archetype rows read `Replaces {items}` from
  `replaces.json`; the file is empty for archetypes until slice 6.

Refs #146, #246.

  once at startup (`recent::load_into`) and updated by `recent::record` before it writes the
  store. A Home that read the store before that write landed renders the row when the signal
  updates, instead of showing an empty "Recently viewed" until a reload. The stored list stays
  the durable source. `merge_stored` orders the in-memory startup merge, and `load_into` marks a
  running write stale so that writer persists the merged list; together they keep a visit and
  the stored history when the two overlap.
