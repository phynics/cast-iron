---
title: "A footnote stays when AoN, the PRD or the printed book carries it"
date: 2026-11-20
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# A footnote stays when AoN, the PRD or the printed book carries it, 2026-11-20

The owner decided this on 2026-11-20 for [phynics/cast-iron-data#117](https://github.com/phynics/cast-iron-data/issues/117), step 2 of the editorial footnotes audit ([phynics/cast-iron-data#109](https://github.com/phynics/cast-iron-data/issues/109)). It refines the rule of 2026-11-19 in `docs/research/editorial-footnotes-audit.md`, which kept a note only when Archives of Nethys (AoN) or the Paizo PRD carries it.

## Decision

**The keep test.** A note stays when AoN or the PRD carries it, or when the printed book carries it. The aim is to remove what a third party added and to keep what Paizo printed.

- **Keep** a footnote block that AoN or the PRD carries (`on_aon_prd = yes`).
- **Keep** a block that neither carries when the audit classified it `printed` (`print-match`, `paraphrase` or `legend`): the book prints it, or a close paraphrase of it. Near misses where AoN rewords a note stay, and so do notes on entries that AoN has no page for.
- **Keep** the four deity notes that differ from AoN only by a leading "This", "This spell" or "The" (Shelyn block 2, Besmara block 1, Ydersius block 3, Geryon block 1). The audit already records them as `yes` through the leading-word allowance.
- **Remove** a block that neither carries and that the audit classified `editorial` (pointers, commentary, compiled lists) or `unclear`.
- **Outside footnote blocks:** remove the editorial and unclear paragraphs (`action = remove`). Book prose that only holds a signal word is not a note and stays.

## Why

- The test follows the aim of the audit. The notes that Paizo printed are source material even when AoN words them differently or has no page for the entry. A reader cannot get them anywhere else in the app.
- Notes that a dataset editor added (page pointers, "original text" remarks, GM advice) have no source. The corpus removes them, and the removal also ends the licensing question for them.
- It needs no schema change, and the audit scripts can apply it as a rule.

## Consequences

- `scripts/editorial-footnotes-nested.py` applies the test. A `no` block classified `printed` keeps `on_aon_prd = no`, because neither AoN nor the PRD carries it, gets `presence_basis = print` and `action = keep`. The audit stops listing the 80 printed blocks as removals.
- `scripts/editorial-footnotes-nested.py` applies the test. A `no` block classified `printed` keeps `on_aon_prd = no`, because neither AoN nor the PRD carries it, gets `presence_basis = print` and `action = keep`. An outside paragraph that is an inline note and that the book prints gets the same basis and action.
- A first pass of the corpus change (`phynics/cast-iron-data#123`) removed 73 blocks and 18 paragraphs by the audit's classification. A review of that PR found 12 notes that the entry's printed source carries (or that AoN carries as the same rule): the audit had marked them `editorial` or `unclear` for want of a print check. They are reclassified `printed` and stay: *Skull & Shackles Player's Guide* pp. 10-11 (three ship-table notes: Oars, Ship Modifications twice), *Legacy of Dragons* p. 27 (the Pyrausta familiar), *Ultimate Intrigue* p. 40 and *Aquatic Adventures* p. 55 (the Underhanded and Underwater ranger styles, three paragraphs), *Inner Sea Gods* p. 75 (Gozreh), the Iapholi and Ydersius Beast Shape restrictions, and the two feat-swap conditions of `ref/rule/bestiary-errata`, which the entry's credited publication prints. Rule for later reviews: a note that no check found is `unclear`, never removed until someone has looked in the book.
- On corpus pin v0.1.30 the footnote TSV has 153 `no` blocks: 89 stay (`presence_basis = print`: 71 print-match, 14 paraphrase, 4 legend) and 64 go (40 pointers, 15 commentary notes, 5 compiled lists, 4 unclear). Of the 16 outside rows with `action = remove`, 15 are removed; `ref/magic-wondrous/maniac-hand` stays, because the paragraph is a stat line of the item (data PR #115 left it for the same reason).
- The corpus change is `phynics/cast-iron-data#117`. A later reader who finds a printed note that AoN lacks should not remove it.
- This entry does not decide the 171 candidate and 35 unchecked "Bonus feat." legends, which the owner keeps.
