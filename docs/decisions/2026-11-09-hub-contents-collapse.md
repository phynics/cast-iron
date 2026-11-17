---
title: "Hub contents collapse"
date: 2026-11-09
status: current
superseded_by: []
issues: [358]
prs: []
area: ui
---

# Hub contents collapse, 2026-11-09

Closes [#358](https://github.com/phynics/cast-iron/issues/358).

- **A corpus marker collapses a long-list node at any level.** `BookNode::collapse` is an
  optional, authored `collapse = true` in `books.toml`. It is additive and omitted when false,
  so the schema version stays and an older reader ignores it. A marked node shows one row with
  its count: the row links to the node's entry, or to its chapter page when the node has no
  entry. This is how a top-level appendix such as Universal Monster Rules collapses without
  hard-coding its title.
- **Without the marker, only a node below chapter level collapses.** A hub is a node that is
  itself an entry and lists every entry below it. The signal is the renderer's own child list:
  `Block::EntryLink` rows in the entry body, not every resolved inline reference. A page that
  mentions a child in a sentence does not get its nested rows removed. The pure rule is
  `cast_iron_schema::books::entry_lists_children`; a node with no entry, no entry below it, or
  one unlinked child stays a branch. An unmarked top-level chapter row keeps its chapter page
  and that page's printed sections (#346).
- **The compiler records the automatic answer on the node.** `BookNode::lists_children` is
  additive and optional in `books.json`; the corpus never authors it, old readers ignore it, and
  an old library reads as `false`. `mark_listed_children` sets it only for kinds Contents shows,
  because a hidden entry has no row link to open. No new file and no new fetch: the book page
  already loads `books.json`.
- **`corpus-check` warns before a marker hides an entry.** A marked node with an entry and at
  least one child entry must link every child entry from its body. Otherwise the warning names
  the node: collapsing it would hide entries its own page does not show. A marked heading with no
  entry has no body to check.
- **Contents shows a collapsed row and skips its list.** A collapsed row keeps its title and
  muted count, and adds no `<ol>`. A marked node and a nested hub link to their entry; a marked
  heading with no entry links to its chapter page. An unmarked top-level chapter row keeps its
  normal chapter-page link and its nested rows, so the printed sections stay reachable.
- **The chapter page collapses a hub section.** A first-level or deeper section that is a hub
  keeps its link and drops its nested list. The chapter page itself still lists its own
  first-level sections; the chapter is the page's subject, not a row inside it. `sections`
  (which the compiler turns into book-section search documents) stops at a collapsed hub too, so
  every section anchor still names a rendered row.
- **Reading order is untouched.** `reading_order` and `reading_neighbors` still walk every
  Contents-visible child in book order, so previous and next cross a collapsed hub as before.
  Only what the two lists display changed.
- **Scope.** With corpus `v0.1.17` and no markers, the automatic rule collapses 92 nodes in 11
  books. The largest are Goods and Services (136), Weapons (82) and Special Abilities (53) in the
  Core Rulebook; NPC Codex has 26 class pages. The Bestiary's Universal Monster Rules is a
  top-level chapter node, so it needs the marker to collapse; Conditions has no child node in
  `books.toml`, so it is not a hub.
