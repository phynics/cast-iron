# Printed book order evidence

> **Retired import.** The evidence below still governs `books.toml` in the corpus. The `book-order.toml` and upstream order file it names belonged to the retired import (git tag `psrd-last-buildable`) and no longer exist on `main`.

This record distinguishes publisher evidence from the order present in the PSRD dataset. A rule-tree sequence is useful for grouping records, but does not establish printed chapter order. Only chapter titles, first pages, and order are recorded here; no book artwork or PDF text is reproduced.

## Core Rulebook

Paizo's [Pathfinder Roleplaying Game Preview #1](https://cdn.paizo.com/blog/pathfinder-roleplaying-game-preview-1), dated May 13, 2009, links a contents PDF. The printed contents page of the Core Rulebook was also checked in this review. Both give the same main chapter sequence. They differ on one first page: the preview contents put Combat at p. 176, the printed contents page puts it at p. 178. The table follows the printed page. No PDF or artwork is included in Cast Iron.

| Chapter | Title | First page |
|---|---|---|
| 1 | Getting Started | 8 |
| 2 | Races | 20 |
| 3 | Classes | 30 |
| 4 | Skills | 86 |
| 5 | Feats | 112 |
| 6 | Equipment | 140 |
| 7 | Additional Rules | 166 |
| 8 | Combat | 178 |
| 9 | Magic | 206 |
| 10 | Spells | 224 |
| 11 | Prestige Classes | 374 |
| 12 | Gamemastering | 396 |
| 13 | Environment | 410 |
| 14 | Creating NPCs | 448 |
| 15 | Magic Items | 458 |

The printed contents page lists the appendices Special Abilities (554), Conditions (565), Inspiring Reading (568), and Game Aids (569), followed by the license, character sheet, and index. The sequence agrees with the first 15 groups in `books.toml`; the remaining `Glossary` group is an upstream aggregate, not a printed chapter title. Main chapter order is verified; nested placement and appendix fidelity remain incomplete. The dataset still places some Creating NPCs text inside Gamemastering and the prestige-class introduction inside Classes.

## Advanced Player's Guide

The printed contents page of the Advanced Player's Guide was checked in this review. It verifies the following main chapter sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Races | 6 |
| 2 | Classes | 24 |
| 3 | Feats | 148 |
| 4 | Equipment | 174 |
| 5 | Spells | 188 |
| 6 | Prestige Classes | 258 |
| 7 | Magic Items | 280 |
| 8 | New Rules | 318 |

The dataset splits the printed Classes chapter into Base Classes and Core Classes. Main chapter order and titles are verified; nested placement remains unverified.

## Advanced Race Guide

The printed contents page of the Advanced Race Guide was checked in this review. It verifies the following main chapter sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Core Races | 6 |
| 2 | Featured Races | 80 |
| 3 | Uncommon Races | 180 |
| 4 | Race Builder | 212 |
| Appendix 1 | Age | 248 |
| Appendix 2 | Height and Weight | 250 |
| Appendix 3 | Spells | 251 |

The dataset combines Appendix 1 and Appendix 2 in one chapter and has no chapter for Appendix 3. Main chapter order and titles are verified; nested placement remains unverified.

## Bestiary

The printed contents page of the Bestiary was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendix 1 | Monster Creation | 290 |
| Appendix 2 | Monster Advancement | 294 |
| Appendix 3 | Glossary | 297 |
| Appendix 4 | Monsters as PCs | 314 |
| Appendix 5 | Monster Feats | 315 |
| Appendix 6 | Monster Cohorts | 316 |
| Appendix 7 | Animal Companions | 316 |
| Appendix 8 | Monsters by Type | 317 |
| Appendix 9 | Monsters by CR | 318 |
| Appendix 10 | Monsters by Terrain | 319 |
| Appendix 11 | Variant Monster Index | 321 |
| Appendix 12 | Ability Index | 322 |
| Appendix 13 | Monster Roles | 323 |
| Appendix 14 | Encounter Tables | 324 |

