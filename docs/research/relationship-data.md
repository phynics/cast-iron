# Relationship data in PSRD-Data

Checked 2026-11-03 against the pinned snapshot (`76ea4784604145ba3dea40eb56f83bb904759467`, 16 selected books). The dataset is untrusted: every number below comes from parsing JSON or from read-only queries (`sqlite3 -readonly`, `mode=ro`). `scripts/survey-relationships.py` reproduces sections 1 to 4 and the database order in section 6. The compiler results (resolution rates, placement) are the ones in `coverage.json` of the library built from this branch.

Marks: **verified** means a script or the compiler measured it. **Unverified** means it rests on memory of the printed books or on a guess, and a person must confirm it.

## 1. Record families with no standalone category before this change

All records have `type`, `name`, `url`, and `source`. The `url` is the hierarchy: `pfsrd://<Book>/<Section>/.../<Name>`.

| Family | Typed records | Also inside the selected rules trees | Decision |
|---|---|---|---|
| `racial_trait` | 342: APG 50, ARG 225, CRB 67 | APG 50 (CRB and ARG copies sit inside the typed `race` records, 292) | First-class kind `racial_trait` |
| `class_archetype` | 221: APG 81, UC 80, UM 55, Tech 5 | all 221 once | First-class kind `archetype`, 218 promoted, 3 not (see below) |
| `trait` | 298: APG 78, Ultimate Campaign 220 | all 298 once | First-class kind `trait` (20 omitted by policy) |
| `drawback` | 17: Ultimate Campaign | all 17 once | `trait` with category `Drawback` |
| `affliction` | 239 | 74 | Already supported (typed directories were selected) |
| `animal_companion` | 68: Bestiary 23, CRB 17, B2 14, UM 10, MC 4 | 28 (MC 8, UM 20); the other 40 are in no selected tree | Not promoted |
| `mythic_spell` | 269 (Mythic Adventures) | 0 | Not promoted; the content is absent from the library |
| `vehicle` 19, `settlement` 8, `haunt` 1 | as listed | all, once each | Not promoted |
| `resource` 193, `army` 45, `kingdom_resource` 70 | Ultimate Campaign | all, once each | Not promoted |

Every nested copy of the four promoted families is byte-identical to its typed record (racial traits 342 of 342, archetypes 224 of 224, traits 298 of 298, drawbacks 17 of 17). Replacing a copy by a link loses no text. The 224 archetype copies include 3 inside a class record. In the library, nested copies replaced by an entry link went from 4,666 to 5,521 (+855, one per new entry).

### Fields and parents

