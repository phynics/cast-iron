---
title: "Categories on Home"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Categories on Home, 2026-11-05

- **Home is the category directory.** The full directory (primary categories, more categories, and shortcuts) now renders on Home under starred entries and recent reading. The separate Browse destination is gone from the navigation; `#/browse` and `#/browse/` open Home, while kind, source, book, and books routes keep their own pages.
- **Starred entries show five rows first.** When more than five are saved, a button with `aria-expanded` reveals the rest and collapses them again; the stored order and the open and unstar actions do not change.
- **Links that named Browse now name Home.** Breadcrumbs, the entry crumb, and the empty-bookmarks action point at `Route::Home`. `BrowseKind`, `BrowseSource`, `Spells`, `SpellList`, and `FeatChains` mark Home as the current section.
- **The footer links to the project and its issue tracker.** The existing disclaimer is unchanged and always visible; the new links are marked as needing a connection, and coarse-pointer targets keep the standard touch height.
