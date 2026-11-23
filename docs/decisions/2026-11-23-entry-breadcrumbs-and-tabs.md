---
title: "Entry breadcrumbs and the masthead tab"
date: 2026-11-23
status: current
superseded_by: []
issues: [571]
prs: []
area: ui
---

# Entry breadcrumbs and the masthead tab, 2026-11-23

Settles #571. The owner accepted the rule and decided its open points on 2026-11-23. Two changes
carry it: the tabs first, the breadcrumbs second. This entry covers both so that the second change
refers to it.

## The tab

- **The tab is the section that holds the route.** Exactly one tab carries `aria-current="page"`,
  or none.
- **Books:** Books, Manage books and a share link, the removed-book Books, a book page and a chapter
  page; an entry with `?book=X`; a kind list with `?source=X`. The last two only when the library
  holds book X. An old publication address (`#/browse/source/<id>`) is the book page when the
  library has `books.json`, so it is Books too.
- **Home:** Home, a kind list (also one filtered by a book the library does not hold), the spell
  hub and spell lists, Feat chains, a publication address in a library without `books.json`, and
  every other entry: one opened without a book, an unavailable one and a retired id.
- **Table:** Quick reference only. **Settings:** Settings, Diagnostics and Legal.
- **None:** Search and Not found.
- A book that the library does not hold gives no Books tab. The page is the one the reader would get
  anyway (the whole list, with its notice), and a tab that names a book the app lacks would point at
  nothing.

## The breadcrumb

A breadcrumb lists the linked pages between the tab's destination and the current page. It never
names the destination and never names the current page. With no page between, no `nav.crumbs` is
rendered.

- **An entry with `?book=X`** that the book places: `{Book} › {Chapter}`, the chapter page, also
  for an add-on, so the trail stays steady while the reader steps with Next.
- **An add-on** opened any other way: none.
- **A rule entry:** its parent chain if it has one (about 610 do; the Barbarian class rule shows
  `Barbarian`), otherwise none.
- **Any other entry:** its kind list, then its parent chain. Printed sections and creature taxonomy
  never become crumbs. A class option's crumb is `Class options`.
- **A chapter page:** `{Book}`. **A kind list filtered by a book:** `{Book}`.
- **Feat chains:** `Feats`. **Spell hub:** `Spells`. **Spell list:** `Spells › Spell lists`.
- The footer trail `ol.book-trail` goes. Previous and Next stay.
- A long book title may wrap the crumb at 390 px. It is never truncated.

## Which earlier entries this replaces

- `2026-11-09-chapter-pages-and-their-address` (the chapter crumb `Books › Book › Chapter`) and
  `2026-11-06-list-page-template-and-scoped-quick-search` (entry crumbs keep `Home` and `Books`)
  speak about crumbs only. Neither states a tab rule, so the tab change does not touch them. The
  crumb change replaces part of both, and it sets their `status` and `superseded_by` when it lands.
- `2026-11-07-books-tab-and-book-page` keeps its rule that a publication address belongs to Home in a
  library without book pages.

## Delivery

1. The tabs (`in_section` in `crates/web/src/app.rs`, `DESIGN.md` Primary navigation, the tab
   assertions of `browse-groups.spec.ts`).
2. The breadcrumbs (`books.spec.ts`; `mobile.spec.ts` where it asserts the footer trail).
