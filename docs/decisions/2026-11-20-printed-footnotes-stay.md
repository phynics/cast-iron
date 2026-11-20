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
- On corpus pin v0.1.30 the footnote TSV has 153 `no` blocks: 80 stay (`presence_basis = print`: 65 print-match, 11 paraphrase, 4 legend) and 73 go (40 pointers, 15 commentary notes, 5 compiled lists, 13 unclear). Of the 19 outside rows with `action = remove`, 18 are removed; `ref/magic-wondrous/maniac-hand` stays, because the paragraph is a stat line of the item (data PR #115 left it for the same reason).
- The corpus change is `phynics/cast-iron-data#117`. A later reader who finds a printed note that AoN lacks should not remove it.
- This entry does not decide the 171 candidate and 35 unchecked "Bonus feat." legends, which the owner keeps.