The dataset's Universal Monster Rules chapter is part of Monsters A to Z rather than a printed appendix. Miscellaneous Monster Rules combines Appendix 4 and Appendix 6. The dataset has no chapter for Appendix 3, Appendix 7, or Appendices 9-12. Main chapter order is verified; appendix coverage and nested placement remain incomplete.

## Bestiary 2

The printed contents page of Bestiary 2 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 292-318 |

The dataset holds only the monster entries; the printed introduction and appendices are not in it. Main chapter order is verified; appendix coverage remains incomplete.

## Bestiary 3

The printed contents page of Bestiary 3 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 290-318 |

The dataset holds only the monster entries; the printed introduction and appendices are not in it. Main chapter order is verified; appendix coverage remains incomplete.

## Bestiary 4

The printed contents page of Bestiary 4 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 288-316 |
| Appendix 14 | Advanced Awards | 318 |
| Appendix 15 | Mythic Creatures | 318 |

The dataset holds only the monster entries; the printed introduction and appendices are not in it. Main chapter order is verified; appendix coverage remains incomplete.

## Game Mastery Guide

The printed contents page of the Game Mastery Guide was checked in this review. It lists the following chapters.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Getting Started | 6 |
| 2 | Running a Game | 22 |
| 3 | Player Characters | 56 |
| 4 | Nonplayer Characters | 78 |
| 5 | Rewards | 100 |
| 6 | Creating a World | 138 |
| 7 | Adventures | 170 |
| 8 | Advanced Topics | 228 |
| 9 | NPC Gallery | 252 |
| Appendix | Appendix | 310 |

The dataset does not model these chapters: it groups topics instead. The topic chapters that could be matched appear in printed page order, but the printed chapter titles and boundaries are not represented in `books.toml`. Main printed chapter order is recorded; the dataset's topic structure remains provisional.

## Monster Codex

The printed contents page of the Monster Codex was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| — | Boggards | 6 |
| — | Bugbears | 18 |
| — | Drow | 30 |
| — | Duergar | 42 |
| — | Fire Giants | 54 |
| — | Frost Giants | 66 |
| — | Ghouls | 78 |
| — | Gnolls | 90 |
| — | Goblins | 102 |
| — | Hobgoblins | 114 |
| — | Kobolds | 126 |
| — | Lizardfolk | 138 |
| — | Ogres | 150 |
| — | Orcs | 162 |
| — | Ratfolk | 174 |
| — | Sahuagin | 186 |
| — | Serpentfolk | 198 |
| — | Troglodytes | 210 |
| — | Trolls | 222 |
| — | Vampires | 234 |
| — | Appendices | 246 |

The 20 race chapters and their order match `books.toml`. Introduction text is not in the dataset. Main chapter order and titles are verified; nested placement remains unverified.

## Mythic Adventures

The printed contents page of Mythic Adventures was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| — | Glossary | 7 |
| 1 | Mythic Heroes | 8 |
| 2 | Mythic Feats | 52 |
| 3 | Mythic Spells | 78 |
| 4 | Running a Mythic Game | 116 |
| 5 | Mythic Magic Items | 140 |
| 6 | Mythic Monsters | 174 |
| 7 | Fire Over Blackcrag | 230 |

The Glossary is front matter in print. The dataset places it first, and has no chapter for Chapter 7. Main chapter order and titles are verified; nested placement remains unverified.

## NPC Codex

The printed contents page of the NPC Codex was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Core Classes | 8 |
| 2 | Prestige Classes | 198 |
| 3 | NPC Classes | 242 |
| 4 | Iconic Characters | 274 |
| — | Appendices | 298 |

The dataset uses the printed chapter titles for Chapters 1-3 and has no chapter for Chapter 4. Main chapter order and titles are verified; nested placement remains unverified.

## Ultimate Campaign

The printed contents page of Ultimate Campaign was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Character Background | 6 |
| — | Story Feats | 66 |
| 2 | Downtime | 74 |
| 3 | Campaign Systems | 132 |
| 4 | Kingdoms and War | 196 |

The dataset splits Story Feats out of the printed Character Background chapter; the 28 feats are placed there. Main chapter order and titles are verified; nested placement remains unverified.

## Ultimate Combat

