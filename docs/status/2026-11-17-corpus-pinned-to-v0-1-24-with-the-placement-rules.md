---
title: "Corpus pinned to v0.1.24, with the placement rules"
date: 2026-11-17
kind: pin
---

# Corpus pinned to v0.1.24, with the placement rules, 2026-11-17

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.24`. The annotated tag object
is `09fd5e5b1bc12c39ae14af11d8a4da55d51caf55`; it peels to commit
`fdb3fc2f24cf06a0c38aedfd92e22a65b78788aa`. The tree hash is
`707a8e583d7f44ef9fa78d849d94599d3841f4b55dc0544bb49584962b634724`
(`cargo xtask corpus-tree-hash` on the extracted archive, which `cargo xtask fetch-sources`
verified). The tag holds data #86 (every placement names a book in the entry's `source_ids`, 9
books with no publication row deleted, 169 Core Rulebook rules placed in printed chapters) and
data #85 (corpus CI). This change adds the two `corpus-check` rules that #86 satisfies and the
retired-address redirects ([#420](https://github.com/phynics/cast-iron/issues/420); see
`docs/decisions.md`, "Placement membership and retired book routes").

- **Counts.** 31,800 entries (unchanged), 439 publications (unchanged), 439 books (was 448:
  the nine deleted books were `prpg-core-rulebook`, `pathfinder-rpg-bestiary`, three Paizo blog
  posts, three Pathfinder Comics issues and Pathfinder Worldscape #2). Placements fall from
  43,455 to 39,159: 3,126 in the deleted books, 1,137 base-entry listings, 33 Bestiary and
  Bestiary 2 placements with no evidence. Reachability: 31,800 reachable, 0 unreachable. Links:
  147,603 resolved, 0 broken. The library schema stays 4 and the search index format stays 2.
- **Rules.** `corpus-check` fails on a placement whose book is not in the entry's `source_ids`
  and on a book with no `publications.toml` row. v0.1.23 would have failed with 4,297
  placements outside `source_ids` and 9 books with no row; v0.1.24 passes.
- **Redirects.** `#/browse/book/prpg-core-rulebook` opens Core Rulebook and
  `#/browse/book/pathfinder-rpg-bestiary` opens Bestiary. A chapter address of either (`page-order`,
  `unpaged`, `?c=`) opens the replacement book's page. An address of one of the seven one-off
  trees opens Books.
- `sources/published-ids.txt` and `sources/published-books.txt` do not change: none of the nine
  deleted ids was on either list.
