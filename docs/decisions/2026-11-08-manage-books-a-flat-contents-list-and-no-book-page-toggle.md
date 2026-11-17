---
title: "Manage books, a flat contents list, and no book-page toggle"
date: 2026-11-08
status: current
superseded_by: []
issues: [287]
prs: []
area: ui
---

# Manage books, a flat contents list, and no book-page toggle, 2026-11-08

Closes [#287](https://github.com/phynics/cast-iron/issues/287). Supersedes the `Active books`
route name, the book page's `Active` toggle row, and the disclosure tree of the earlier Books
entries.

- **The page is `Manage books` (`#/books/manage`).** The link on Books, the H1, the Settings
  Library row and Home's empty-set notice read `Manage books`. `#/books/active` still parses to
  the same route, and the router replaces its address with `#/books/manage`, so an old bookmark
  opens the page without a second history entry. Preset names are unchanged.
- **The book page has no toggle.** Books are switched on or off only on Manage books. While a
  book is off, `#/browse/book/<id>` shows one muted line: `This book is turned off. Its entries
  don't appear in search or lists.` An active book shows neither the line nor a control. The
  page no longer writes `disabled_books`.
- **Contents is one expanded list.** Chapters and sections are nested `<ol>` rows, not
  `<details>` or `button[aria-expanded]`. A branch row keeps its title (a link when the chapter
  is itself an entry) and its muted tabular count; nested lists carry the children. No row uses
  dotted leaders. The `toc-` ids, `?c=3.2` addresses and the `aria-current="location"` mark stay,
  so deep links and the entry trail still resolve.
- **Every kind with a list page leaves Contents.** `linked_kind` is now `kind != Rule`;
  spells, feats, creatures, items, classes, prestige classes, races, skills, afflictions, traps,
  traits, racial traits, archetypes and class options are reached through `In this book`. A
  chapter whose entries are all left out is not listed, so the Core Rulebook's contents hold
  only rules chapters and sections. The catch-all chapter follows the same filter.
- **Tests follow the new UI.** The e2e suite opens the Manage books page for every selection
  change, asserts the old address still works and rewrites to the new one, and checks that the
  Core Rulebook contents have no disclosure buttons, no feat or item rows, and a `Feats` link to
  `#/browse/kind/feat?source=core-rulebook`.