- **`racial_trait`** (fields: `body`, optional `description` such as `+2 Constitution, +2 Wisdom, -2 Charisma`, `subtype` = race in the singular, `dwarf`). Parent: the typed `race` record that contains it. CRB: 67 under `Races/<Race>/<Race> Racial Traits/`, all standard traits of the 7 core races. ARG: 225 under `Races/<Race>/Alternate Racial Traits/`, parent race record above each in the url (225 of 225). APG: 50 under `Rules/Races/<Race>/Alternate Racial Traits/`; no race record exists above them, because the APG races are sections of a rules tree. They name a race (`Gnomes`) that exists as a CRB race. 275 racial traits are alternate and 67 are standard (verified by url).
- **`class_archetype`** (fields: `body`, `subtype` = class name in lowercase, `sections` = the archetype's abilities, type `ability`). `subtype` equals a class title for 221 of 221 records (verified). Parent class: the class entry with that title. Three records are not archetypes: APG `Class Features`, `Class Skills`, and `Ex-Antipaladins` under `Rules/Core Classes/Paladin/Antipaladin/` are typed `class_archetype` with subtype `paladin`. The check that rejects them: the url's parent segment must end in `Archetypes`. They stay text inside the Antipaladin class and are listed in coverage as unpromoted.
- **`trait`** (fields: `body`, `subtype`, optional `abbrev` on 52, `sections` on 4). `subtype` is the category: `combat`, `faith`, `magic`, `social`, `campaign`, `regional` (APG) or `region` (Ultimate Campaign), `religion`, `bloodline`, and `racial_<race>` (`racial_half_elf`). It matches the url (`Traits/Basic Traits/Combat Traits/`, `Traits/Race Traits/Dwarf Race Traits/`). The library has 295 traits: Combat 50, Faith 41, Magic 62, Social 55, Campaign 3, Race 34, Regional 18, Religion 15, Drawback 17.
- **`drawback`**: no `subtype`; all 17 sit under `Traits/Drawbacks/`.

### Policy catch

The 20 Ultimate Campaign religion traits are omitted for Product Identity by `content-policy.toml`, which names their container `.../Traits/Religion Traits`. The policy matched exact urls, so the typed files below that container would have passed it. It now excludes every url below an excluded url. The 20 appear in the rejected list with their reason.

## 2. Feat prerequisites

- 1,099 feat records; 1,002 (91.2%) have a prerequisites section. **The wording is in the section's `description`, not its `body`, for 811 of them**; 191 have it in `body`. Before this change the extractor read only `body`, so `FeatFields.prerequisites` was empty for those 811.
- No prerequisite text contains a link, so no record names its prerequisite feats by url (0 of 1,002). The index database repeats the same text (`central_index.feat_prerequisites`, 939 rows including Advanced Class Guide); it adds no structure.
- Parsing: split at top-level commas and semicolons, split alternatives at ` or `, and read each requirement as an ability score, base attack bonus, skill ranks, level, class feature, racial trait, race, or a name. A name resolves when its normalized form equals one feat title exactly (punctuation and case ignored, `(...)` stripped, `with selected weapon` stripped), and a title shared by two books resolves only within the dependent feat's book. Nothing is fuzzy.
- Result on the library: 2,521 requirements in 1,002 feats.

| Reading | Requirements | Share |
|---|---:|---:|
| feat (resolved to an entry) | 969 | 38.4% |
| ability score | 279 | 11.1% |
| race | 258 | 10.2% |
| base attack bonus | 223 | 8.8% |
| class feature | 158 | 6.3% |
| skill ranks | 141 | 5.6% |
| level (class, character, caster, mythic tier) | 120 | 4.8% |
| racial trait | 44 | 1.7% |
| other (unread) | 329 | 13.1% |

- 597 of 1,002 feats name at least one resolved feat: 971 `prerequisite` edges to 314 distinct feats. Of the 1,298 requirements that are not numeric, a class feature, a racial trait, or a race, 969 resolve to a feat: **74.7%**. The 329 unread requirements are mostly not feats (`Small size`, `Darkvision`, `Ability to cast dispel magic`, `Cursed Birth background`). 119 start with a capital letter. A few are feats that this compiler does not resolve, such as `Light Armor Proficiency` (the feat is titled `Armor Proficiency, Light`). They stay in `coverage.json` (`relations.prerequisites.unread`) with the feat id. Ambiguous names: 0.
- Longest chain: 5 feats (`Bashing Finish` > `Shield Master` > `Shield Slam` > `Improved Shield Bash` > `Shield Proficiency`). Most required: `Improved Unarmed Strike` 94, `Combat Expertise` 47, `Power Attack` 41.
- Either-or lists (`Great Fortitude, Iron Will, or Lightning Reflexes`) are marked with the relation label `or`. The rule is a heuristic (a list is either-or when it ends with `, or X` and every item is a name). The flat `prerequisite_feats` list contains all alternatives. Treat it as "named in the prerequisite text", not as "all required".
- Class levels (`Fighter level 4th`) and races are counted but not turned into relations: only a feat can be a `Prerequisite` target today. The relation kind allows other targets.

## 3. Spell lists

- 1,409 spells, 8,504 `(class, level)` pairs, 23 distinct class names. The nested-set databases hold the same pairs: `spell_lists` rows in the 8 books with spells total 8,504 (verified).
- 494 pairs are repeats inside one spell (`Hunter` 492, `Oracle` 2; 287 spells). After removing them: 8,010 distinct pairs. No spell lists one class at two levels.
- Names are already consistent (no case variants, no `Sorcerer/Wizard`: none of the 23 contains a `/`). Pairs per class: Hunter 868, Sorcerer 850, Wizard 850, Arcanist 847, Witch 486, Oracle 466, Cleric 460, Warpriest 460, Druid 381, Bard 349, Skald 349, Inquisitor 310, Shaman 248, Magus 230, Summoner 219, Alchemist 216, Bloodrager 188, Ranger 169, Paladin 139, Elementalist Wizard 129, Investigator 125, Antipaladin 97, Adept 68.
- Mapping to class entries by title: 15 of 23 map (Adept, Alchemist, Antipaladin, Bard, Cleric, Druid, Inquisitor, Magus, Oracle, Paladin, Ranger, Sorcerer, Summoner, Witch, Wizard). **Unmapped: Arcanist, Bloodrager, Hunter, Investigator, Shaman, Skald, Warpriest** (classes of books outside the library) and **Elementalist Wizard** (129 spells, all at the same level as Wizard; a Wizard school list, not a class). They stay as lists without a class entry and appear in coverage.
- Decision: `lists.json` indexes `SpellFields.levels`; there is no `SpellListMember` relation kind (see `docs/decisions.md`).

## 4. Race and racial traits

- **Standard racial traits.** CRB: typed `racial_trait` records, 67 (Dwarves 12, Elves 9, Gnomes 12, Half-Elves 10, Half-Orcs 9, Halflings 9, Humans 6), nested in the 7 typed CRB `race` records. ARG: the standard traits of its 37 races are plain `section` children (351 sections in 37 race records), not typed. They stay inside the race body. This is the largest gap in "races with their racial traits".
- **Alternate racial traits.** ARG: 225 typed, nested in the ARG race records. APG: 50 typed, nested only in `apg/rules/races.json`; they belong to CRB races by name.
- Races in total: 79 (CRB 7, ARG 37, and 35 monster races from the Bestiaries, subtype `monster_race`). The compiler now tags each race with its subtype (`standard_race`, `core_race`, `featured_race`, `uncommon_race`, `monster_race`).
- Not covered: "replaces X" links between an alternate trait and the standard trait it replaces; ARG `Racial Subtypes` and `Favored Class Options` (plain sections); ARG racial archetypes (sections with urls).

## 5. Relation counts in the library

| Relation | Count | Source of the parent |
|---|---:|---|
| `part` racial trait to race | 342 | race record that nests it: 292; name match to the CRB race: 50 (all `alternate`) |
| `archetype` to class | 218 | url ancestor or `subtype` equals the class title; 218 of 218 resolve |
| `prerequisite` feat to feat | 971 | exact title match, section 2 |
| total | 1,531 | |

Unplaced parts (racial trait or archetype without a parent): 0.

## 6. Original chapter order

### What the dataset has

**Verified.**

- The `sections` table of each book database is a nested set: `lft`, `rgt`, `parent_id`. Document order of the JSON `sections` arrays equals `lft` order in 100 of the 101 selected rules trees (15,464 nodes compared; one inversion in `monster_codex/rules/drow.json`).
- The JSON trees are one file per child of `Rules`. The order of those files exists only in the database.
- Typed lists (Spells, Races, Skills, Feats, Classes, Monsters) are separate roots of the database, placed before `Rules`. In the Core Rulebook the root order is `Spells, Races, Skills, Feats, Classes, Rules`, which is not the printed order.
- Order inside a typed list is alphabetical with punctuation ignored, with exceptions. Core Rulebook races: `Dwarves, Elves, Gnomes, Half-Elves, Halflings, Half-Orcs, Humans`. 15 of the 622 neighbouring pairs among the 623 Core Rulebook spells break the alphabet (`Summon Monster VIII` before `VII`, `Polymorph Any Object` before `Polymorph`). Creature families follow size or age, not the alphabet (Bestiary `Elemental, Air`, `Small`, `Medium`, `Large`, `Huge`, `Greater`). So the database order is not the alphabet, and it is not always the printed order either: the Ultimate Combat classes come out `Ninja, Samurai, Gunslinger`.

### Comparison with the printed tables of contents

The table began as a comparison against recollection of the books. The Core Rulebook main sequence was subsequently checked against Paizo's contents PDF. All other proposed sequences remain unverified. Current publisher evidence is recorded in [Printed book order evidence](book-order-evidence.md); Legacy website sections are recorded separately in [AoN and legacy PRD book sections](aon-book-sections.md).

| Book | Dataset root, then `Rules` children (in order) | Proposed printed order | Evidence and gaps |
|---|---|---|---|
| Core Rulebook | Spells, Races, Skills, Feats, Classes; Rules: Getting Started, Classes, Using Skills, Feats, Equipment, Additional Rules, Combat, Magic, Spells, Gamemastering, Environment, Magic Items, Glossary | Getting Started, Races, Classes, Skills, Feats, Equipment, Additional Rules, Combat, Magic, Spells, Prestige Classes, Gamemastering, Environment, Creating NPCs, Magic Items, appendix | Main 15-chapter sequence verified from the publisher PDF. Races, Prestige Classes, and Creating NPCs are not Rules children; nested placement and appendices remain incomplete |
| Ultimate Equipment | Arms And Armor, Gear, Magic Arms and Armor, Rings Rods & Staves, Wondrous Items, Artifacts and Other Items, Appendix | same | Unverified proposed match from memory |
| Monster Codex | 20 monster races A to Z, Appendix | same | Unverified proposed match from memory |
| Advanced Player's Guide | Races, Base Classes, Core Classes, Prestige Classes, Gear, Feats, Magic Items, Spell Lists, New Rules; roots Spells, Feats, Classes | Races, base classes, core classes, prestige classes, then equipment, feats, spells | Plausible; where Spells and Magic Items sit is unknown. Low confidence |
| Advanced Race Guide | Core Races, Featured Races, Uncommon Races, Race Builder, Age Height & Weight | probably the same | Unknown whether Age/Height/Weight is an appendix |
| Bestiary | Feats; Introduction, Monster Creation, Monster Advancement, Universal Monster Rules, Creature Types, Monster Roles, Encounter Tables, Misc Monster Rules; Monsters | Introduction, Monsters A to Z, then appendices | Appendix order unknown. Low confidence |
| Bestiary 2, 3, 4 | Monsters only | Introduction, Monsters, appendices | The dataset has only the monsters |
| Game Mastery Guide | Monsters; Planar Adventures, Settlements, Fast Play Ship Combat, Chases, Disasters, Drugs and Addiction, Haunts, Hazards, Sanity and Madness, NPCs | begins with running the game and campaign chapters | Does not match as far as I remember. Lowest confidence |
| Mythic Adventures | Spells, Feats, Monsters; Mythic Game, Mythic Heroes, Mythic Feats, Mythic Spells, Mythic Items, Mythic Monsters, Glossary | not remembered | Unknown |
| NPC Codex | Core Class NPCs, Prestige Class NPCs, NPC Class NPCs | not remembered | Unknown |
| Technology Guide | Technology in the world, Equipment, Hazards and Artifacts | not remembered | Unknown |
| Ultimate Campaign | Rules: Character Background, Downtime, Campaign Systems, Kingdoms And War; Feats | not remembered | Unknown; where the 28 feats sit is unknown |
| Ultimate Combat | Spells, Feats, Classes; Rules: Class Archetypes, Feats, Mastering Combat, Vehicles, Variant Rules, Spell Lists | New classes and archetypes come first | Unknown; the dataset has no `Classes` chapter among the Rules children |
| Ultimate Magic | Spells, Feats, Classes, Monsters; Rules: Spellcasting Class Options, Mastering Magic, Feats, Words of Power, Updates | not remembered | Unknown |

### How the book view is built

`sources/psrd-data/book-order.toml` is a curated chapter list per book: chapter titles in curated order, each mapped to upstream url prefixes (and optionally entry kinds or a file path prefix, to separate core, prestige, and NPC classes of one book, or the core, featured, and uncommon races of the Advanced Race Guide).

- **Curated, with evidence recorded per book:** the Core Rulebook's 15 main chapter titles and positions match the publisher contents PDF. Other lists remain based on upstream grouping and recollection. Each book states its `basis` and open questions in `review`; chapters with uncertain placement carry their own `review` note. Dataset order alone does not establish printed order.
- **From the data:** the order of sections inside a chapter (JSON document order, with the database comparison and exception noted above), the position of nested entries (a racial trait under its race, an archetype under the class section that holds it), and the order of typed records that no tree contains (`upstream-order.json`, extracted from the nested sets by `scripts/extract-upstream-order.py`: 4,653 records in 1,054 runs).
- **Result:** every one of the 9,967 entries sits in exactly one place; 0 are unplaced. 33 of them sit in the tree of another publication than their `source`: 32 Universal Monster Rules abilities (29 name Bestiary 2 and 3 name Bestiary 3 as `source`) that live in the Bestiary tree, and 1 Bestiary affliction at a Bestiary 2 url. The url decides.
- Where the database order disagrees with the printed book (for example the Core Rulebook race order), the book view repeats the database order. Fixing that needs a per-list override in `book-order.toml`, which does not exist yet.

## 7. What is not in the data

- Printed page numbers and printed chapter numbers: none in the JSON or the databases.
- Prerequisite structure for traits and for non-feat targets.
- Standard racial traits of the Advanced Race Guide as typed records.
- The Advanced Class Guide's classes: `Hunter`, `Arcanist`, and others appear in spell levels but their classes are not in the 16 books.
