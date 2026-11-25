---
title: "Modifier titles sort after their base in every list"
date: 2026-11-24
status: current
superseded_by: []
issues: [678]
prs: []
area: ui
---

# Modifier titles sort after their base in every list, 2026-11-24

A reader who looks for `Weapon Focus` also wants `Greater Weapon Focus`. Print keeps such options
together; a plain title sort files the second under G. The corpus keeps normal-order titles
(`Greater Weapon Focus`) for every feat and every Pf Data spell, and ids and titles do not change.
The owner ruled on 2026-11-24 (survey in the corpus repository, issue 292) that the app orders the
lists instead. The code is `crates/web/src/views/list_order.rs`.

## Rules

1. **Every list.** Each kind's page and each book's kind group sorts this way. The short lists on an
   entry page (Related, backlinks, chains, archetypes of a class) and search keep their order. Search
   tie-breaks are unchanged.
2. **A modifier title follows its base.** A title that begins with a modifier word and whose rest is
   the title of another row of the same list sits directly after that row, under that row's letter.
   It keeps its printed title. The letter bar, the section heading and its count use the same letter
   as the row, so a row never shows under W and counts under G.
3. **No base row.** When no row has the rest as its title, two or more modifier titles that share
   the rest gather after the weakest of them (`Greater Bull Rush` after `Improved Bull Rush`, under
   I). A modifier title that is alone in its list (`Improved Initiative`) does not move.
4. **Order inside a family.** The base, then its mythic version, then the members by strength, then
   by title:

   | Step | Modifiers |
   |---|---|
   | 1 | Lesser, Minor |
   | 2 | Improved, Advanced |
   | 3 | Greater, Major |
   | 4 | Mass, Communal |
   | 5 | Superior |
   | 6 | Grand |
   | 7 | Masterful |
   | 8 | Supreme, Elite |
   | 9 | True, Legendary |

   The owner named steps 1 to 3 and 5 to 7 and 9. `Advanced` is placed with `Improved` because print
   lists `Advanced / Greater / Superior`. `Mass` and `Communal` name a spell's group form and sit above
   `Greater`; `Supreme` and `Elite` sit between `Masterful` and `True`. These three placements are
   this entry's choice, not the owner's.
5. **Mythic versions.** `X, Mythic` sits directly after `X`, whatever `X` is, including a modifier
   title (`Improved Bull Rush, Mythic` after `Improved Bull Rush`). It does so even when `X` is not in
   the list, as the feat list did before.

## What counts as a modifier

The words of the table, as a whole leading word, in any case. `Extra`, `Master` and `Expert` are not
modifiers: they name feats of their own (`Extra Rage` is not a version of `Rage`). Only a leading
modifier moves a row. A title with a trailing modifier (`Restoration, Greater`, the carried PSRD
spells) already sorts beside its base. `, Mythic` is the one trailing form the app reads.

## How the order is computed

The order depends on the set of titles, so it is computed for each list, after any filter: a modifier
title sits after its base only while the base is in the list shown. A filtered list (`Combat feats`)
can therefore show `Greater Weapon Focus` under G when `Weapon Focus` is not in it. The function reads
every title once, looks the rest up in hash sets, and sorts once, so a list of any size costs one sort.
Hash maps are only looked up; the sort key ends in the row's id and its position, so the output does
not depend on input order or on hash order.
