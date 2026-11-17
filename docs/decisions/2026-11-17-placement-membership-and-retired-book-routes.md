---
title: "Placement membership and retired book routes"
date: 2026-11-17
status: current
superseded_by: []
issues: [420]
prs: []
area: corpus
---

# Placement membership and retired book routes, 2026-11-17

Refs [#420](https://github.com/phynics/cast-iron/issues/420). It pins corpus `v0.1.24`, which holds [cast-iron-data #86](https://github.com/phynics/cast-iron-data/pull/86).

- **A placement must name a book the entry prints.** `corpus-check` rejects a `books.toml` node whose book is not in the entry's `source_ids`. The rule covers every node with an entry: a chapter, a nested section, an anchored node, and a node in a catch-all, `Page order` or `Unpaged` chapter. The message names the `books.toml` line, the book, the node title and its top-level chapter, the entry, the entry's `source_ids` and its file. Two fixes are valid: remove the placement, or add the book to `source_ids` when it prints the entry. Reason: the first placement in book order decides an entry's book in the app (counts, the active-book filter, the breadcrumb), so a placement without evidence put 116 entries in the wrong book, and `source_ids` is the record of what a book prints. Corpus v0.1.23 had 4,297 such placements; v0.1.24 has none.
- **Every book needs a `publications.toml` row.** `corpus-check` rejects a `[[books]]` block whose `source` has no row. A printed book needs a copyright and a Section 15 notice or a `notice_pending` decision (AGENTS.md rule 12); a blog post or comic issue is not a printed book and gets no book. v0.1.23 had nine such books (448 books against 439 publications).
- **The guard and the pin land together.** v0.1.23 fails both rules, v0.1.24 passes them, so one PR carries both. The step's reachability check stays strict: the 169 Core Rulebook rules that only the deleted PRPG listing made reachable now sit in their printed chapters.
- **An address of a deleted tree still opens something.** Bookmarks, shared links and the reader's saved place hold book ids, and the nine deleted ids were reachable through Contents and the breadcrumb (`book_source.rs`):

  | Old address | Opens |
  |---|---|
  | `#/browse/book/prpg-core-rulebook`, `#/browse/source/prpg-core-rulebook`, and `?book=` on an entry address with that id | Core Rulebook |
  | `#/browse/book/pathfinder-rpg-bestiary` and the same forms | Bestiary |
  | A chapter address of either tree (`…/page-order`, `…/unpaged`, `#/books/<id>/<chapter>`, `?c=<path>`) | The replacement book's page |
  | A book, chapter or source address of one of the seven one-off trees (the three Paizo blog posts, Pathfinder Comics #3, #5 and #11, Pathfinder Worldscape #2) | Books |

  A chapter of a deleted tree has no equivalent in the printed book: the old chapters were `Page order` and `Unpaged` listings, and the replacement's chapters are printed ones. A position address (`?c=1`) would mean a different chapter in the printed book, so it also opens the book page and not a guessed chapter. A one-off tree has no replacement and no entry to name in the address, so Books is the destination. The app has no pattern for a notice on a redirected address, so there is none. The saved reading place and stored `?book=` values of the two listing trees follow the same alias as the Game Mastery Guide merge. The disabled-book set needs no migration: neither tree had a row, so a reader could not turn it off.
