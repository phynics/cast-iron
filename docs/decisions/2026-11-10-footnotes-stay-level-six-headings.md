---
title: "Footnotes stay level-six headings"
date: 2026-11-10
status: current
superseded_by: []
issues: [137]
prs: []
area: corpus
---

# Footnotes stay level-six headings, 2026-11-10

Refs [#137](https://github.com/phynics/cast-iron/issues/137). This is a documentation change. The
representation, `SCHEMA_VERSION` 4, and corpus format 1 do not change.

- **A note keeps its level-six heading.** The body has no footnote block, and only a heading
  carries an anchor that a body link can name. Level six is the deepest level, so the note sits
  below the section text.
- **The issue's other option is not taken.** A footnote block would change the serialized body and
  increase `SCHEMA_VERSION`, while every existing note already has an anchor. The docs record the
  convention instead.
- **The anchor is the contract.** A reference such as
  `^[5](ref/ability/improved-familiars#footnote-h)^` names `{#footnote-h}`, and `corpus-check`
  fails when a link anchor is not a heading key in the target entry. A prose edit leaves the anchor
  alone.
- **The renderer has no footnote special case.** A level-six heading renders as `<h6>` and joins
  the "On this page" list when the page shows one. Editors identify a note by its marker, its
  `footnote-` anchor, and its position, not by a block type.