The printed contents page of Ultimate Combat was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Classes | 6 |
| 2 | Feats | 76 |
| 3 | Mastering Combat | 124 |
| 4 | Vehicles | 168 |
| 5 | Variant Rules | 188 |
| 6 | Spells | 208 |
| Appendix | Appendix | 250 |

The dataset splits the printed Classes chapter into Classes and Class Archetypes. Main chapter order and titles are verified; nested placement remains unverified.

## Ultimate Equipment

The printed contents page of Ultimate Equipment was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Arms and Armor | 6 |
| 2 | Gear | 54 |
| 3 | Magic Arms and Armor | 112 |
| 4 | Rings, Rods, and Staves | 164 |
| 5 | Wondrous Items | 204 |
| 6 | Artifacts and Other Items | 326 |
| Appendices | Appendices | 360 |

The sequence matches `books.toml`. Main chapter order and titles are verified; nested placement remains unverified.

## Ultimate Magic

The printed contents page of Ultimate Magic was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 4 |
| 1 | Spellcasters | 6 |
| 2 | Mastering Magic | 90 |
| 3 | Feats | 140 |
| 4 | Words of Power | 160 |
| 5 | Spells | 194 |
| Appendix | Appendix | 250 |

The dataset splits the printed Mastering Magic chapter into Mastering Magic and New Familiars, and places the printed appendix material in Updates. Main chapter order and titles are verified; nested placement remains unverified.

## Other books in the curated list

No printed contents page was available in this review for the Technology Guide, so its proposed chapter order and labels remain provisional upstream grouping, not verified printed order. The Game Mastery Guide printed chapter order is recorded above, but its dataset structure remains a topic tree rather than the printed chapters.

No publisher contents listing was located for any publication outside the sixteen curated books. Those titles keep the page-ordered `Page order` and `Unpaged` lists rather than a chapter structure.

## Page attribution in Pf Data

Pf Data records carry attribution in their description lines, such as `‹SOURCE Inner Sea Gods/12›`, or as a `source="Inner Sea Gods/12"` attribute on a stat directive. The page is publisher evidence for that record in that publication: it is where the record starts in print. It is stronger than a website menu or dataset order, and it is the evidence the canonical book index uses.

It is not chapter evidence. A page number does not name a chapter, prove that two records are in the same chapter, or order records that share a page. A record may name several publications, each with its own page. A range or list such as `12-14` or `12, 14` keeps its first page. A SOURCE line with no page, or a record with no SOURCE line, gives no page evidence; the canonical index puts that entry in an explicit Unpaged section and reports it. A derived child inherits its host's attribution unless its own extracted lines carry a SOURCE line. These limits are why the canonical index starts page-ordered rather than claiming printed chapters.

## Curation rule

Keep match rules and record placement independent from evidence status. When a publisher source verifies only a subset, record precisely that subset and retain a review note for the rest. Never infer printed sequence from SQLite nested-set order, PSRD JSON arrays, alphabetical naming, a third-party review, or a product-description bullet list. The curated `books.toml` now records main chapter order from a printed contents page for the books listed above. The Technology Guide, the Game Mastery Guide topic structure, and all nested placement remain provisional and must not be described as complete printed chapter fidelity.

## Canonical library

The canonical library (the default `cargo xtask content`) keeps this curated chapter list and order for the same sixteen publications. It does not add, remove, reorder, or rename a PSRD chapter. Instead it re-addresses each placed entry through the reviewed legacy alias map, so the reviewed tree decides the chapter and the canonical id decides the entry. A shared canonical target appears once in a book, at the first placement; a legacy id whose canonical target is not installed is dropped and listed in coverage.

Canonical entries that the PSRD tree does not place for a publication (new Pf Data content) go into one final chapter, labelled `New entries (page order)`. That chapter is ordered only by the page in the entry's source line and is not evidence that the entries belong at the end of the printed book. Publications outside the sixteen PSRD books keep the page-ordered `Page order` and `Unpaged` lists. This mechanism does not upgrade any provisional chapter sequence: the Core Rulebook and thirteen other publications have their main chapter order checked against a printed contents page, the Game Mastery Guide records its printed chapters but keeps a topic tree, and the Technology Guide remains unverified.
