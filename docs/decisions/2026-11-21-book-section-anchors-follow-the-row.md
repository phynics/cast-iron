---
title: "Book section anchors follow the row, not its position"
date: 2026-11-21
status: current
superseded_by: []
issues: [546]
prs: []
area: content
---

# Book section anchors follow the row, not its position, 2026-11-21

Closes [#546](https://github.com/phynics/cast-iron/issues/546). Owner decision of 2026-11-21: it gates
promoting a `main` that carries corpus v0.1.33.

**Problem.** A row of a chapter page had the DOM id `toc-<path>` (`toc-2-26`), the row's position in the
book tree. Search results, shared addresses and saved addresses carry it as `?a=toc-2-26`. A pin that adds
or removes a row moves every later row, so the same address opened another section:
`#/browse/book/blood-of-the-beast/unpaged?a=toc-2-26` was Ranger on corpus v0.1.31 and v0.1.32 and is
Rogue on v0.1.33.

**Anchors.** `cast_iron_schema::books::section_anchors` names a row by what it is. The compiler (search
results) and the client (the id on the page) both call it.

- An entry row is `row-<slug of the entry id>`, with `--<slug of the node's heading anchor>` when the node
  names one: `row-ref-class-ranger`.
- A heading-only row is `row-<parent identity>--h-<slug of its heading anchor or title>`, because a heading
  such as `Alternate Racial Traits` repeats under every race: `row-ref-race-dwarf--h-alternate-racial-traits`.
- The first row to claim an anchor in a chapter keeps it, and a later one takes `-2`, `-3`, ... in tree
  order. On corpus v0.1.33 no entry row repeats; about 130 of 40,420 rows are repeated headings that still
  share a parent and take a number.
- The walk covers every node of the chapter, listed or not. What a kind hides depends on the catalog and
  does not move an anchor. The chapter row has no anchor: its page opens at the top.
- `row-` cannot be read as `toc-`, so an address tells which kind it is.

The Contents page keeps `toc-<path>` for its own rows, which `?c=3.2` scrolls to. The client computes those
from the tree it has loaded, and nothing saves them as a section address.

**Old addresses.** `sources/legacy-sections.json` maps the positional anchor of every row that the last
published tree rendered to that row's stable anchor (6,795 rows in 244 books, 378,360 bytes). The compiler
checks the file, writes it into the library as `legacy-sections.json` (additive and optional, read only when
a `toc-` address arrives) and reports in the compile notes how many rows still find their row. When a
`toc-<path>` address opens a chapter page, the client looks the stable anchor up in the table, finds the
chapter that now holds it, and replaces the address with `?a=<stable anchor>` there. A row that has left the
book, or a path that no published tree made, opens its chapter with no anchor. A library without the table
(an older install) resolves the address against its own tree, which is the tree that made it.

- **The table is the last published tree.** It was made once, by `cargo xtask legacy-sections --books
  <books.json> --catalog <catalog.json>`, from the `books.json` and `catalog.json` of the live library at
  corpus v0.1.31 (content `v4192151b29702eab`). Corpus v0.1.32 never shipped. The positional anchors of
  v0.1.33 exist only on `main` and in no release, so where v0.1.31 and v0.1.33 disagree about a path the
  published meaning wins. The Blood of the Beast case is such a path: `toc-2-26` opens Ranger.
- **Future pins need no change.** No release after this one makes a positional anchor, so the table does not
  grow. It stores stable anchors, which a pin does not move, so the table stays correct while rows stay in
  their books. Nothing fails when a row leaves a book (unlike published entry ids, which an alias keeps
  alive): the old address opens the chapter, and the compile note counts those rows. On v0.1.33, 6,794 of
  6,795 still open their row.

**Format impact.** `SCHEMA_VERSION` stays 6 and `INDEX_FORMAT_VERSION` stays 2. `books.json` and the index
layout are unchanged; the book-section `anchor` field of `search.bin` carries the new string, and
`legacy-sections.json` is a new optional file that no older reader looks for, as with `aliases.json`. An
older app shell that reads a new library renders `toc-` ids and meets `row-` anchors in search results, so a
section result opens its chapter without scrolling to the row. The shell and the library update together
in normal use.

**Not covered.** `#/browse/book/<id>?c=<path>` is still a position, and the trail above an entry writes it
from the loaded tree each time. It is not a saved section address, and #546 names only `?a=toc-…`.
