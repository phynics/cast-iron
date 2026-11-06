# Missing PF1e book source audit

> **Retired import.** This document describes the upstream import that built the library before it moved to the pinned plain-text corpus. The import code is preserved at the git tag `psrd-last-buildable`; the facts below stay as evidence and history, and the paths they name no longer exist on `main`.

Checked 2026-11-03. This is a source-shape and notice audit, not legal clearance. Repository code licenses do not settle rights in game data. Record counts below are reproducible counts of named JSON objects carrying an exact source-title string in `sources`; duplicate appearances across files are counted as occurrences, not deduplicated entities. They show concrete tagged data, not completeness or book-order fidelity.

## Pinned sources and method

- **Pf Data 1e:** `jasontankapps/pathfinder-data-1-e` commit [`08ea71f24375908d0190c822fd1fd080797667a9`](https://github.com/jasontankapps/pathfinder-data-1-e/tree/08ea71f24375908d0190c822fd1fd080797667a9). Inspected every `json/*.json`; recursively counted dictionaries with `name` and an exact book title in `sources`. The JSON is human-readable Markdown-backed content, not a book/chapter ordered corpus. Names and proper nouns remain present; the source's global Product Identity declaration expressly excludes them from OGC. The repository says the JSON directory is OGC, while non-OGC app material is copyright 2024 Jason Tamez and GPL-3.0-or-later. Its `OPENGAMECONTENT.md` contains OGL 1.0a, Section 15 lines naming each relevant publication and a Paizo PI declaration; README separately states Community Use Policy use and that the app is unofficial/free. These notices are evidence of the publisher's stated licensing boundaries, not a record-by-record clearance.
- **PCGen:** `PCGen/pcgen` commit [`69ad2ab9145038f4b0056c161ffeebf84daa154a`](https://github.com/PCGen/pcgen/tree/69ad2ab9145038f4b0056c161ffeebf84daa154a/data/pathfinder/paizo). Inspected Paizo `.pcc` manifests and their referenced `.lst` files. The counts below are manifest `.lst` references, not unique mechanics or complete book records. The repo's LGPL code license does not grant a data license. Manifests contain per-source OGL 1.0a and copyright lines; several also carry a CUP/Unofficial notice. They can include PCGen-authored conversions and third-party OGL material alongside Paizo content. Use requires source-level selection and attribution review.
- **AoN:** consulted only as navigation/reference evidence through the existing local AoN mapping audit. No AoN pages or archive data were imported.

Paizo's current [Community Use Policy](https://paizo.com/licenses/communityuse) is the separate reference for descriptive Paizo marks/names and its required terms; the [licenses overview](https://paizo.com/licenses) describes OGL and CUP routes. The existing [licensing audit](licensing-audit.md) records direct 2026-11-03 review of those policy pages and their restrictions. Neither a repo's OGL file nor a CUP notice grants permission to reproduce Product Identity prose, artwork, trade dress, or third-party material without its applicable permission.

## The 14 missing AoN headings

| AoN heading | Pf Data 1e tagged occurrences / files | What the tagged records actually indicate | PCGen evidence at pinned commit |
|---|---:|---|---|
| Pathfinder #28: The Infernal Syndrome | 16 / 8 | Monsters, feats, and campaign/rules records; adventure-specific named plot material must not be inferred licensed from these records. | No matching AP manifest found. |
| Heroes of the Wild | 79 / 43 | Primarily player options: feats, race records, archetypes, spells, equipment; a few rules records. | `player_companion/heroes_of_the_wild/_heroes_of_the_wild.pcc`, 10 `.lst` references; OGL, book copyright, and CUP notice. |
| Pathfinder Unchained | 932 / 39 | Mixed, with substantial rules/subsystem and skills material plus feats/class abilities. | `roleplaying_game/pathfinder_unchained/_pathfinder_unchained.pcc`, 16 `.lst` references; OGL and exact book copyright. An additional `_pfs/_.pcc` repeats source/copyright notice, with no `.lst` references. |
| Occult Adventures | 625 / 99 | Mixed rules and character options: occult classes/abilities, rituals, spells, haunts, feats, archetypes. | `roleplaying_game/occult_adventures/_occult_adventures.pcc`, 25 `.lst` references; OGL and exact book copyright. |
| Ultimate Intrigue | 543 / 100 | Mixed intrigue subsystem/rules records and player options. | `roleplaying_game/ultimate_intrigue/_ultimate_intrigue.pcc`, 21 `.lst` references; OGL, book copyright, CUP notice. |
| Inner Sea Intrigue | 109 / 53 | Mostly character options, equipment, and setting-linked records; not a complete setting chapter. | `campaign_setting/inner_sea_intrigue/_inner_sea_intrigue.pcc`, 16 `.lst` references; OGL and exact source copyright, CUP notice. Its manifest's `SOURCELONG` says “Pathfinder Player Companion: Inner Sea Intrigue,” unlike the book title/Section 15 line. |
| Horror Adventures | 498 / 101 | Mixed: madness, curses, haunts, occult rituals, monsters and character options. | `roleplaying_game/horror_adventures/_horror_adventures.pcc`, 26 `.lst` references; OGL, book copyright, CUP notice. Its copyright line contains the typo “Pathfnder”. |
| Adventurer's Guide | 303 / 92 | Mainly prestige classes, archetypes, feats, equipment, and options; limited book rules material. | `roleplaying_game/adventurers_guide/_adventurers_guide.pcc`, 19 `.lst` references; OGL, book copyright, CUP notice. Some conversion copyright text refers to a different Player's Guide, requiring attribution review. |
| Book of the Damned | 284 / 55 | Mixed faith/domain records, monsters, rituals, and options. Faith descriptions and setting prose need PI filtering. | No direct manifest for the 2017 `Pathfinder Roleplaying Game Book of the Damned` found. PCGen has older Book of the Damned Volumes I and II manifests (12/14 `.lst` references plus 4/14 in companion PFS manifests); the inventory reports these as related titles, not an exact match. |
| Ultimate Wilderness | 693 / 122 | Mixed rules/subsystems, monsters, plant items, classes, archetypes, feats and equipment. | `roleplaying_game/ultimate_wilderness/_ultimate_wilderness.pcc`, 35 `.lst` references; OGL, book copyright, CUP notice. |
| Pathfinder #128: Songbird, Scion, Saboteur | 17 / 6 | Small mixed set: relics, monsters, and rules records; not the adventure plot. | No matching AP manifest found. |
| Planar Adventures | 257 / 80 | Mixed options, monsters, faiths, and setting/rules records; does not imply chapter-order coverage. | No matching Planar Adventures manifest found. |
| Heroes of Golarion | 106 / 35 | Primarily player options and class abilities, with a campaign/rules chunk. | `player_companion/heroes_of_golarion/_heroes_of_golarion.pcc`, 19 `.lst` references; OGL and exact book copyright; no CUP notice observed in this manifest. |
| Pathfinder #144: Midwives to Death | 20 / 15 | Small set of faiths, haunts, monsters, and class options; no adventure plot coverage. | No matching AP manifest found. |

PCGen has direct source manifests for nine of the 14 exact titles. Five have no exact source match in the inspected tree: the three Adventure Path titles, Planar Adventures, and the 2017 Book of the Damned (older volumes are listed separately above). This is limited to the inspected Paizo tree at that commit, not a claim those books have no records elsewhere in PCGen.

## Practical choice

Use **Heroes of the Wild, limited to the OGC-tagged option records in Pf Data 1e**, as the first bounded adapter spike. Its 79 source-tagged occurrences across 43 JSON files are a small, concrete corpus; the sample families span race options, feats, archetypes, spells, equipment, and a small number of rules records. This gives a meaningful cross-family test without adopting an entire new data system. Resolve duplicate records deterministically, convert the documented custom Markdown/directive links to typed nodes, preserve provenance and exact OGL attribution, omit artwork and PI prose, and report unsupported or excluded records. Do not claim chapter order: neither this JSON layout nor the PCGen manifests establish it.

The adapter should first inventory exact object IDs and unsupported markup before creating source configuration. Preserve the source's stated OGC boundary and attribution; do not treat other Pf Data app content as part of its OGC JSON grant.

## Reproduction

From the repository root, fetch the two checkouts into temporary directories and pin them to the audited commits:

```sh
git clone https://github.com/jasontankapps/pathfinder-data-1-e.git /tmp/pf1-data-audit
git -C /tmp/pf1-data-audit checkout --detach 08ea71f24375908d0190c822fd1fd080797667a9
git clone https://github.com/PCGen/pcgen.git /tmp/pcgen-audit
git -C /tmp/pcgen-audit checkout --detach 69ad2ab9145038f4b0056c161ffeebf84daa154a
python3 scripts/inventory-missing-sources.py /tmp/pf1-data-audit /tmp/pcgen-audit \
  --output /tmp/missing-book-inventory.json
```

The script rejects either checkout if `git rev-parse HEAD` differs from its embedded pin. Its stable JSON reports exact source tags, named-object occurrence counts, containing files and filename families; PCGen `.pcc` paths, `.lst` directive reference counts, and explicit `no_match` statuses. It does not modify the checkouts or repository content.

## Primary source links

- [Pf Data 1e README at pinned commit](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/README.md), [OPENGAMECONTENT.md](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/OPENGAMECONTENT.md), [JSON format guide](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/JSON.md).
- [PCGen pinned data tree](https://github.com/PCGen/pcgen/tree/69ad2ab9145038f4b0056c161ffeebf84daa154a/data/pathfinder/paizo); for example [Heroes of the Wild manifest](https://github.com/PCGen/pcgen/blob/69ad2ab9145038f4b0056c161ffeebf84daa154a/data/pathfinder/paizo/player_companion/heroes_of_the_wild/_heroes_of_the_wild.pcc) and [Pathfinder Unchained manifest](https://github.com/PCGen/pcgen/blob/69ad2ab9145038f4b0056c161ffeebf84daa154a/data/pathfinder/paizo/roleplaying_game/pathfinder_unchained/_pathfinder_unchained.pcc).
- [Paizo Community Use Policy](https://paizo.com/licenses/communityuse) and [Paizo licensing overview](https://paizo.com/licenses).

For declaration locations, Pf Data 1e [`README.md`, “Licenses”](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/README.md) labels `json/` as OGC and points to `OPENGAMECONTENT.md`; that file's `## Open Game License` includes OGL 1.0a and Section 15, and its `## Product Identity` contains Paizo's declaration. PCGen declarations are in each matched `.pcc` under `COPYRIGHT:` and, where present, `INFOTEXT:`; e.g. the Heroes of the Wild manifest above contains its OGL line, the Paizo book copyright/author line, and the CUP/unofficial notice. The inventory output preserves the exact matched lines for review.

## Priority after passage matching

The separate opening-passage audit found candidate records for 13 passages missing from the installed snapshot, including existing Core Rulebook, GameMastery Guide, and Mythic Adventures text. This provides a smaller first integration than a new book. Begin with `json/rules_basics04.json#/magic`, named Magic and Spellcasting, at the same pinned Pf Data commit. It contains the Magic Basics passage from Core Rulebook page 206, with a source marker and ordinary prose. Its description totals 756 characters across five strings. Its text and source match the primary AoN page; the import must still preserve notices and typed rendering. The exact candidate inventory is in `sources/psrd-data/aon-rule-evidence.json`.

Heroes of the Wild remains the first proposed new-book trial after that bounded adapter works. Its source-tagged options must not be presented as the complete printed book. No material from either candidate has been added to the public library by this audit.
