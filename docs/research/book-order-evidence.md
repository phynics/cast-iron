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

The printed contents page lists the appendices Special Abilities (554), Conditions (565), Inspiring Reading (568), and Game Aids (569), followed by the license, character sheet, and index.

`books.toml` now has the 15 printed chapters followed by two appendix groups, Special Abilities and Conditions. The upstream `Glossary` aggregate holds only links to those two appendices, so it opens the Special Abilities group instead of standing as a chapter of its own. Printed Appendix 3 Inspiring Reading (568) and Appendix 4 Game Aids (569) have no corpus entries, so they have no group.

Main chapter order and nested placement are verified against the printed Core Rulebook. Each entry sits in its printed chapter and section, and the tree follows the printed page order: Races runs Dwarves, Elves, Gnomes, Half-Elves, Half-Orcs, Halflings, Humans (pp. 21-27); the Prestige Classes introduction opens Chapter 11 (p. 374), not Classes; the Creating NPCs introduction opens Chapter 14 (p. 448), not Gamemastering; the Magic and Spellcasting introduction opens Chapter 9 (p. 206); and the spell descriptions follow the alphabetical order stated on p. 224 (pp. 239-371). Entries that share a printed page keep the dataset order inside that page.

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

The dataset split of the printed Classes chapter is kept as two sub-groups, Base Classes and Core Classes, inside the top-level Classes chapter. Main chapter order and titles are verified. Nested placement is also verified against the printed section headings and page order: Races opens with Racial Attitudes (p. 8), Racial Traits (p. 8), and Racial Favored Classes (p. 8), then Dwarves (p. 10), Elves (p. 12), Gnomes (p. 14), Half-Elves (p. 16), Half-Orcs (p. 18), Halflings (p. 20), and Humans (p. 22). Base Classes (p. 26) holds Alchemist (p. 26), Cavalier (p. 32), Inquisitor (p. 38), Oracle (p. 42), Summoner (p. 54), and Witch (p. 65). Core Classes (p. 72) holds Barbarian (p. 74), Bard (p. 80), Cleric (p. 86), Druid (p. 98), Fighter (p. 104), Monk (p. 110), Paladin (p. 116), Antipaladin (p. 118), Ranger (p. 124), Rogue (p. 130), Sorcerer (p. 136), and Wizard (p. 142). Each class option sits under its class: Discoveries (pp. 28-32), Cavalier Orders (p. 34), Judgments (p. 39), Curses (pp. 43-44), Mysteries (p. 45), Eidolons (p. 58), Evolutions (p. 60), Hexes (pp. 66-69), Patron Spells (p. 70), Rage Powers (p. 74), Domains (pp. 86-96), Animal Shamans (pp. 102-103), Rogue Talents (pp. 130-132), Bloodlines (pp. 136-141), and Arcane Schools (pp. 142-146).

The remaining chapters follow the printed sections: Feats (Types of Feats and Feat Descriptions, p. 150), Equipment (Weapons p. 176, Armor p. 179, Goods and Services p. 181), Spells (Spell Lists p. 190, Spell Descriptions p. 200), Prestige Classes (p. 258), Magic Items (Armor p. 283, Weapons p. 286, Rings p. 292, Rods p. 294, Staves p. 296, Wondrous Items p. 300, Cursed Items p. 310, Minor Artifacts p. 313, Major Artifacts p. 316), and New Rules (Combat Maneuvers p. 320, Hero Points p. 322, Traits p. 326). The residual `New entries (page order)` chapter keeps 52 entries whose Advanced Player's Guide attribution is a secondary source and which have no APG section: 37 class entries whose APG text is the shared Archetypes section at p. 72, 7 race entries with their racial-trait children, 6 cross-book rule hubs, and 2 sidekick hubs. They are not placed by guess. The entry set is unchanged.

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

`books.toml` splits the dataset's combined Age, Height & Weight chapter into the two printed appendices, Age and Height and Weight. Printed Appendix 3 Spells (251) has no corpus entries, so it has no group.

Main chapter order and nested placement are verified against the printed Advanced Race Guide. Each entry in the curated chapters sits in its printed chapter and section, and the tree follows the printed page order: the Core Races run Dwarves, Elves, Gnomes, Half-Elves, Half-Orcs, Halflings, Humans (pp. 10-79); each race keeps Alternate Racial Traits before its New Racial Rules, whose Equipment, Feats, Magic Items, and Spells subsections follow print (for example Catfolk pp. 91-95); and the Race Builder keeps the racial-quality and racial-trait descriptions inside Step 2 (p. 215) and Step 3 (p. 220). Entries that share a printed page keep the dataset order inside that page.

