---
title: "Chapter pages and their address"
date: 2026-11-09
status: current
superseded_by: []
issues: [161]
prs: []
area: ui
---

# Chapter pages and their address, 2026-11-09

Refs [#161](https://github.com/phynics/cast-iron/issues/161). Owner decisions recorded in the
issue on 2026-11-09.

- **A chapter's id is its anchor, else a slug of its printed title.** The base comes from the
  corpus node (`BookNode::anchor` or `chapter_base`), never from the row's index. A title with
  no ASCII letter or digit gets `chapter`. If two displayed chapters claim the same base, the
  first in Contents order keeps it and the later rows take `-2`, `-3`, and so on
  (`graph::chapter_ids`). Moving a chapter in the tree does not rename it unless it changes a
  collision.
- **The page is `#/browse/book/<source>/<chapter>`.** The id is one percent-encoded path
  segment; `#/books/<source>/<chapter>`, the shape in the issue, parses to the same route and
  the address bar does not rewrite it. The old `#/browse/book/<source>?c=3.2` address still
  parses to the book page and still marks the row it names. The book page remains
  `#/browse/book/<source>`.
- **The chapter is a full-width reading page.** The breadcrumb is Books › Book › Chapter. The
  chapter's children are `h2` headings; deeper nodes are plain indented lists. Nothing on the
  page is a disclosure. A chapter that is itself an entry offers one link to that entry.
- **Chapter pages list what Contents lists.** The page and its ids come from `chapter_rows`,
  which starts from `contents_roots`, so the kinds that Contents leaves out (feats, spells,
  items, class options, and every other kind with a list page) are absent here too.
