---
title: "Feature heads and search aliases for class features"
date: 2026-11-24
status: current
superseded_by: []
issues: [540]
prs: []
area: ui
---

# Feature heads and search aliases for class features, 2026-11-24

Slice 2 of #540 renders the corpus format 4 fields that slice 1 (#677) added
(`2026-11-24-corpus-format-4-feature-facts`). The data is not converted yet, so no entry has
`features` and the pinned library renders as before.

## Decision

- **Head.** A heading that an entry lists in `features` renders as the head of a class-option
  power (#537): the printed name, a dotted leader and `Level N` on the right. A feature with no
  `level` shows the name alone, with no leader and no right side. The heading keeps its element
  level, its id and its outline place; `?a=<anchor>` reaches it. A heading that no `features` item
  names renders as before. DESIGN.md, "Feature heads", holds the rest.
- **No power layout on a converted entry.** An entry with `features` is converted and has no import
  rows, so the class-option power layout (`powers::plan`) and its one-power facts do not run on it.
  A single-power class option has no `features` and keeps the one-power page.
- **`On this page`.** A feature is a heading, so it counts like any heading; the list still appears
  from four headings. The list shows no level.
- **Trades line.** A name links to the feature of the page that states it (the first anchor of
  the trade's `by` that has a head), on a page that lists `On this page` (four or more headings),
  because only on a longer page does the jump save a scroll. A trade with no `by` stays text. The
  link text is the printed name; a visually hidden suffix names the target. Chosen against linking
  always (a link on a name that is one screen away adds a target and no value) and against no link
  (the first feature of a long archetype is far from the line that names what it replaces). Class
  options show no trades line today, and this slice adds none.
- **Search aliases.** The compiler adds the printed name of each listed feature, without its
  ability tag, to the aliases of an archetype, a class option and a base class (`pipeline.rs`
  `search_aliases`). The index already stores aliases per document, so `INDEX_FORMAT_VERSION`
  stays 2 and `SCHEMA_VERSION` stays 7. The client needs the anchor, and the index has none, so the
  engine reports which alias a hit matched (`Hit.alias`, filled when no word of the title matched:
  not part of the stored index), and the results page reads the entry (a cached shard) only for a
  hit with such an alias, then opens it at the feature whose name equals the alias. Rule entries
  keep their own heading match. A title or text hit never reads a shard.

## Size budget

The pinned library has no `features`, so the budget does not change today (aliases added: 0). The
issue's survey counts about 6,800 archetype and 8,200 class-option feature names. Each alias costs
its text in the index's document table and one posting per new word. At about 25 bytes of alias text
and about 8 bytes of postings each, 15,000 names add roughly 0.5 MB to `search.bin` before
compression, against a budget of 72.3 MB. The measurement waits for the data (the first converted
batch); `cargo xtask size-budget` is the gate.