The page-ordered list of entries that the curated tree does not place is not a printed chapter position. It keeps its entries in page order and is outside this check.

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

`books.toml` follows this sequence: Monsters A to Z, then the appendices. Universal Monster Rules, whose rules sit inside the printed Monsters A to Z section, is kept as a separate dataset appendix after it. The printed Appendices 4 and 6 are separate top-level groups, Monsters as PCs and Monster Cohorts; the dataset's Miscellaneous Monster Rules aggregate opens Monsters as PCs. The dataset's Creature Types appendix uses the printed Appendix 8 title, Monsters by Type. Printed Appendix 3 Glossary (297), Appendix 7 Animal Companions (316), and Appendices 9-12 (Monsters by CR, Monsters by Terrain, Variant Monster Index, Ability Index) have no corpus entries, so they have no groups.

Main chapter order and nested placement are verified against the printed Bestiary. Inside Monsters A to Z, each printed family heading holds its own monsters in page order: Angel (p. 9; astral deva p. 10, planetar p. 11, solar p. 12), Archon (p. 18), Azata (p. 23), Demon (p. 56), Devil (p. 71), Dragon (p. 90; chromatic black p. 92, blue p. 94, green p. 96, red p. 98, white p. 100; metallic brass p. 102, bronze p. 104, copper p. 106, gold p. 108, silver p. 110), Elemental (stat blocks pp. 120-127), Familiar (p. 131), Golem with the Shield Guardian subsection (p. 158), and Lycanthrope (p. 197). Each template heading keeps its example creatures: Ghost (p. 144), Half-Celestial (p. 169), Half-Dragon and Dracolisk (p. 170), Half-Fiend (p. 171), Lich (p. 188), Skeleton (p. 250), Skeletal Champion (p. 252), Vampire (p. 270), and Zombie (p. 288).

The appendices hold their printed sections. The Introduction keeps the Monster Rules and Stat Block text (p. 5). Monster Creation keeps its steps (pp. 290-293). Monster Advancement keeps the simple templates and the class-level steps (pp. 294-297). The Glossary creature types and subtypes sit under Monsters by Type (pp. 306-310; the group title is the printed Appendix 8 title, p. 317). Monster Feats keeps Damage Die Adjustments (pp. 314-316). The encounter tables keep their printed regions (pp. 324-327). Animal companion stat blocks, diseases, poisons, and variant stat blocks sit inside the monster that prints them. The alphabetical order follows the printed pages; entries that share a printed page keep the dataset order.

Four entries remain in the residual `New entries (page order)` chapter because their Bestiary attribution is a cross-book hub or a split of another book's entry, not a printed Bestiary section: Gamemastery, Rare Races, Slimes, Molds, and Fungi, and a skaveling affliction from Bestiary 2.

## Bestiary 2

The printed contents page of Bestiary 2 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 292-318 |

The dataset's Bestiary 2 monster entries sit in the top-level group with the printed title Monsters A to Z; the printed introduction and appendices have no curated group. Main chapter order and nested placement are verified against the printed Bestiary 2. Inside Monsters A to Z, each printed family heading holds its own monsters in page order: Aeon (p. 8; akhana p. 9, bythos p. 10, paracletus p. 11, pleroma pp. 12-13, theletos p. 14), Agathion (p. 15; avoral p. 16, cetaceal p. 17, draconal pp. 18-19, leonal p. 20, silvanshee p. 21, vulpinal p. 22), Daemon (p. 62), Gremlin (p. 141), Inevitable (p. 161), Nightshade (p. 199), Protean (p. 213), and Qlippoth (p. 218). The primal dragons keep the printed colour headings: Dragon, Primal (p. 92), with brine p. 94, cloud p. 96, crystal p. 98, magma p. 100, and umbral p. 102, each holding its base stat block and age variants. The ice (p. 114), lightning (p. 116), magma (p. 118), and mud (p. 120) elementals keep their size variants, and each golem (pp. 134-139) keeps its Construction section.

