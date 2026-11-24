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
  stays 2 and `SCHEMA_VERSION` stays 7. "Which heading is a feature" is one function
  (`headings::feature_of`) that the page and the aliases share.
- **Deep link.** The index holds no anchors, and the results page does not read entry shards. The
  engine reports which alias a hit matched (`Hit.alias`: the alias whose words are exactly the
  query's, else the first that holds every word; filled only when no word of the title matched;
  not part of the stored index). The result's link carries that name in the existing anchor
  parameter (`#/entry/<id>?a=Shattering%20Strike`), and the entry page resolves it from the entry
  it loaded (`resolve_anchor`): a name that a listed feature prints becomes the feature's anchor,
  anything else is left as it is, so an alias that the entry no longer lists opens it at the top.
  Rule entries keep their own heading match on the results page.
- **Ranking (owner ruling, 2026-11-24).** An entry that a feature name matches ranks below every
  entry whose title holds all the query words: a non-rule alias that equals the query adds 100
  (`BOOST_FEATURE_ALIAS`), where an all-words title adds 200. Rule aliases keep their boost.
  Query `Evasion` lists the entry titled Evasion, then Improved Evasion, then the archetypes
  that have the feature.
- **Scaffold headings are never features (owner ruling, 2026-11-24).** `corpus-check` fails a
  `features` anchor whose heading reads as one of `NOT_FEATURE_HEADINGS` (Size, Speed, Attack, AC,
  Ability Scores, Companion Type, Prerequisites, Effect, Use, Cost, Action, Bonus Spells, Bonus
  Feats, Weapon and Armor Proficiency), so such a heading gets no head and no alias.

## Size budget

The pinned library has no `features`, so the budget does not change today (aliases added: 0). The
issue's survey counts about 6,800 archetype and 8,200 class-option feature names. The words of an
alias already have text postings, because the headings are in the indexed text, so the cost is the
alias strings in the document table (`DocMeta`): about 0.2 MB for about 13,600 names. The
measurement waits for the pilot batch; `cargo xtask size-budget` is the gate.