The printed template entries keep their rules and example stat block together: Ogrekin (p. 204), Petitioner (p. 208), Ravener (p. 230), Worm That Walks (p. 286), and Zombie, Juju (p. 291). Dataset splits sit inside the entry that prints them: animal companion stat blocks (for example Allosaurus p. 90, Hippopotamus p. 157, Arsinoitherium p. 186, Baboon p. 212, Stingray p. 232, and Giant Snapping Turtle p. 273), the giant dragonfly nymph (p. 105), the Merrow stat blocks (p. 189), the werebear, wereboar, and weretiger forms (pp. 181-183), the Spriggan size variants (p. 257), and the duplicate stat blocks. The Merrow and lycanthrope headings are synthesized because the dataset has no heading entry for them. Monsters A to Z follows the printed pages; entries that share a printed page keep the dataset order.

Entries whose printed home is an appendix or another book remain in the residual `New entries (page order)` chapter because this book tree has no appendix chapters: the simple templates Entropic and Resolute (pp. 292-293), Monster Creation Step 2 (p. 293), Lycanthropic Empathy (p. 298), the Nightshade subtype (p. 308), the cross-book creature subtypes, the carried Bestiary universal monster rules, and the Elemental family entry. Appendix coverage remains incomplete.

## Bestiary 3

The printed contents page of Bestiary 3 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 290-318 |

The dataset holds only the monster entries, in a top-level group with the printed title Monsters A to Z; the printed introduction and appendices are not in it. Main chapter order is verified; appendix coverage remains incomplete.

## Bestiary 4

The printed contents page of Bestiary 4 was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 5 |
| — | Monsters A to Z | 7 |
| Appendices 1-13 | Appendices | 288-316 |
| Appendix 14 | Advanced Awards | 318 |
| Appendix 15 | Mythic Creatures | 318 |

The dataset holds only the monster entries, in a top-level group with the printed title Monsters A to Z; the printed introduction and appendices are not in it. Main chapter order is verified; appendix coverage remains incomplete.

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

`books.toml` now uses these printed chapters, in printed order: Introduction, Getting Started, Running a Game, Player Characters, Nonplayer Characters, Rewards, Creating a World, Adventures, Advanced Topics, and NPC Gallery. Each chapter node opens its printed chapter entry, and each chapter holds its printed section headings in printed page order. The printed Appendix (310) has no corpus entries, so it has no group.

Main chapter order and nested placement are verified against the printed Game Mastery Guide. Getting Started runs Duties of a GM (p. 8), Tone and Maturity (p. 10), The GM as Host (p. 12), Rules of the House (p. 14), Preparing to Run a Game (p. 16), and Creating a Campaign Guide (p. 20). Running a Game runs How to Run a Game (p. 24), The Art of GMing (p. 28), The Science of GMing (p. 34), GM Considerations (p. 46), and Tools for Game Masters (p. 52). Player Characters runs Starting Characters (p. 58), New Players (p. 63), The Life of a Party (p. 64), and Player Interactions (p. 70). Nonplayer Characters runs Cast of Characters (p. 80), Making NPCs Unique (p. 84), Life of the NPC (p. 86), NPC Boons (p. 88), Villains (p. 90), and NPC Creator's Toolbox (p. 94). Rewards runs The Role of Rewards (p. 102), Starting Treasure (p. 108), What is a Reward? (p. 110), PCs Controlling Rewards (p. 114), and Random Item Generators (p. 118). Creating a World runs World Building (p. 140), Detailing Your World (p. 142), Geography (p. 146), Cultural Considerations (p. 148), The Primitive Society (p. 150), The Feudal Society (p. 152), The Rural/Agrarian Society (p. 154), The Cosmopolitan Society (p. 156), Other Societies (p. 158), Technology (p. 160), Time (p. 162), The Cosmos (p. 164), The Planes (p. 166), and Parallel Worlds (p. 168). Adventures runs Elements of Adventure (p. 172), Dungeons (p. 174), Planar (p. 184), Taverns (p. 198), Urban (p. 202), Water (p. 214), and Wilderness (p. 220). Advanced Topics runs Customizing Your Game (p. 230), Chases (p. 232), Disasters (p. 234), Drugs and Addiction (p. 236), Fortune-Telling (p. 238), Gambling and Games of Chance (p. 240), Haunts (p. 242), Hazards (p. 244), Mysteries and Investigations (p. 246), Puzzles and Riddles (p. 248), and Sanity and Madness (p. 250).

The NPC Gallery runs NPC Boons (p. 254), the NPC list (p. 255), and the printed role groups from Adventurers (p. 255) through Villagers (p. 308), each holding its three NPCs. Sanity and Madness keeps Curing Insanity and Types of Insanity, with all seven insanity types of the printed table under Types of Insanity (p. 251), Dissociated Identity and Moral Insanity included. Three printed headings have no corpus entry and are group nodes: Duties of a GM (p. 8), How to Run a Game (p. 24), and Hazards (p. 244). Entries that share a printed page keep the dataset order inside that page. The entry set is unchanged.

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

## Technology Guide

The printed contents page of Technology Guide was checked in this review. It verifies the following sequence.

| Chapter | Title | First page |
|---|---|---|
| Introduction | Introduction | 2 |
| — | Technology in the World | 4 |
| — | Technological Equipment | 18 |
| — | Technological Hazards and Artifacts | 54 |

The corpus already has this order. The printed Introduction has no corpus chapter. Two top-level chapters are renamed to the printed titles: `Equipment` to `Technological Equipment`, and `Hazards and Artifacts` to `Technological Hazards and Artifacts`. The final `New entries (page order)` chapter holds canonical entries the curated tree does not place; it is page-ordered and is not a printed chapter. No entry moves between chapters, and the entry set is unchanged.

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

The dataset's Story Feats split stays as a sub-group inside the printed Character Background chapter. Main chapter order and titles are verified; nested placement remains unverified.

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

The dataset's Class Archetypes split stays as a sub-group inside the printed Classes chapter. Main chapter order and titles are verified; nested placement remains unverified.

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

The dataset's New Familiars split stays as a sub-group inside the printed Mastering Magic chapter. The printed appendix material is the top-level Appendix chapter; its Alphabetized Words of Power section has no corpus entry. Main chapter order and titles are verified; nested placement remains unverified.

## Other books in the curated list

The printed chapters of the Game Mastery Guide are recorded above, and its `books.toml` tree now uses them.

No publisher contents listing was located for any publication outside the sixteen curated books. Those titles keep the page-ordered `Page order` and `Unpaged` lists rather than a chapter structure.

## Page attribution in Pf Data

Pf Data records carry attribution in their description lines, such as `‹SOURCE Inner Sea Gods/12›`, or as a `source="Inner Sea Gods/12"` attribute on a stat directive. The page is publisher evidence for that record in that publication: it is where the record starts in print. It is stronger than a website menu or dataset order, and it is the evidence the canonical book index uses.

It is not chapter evidence. A page number does not name a chapter, prove that two records are in the same chapter, or order records that share a page. A record may name several publications, each with its own page. A range or list such as `12-14` or `12, 14` keeps its first page. A SOURCE line with no page, or a record with no SOURCE line, gives no page evidence; the canonical index puts that entry in an explicit Unpaged section and reports it. A derived child inherits its host's attribution unless its own extracted lines carry a SOURCE line. These limits are why the canonical index starts page-ordered rather than claiming printed chapters.

## Curation rule

Keep match rules and record placement independent from evidence status. When a publisher source verifies only a subset, record precisely that subset and retain a review note for the rest. Never infer printed sequence from SQLite nested-set order, PSRD JSON arrays, alphabetical naming, a third-party review, or a product-description bullet list. The curated `books.toml` records main chapter and appendix order from a printed contents page for the books listed above.

Top-level groups follow the printed chapters and appendices. A dataset split that print does not make stays as a sub-group inside its printed chapter; a dataset hub entry that print splits down becomes the opening entry of the first printed chapter it links to. A printed chapter or appendix with no entries gets no group. Nested placement is checked separately and must not be described as complete printed chapter fidelity.

## Canonical library

The canonical library (the default `cargo xtask content`) keeps this curated chapter list and order for the same sixteen publications. It does not add, remove, reorder, or rename a PSRD chapter. Instead it re-addresses each placed entry through the reviewed legacy alias map, so the reviewed tree decides the chapter and the canonical id decides the entry. A shared canonical target appears once in a book, at the first placement; a legacy id whose canonical target is not installed is dropped and listed in coverage.

Canonical entries that the PSRD tree does not place for a publication (new Pf Data content) go into one final chapter, labelled `New entries (page order)`. That chapter is ordered only by the page in the entry's source line and is not evidence that the entries belong at the end of the printed book. Publications outside the sixteen PSRD books keep the page-ordered `Page order` and `Unpaged` lists. This mechanism does not upgrade any provisional chapter sequence: the sixteen publications have their main chapter and appendix order checked against a printed contents page, while nested placement remains a separate check.
