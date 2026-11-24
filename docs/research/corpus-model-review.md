# Corpus model review

Reviewed 2026-11-24 against the pinned corpus v0.1.42: 31,735 entries, 76 MB, format 3. The
question: is the corpus modelled well, and where would a change pay off? The numbers below come from
reading every entry header. The format itself is specified in [corpus-format.md](../corpus-format.md).

## Verdict

The base model is sound and should stay: one file per entry, a permanent id that fixes the path,
typed fields beside a readable body, `[[sources]]` rows for print citations, and a check that fails
when a field and the body disagree. None of the findings below needs an id to change.

The weak points are of three sorts:

1. **Values that are spelled more than one way** in fields that should be closed sets. This is a data
   defect inside today's model, and cheap to fix.
2. **Kinds that carry statistics only as body text.** They cannot be filtered, and they cannot be
   compared with an outside dataset.
3. **Typed things filed under the catch-all kind `rule`**, and item statistics filed under free
   labels.

## What the corpus holds

| Kind | Entries | Typed fields | Print page missing |
|---|---|---|---|
| item | 8,097 | 7 keys; `price` 88%, `slot` 61% | 357 |
| rule | 6,026 | none | 906 |
| creature | 3,662 | 15 keys, on 86% of entries | 328 |
| feat | 3,657 | 6 keys | 77 |
| spell | 3,086 | 11 keys, on 99% | 68 |
| class_option | 2,819 | `group`, trades | 227 |
| trait | 1,937 | `category` | 16 |
| archetype | 1,299 | trades | 10 |
| affliction | 466 | none (the type is a tag) | 71 |
| racial_trait | 342 | `alternate` | 0 |
| prestige_class, class | 164 | `class_type`, `hit_die`, `skill_ranks` | 0 |
| race | 76 | none (`race_group` in the header) | 0 |
| trap | 60 | none | 7 |
| skill | 44 | none | 0 |

Headers are 24% of the corpus bytes. Provenance: 28,867 entries from the Pathfinder Data records,
1,900 from the PSRD carry-over (`entries/carried/`), 968 written since.

## Findings

### 1. Closed sets with several spellings

| Field | Finding | Entries |
|---|---|---|
| creature `size` | `Medium` beside `medium`; all in the carry-over | 325 |
| creature `alignment` | `N`, `CE` beside `neutral`, `chaotic evil` | 309 |
| class `hit_die` | `d8.` beside `d8` | 120 |
| spell `school` | 18 values hold a bracketed descriptor list or note: `evocation [fire or cold]` | 49 |
| spell `subschools` | 25 values lower-case, 1,074 capitalised | 25 |
| spell `descriptors` | `glamer`, an illusion subschool, listed as a descriptor | 82 |

`corpus-check` accepts all of these because it checks that a field matches the body, not that the
value belongs to a set. Ticket: [phynics/cast-iron-data#287](https://github.com/phynics/cast-iron-data/issues/287).
The durable fix is a closed-vocabulary check for these fields, as `slot` and `feat_types` already
have.

### 2. Statistics that exist only as body text

| Kind | Printed rows in the body | State |
|---|---|---|
| affliction (466) | Type, Save, Onset, Frequency, Effect, Cure | No typed field. 278 entries write them as bold paragraphs, not a stat block |
| trap (60) | Type, CR, Perception DC, Disable Device DC, Trigger, Reset, Effect | No typed field |
| creature, NPCs (487) | The full stat block | None of the 487 entries in `entries/npc/` has typed statistics; 27 carry-over creatures lack them too |
| item, weapons and armor (733) | Damage, critical, range, type, proficiency; armor bonus, check penalty | 43 entries hold weapon rows as `attributes`; the rest have body text only |

The creature gap needs no format change: the keys exist and the entries do not fill them. The
affliction and trap gaps need new typed keys. New optional keys are additive.

### 3. Free labels and packed strings

- **Item `attributes`** has 823 distinct labels on 310 entries (`Weapon: Dmg (M)`, `Common: Price`,
  `Construction Requirements: Cost`). It is a place to keep rows, not a model. The weapon and armor
  rows are a fixed printed table and deserve fixed keys.
- **Item `price`** is a string: 4,733 of 7,174 values are one amount (`8,000 gp`); the others are
  `+1 bonus`, `Varies`, or several amounts for sizes and grades. Nothing can sort or filter by
  price. A second, optional numeric key for the single-amount case would serve that without
  touching the printed text.
- **Creature `hp` and `ac`** are packed: `19~3d8+6`, `14/14/12`. Already listed in
  [#134](https://github.com/phynics/cast-iron/issues/134).
- **Spell `levels`** name a class by its title. All but two names (`Elementalist Wizard`,
  `Sahirafiyun`) match a class entry, so the check could require a match or a listed exception.

### 4. The kind `rule` holds typed things

5,873 of the 6,026 `rule` entries are prose, which is right. But the kind also holds families that
have their own printed form and their own lists in the app:

| Directory | Entries | What they are |
|---|---|---|
| `faith` | 463 | Deities |
| `template` | 170 | Creature templates |
| `umr` | 134 | Universal monster rules |
| `family` | 114 | Creature families |
| `subtype`, `type` | 99 | Creature subtypes and types |
| `spelldef` | 52 | Spell definitions |
| `haunt` | 45 | Haunts (now their own kind: [#618](https://github.com/phynics/cast-iron/issues/618), schema 7 in #662, data in corpus v0.1.43) |

The client finds these families by id prefix or by book position. A kind, or a typed `group` as
class options have, would state it in the data. The haunt work is the trial: if it goes well,
deities are the next family worth a kind, because they carry a regular printed block (alignment,
domains, favored weapon) that is filterable.

### 5. One value, two copies

Typed fields repeat in the body stat block for spells, creatures, items and feats. The drift check
keeps the copies equal, so this is a cost in size and in edit effort, not a correctness risk. It
does mean that an import error lands in both copies and passes the check: Major Image held another
spell's level and duration in both places
([phynics/cast-iron-data#282](https://github.com/phynics/cast-iron-data/issues/282)). Choosing one
owner per kind is open in [#134](https://github.com/phynics/cast-iron/issues/134). Only a
comparison with something outside the corpus catches this class of error; see
[cross-checking.md](../cross-checking.md).

### 6. Duplicates

283 carry-over entries share their kind and title with an entry outside the carry-over (226 of them
rules). Outside the carry-over, 693 groups share a kind and title; most are legitimate (a creature's
`Poison (Ex)`, a deed two classes share). The same-book pairs are in
[phynics/cast-iron-data#267](https://github.com/phynics/cast-iron-data/issues/267).

### What does not need to change

- **Directory names** (`monster`, `pclass`, `magic-wondrous`, `carried`) do not match kinds. They
  are part of the permanent ids and stay.
- **`section` and `parent`** are sparse because book order lives in `books/` and page order. That
  split works.
- **`provenance`** is frozen history and costs little.
- **A per-entry "verified" flag** was considered for print checks and is not proposed: the page in
  a `[[sources]]` row plus the ticket history already says what was checked, and a flag would go
  stale on the next edit.

## Proposals, in order of value for effort

The owner approved all seven on 2026-11-24, with 6 and 7 as a direction only
([decision](../decisions/2026-11-24-corpus-model-proposals-approved.md)).

| # | Change | Format change | Ticket |
|---|---|---|---|
| 1 | Normalise the six fields of finding 1 and add a closed-vocabulary check | None | [phynics/cast-iron-data#287](https://github.com/phynics/cast-iron-data/issues/287) |
| 2 | Fill creature fields for the 487 NPCs and 27 carry-over creatures | None | [phynics/cast-iron-data#289](https://github.com/phynics/cast-iron-data/issues/289), with the stat-block work in phynics/cast-iron-data#166 |
| 3 | Typed fields for afflictions and traps | Additive keys | [#670](https://github.com/phynics/cast-iron/issues/670) |
| 4 | Fixed keys for weapon and armor statistics, replacing those `attributes` rows | Additive keys, then a migration of 43 entries | [#671](https://github.com/phynics/cast-iron/issues/671), after 3 |
| 5 | Optional numeric price | Additive key | [#672](https://github.com/phynics/cast-iron/issues/672) |
| 6 | State the family of deities, templates and universal monster rules in the data | Schema version | [#673](https://github.com/phynics/cast-iron/issues/673): direction only, shape open |
| 7 | Structured `hp`, one owner for field and body | Format and schema version | [#134](https://github.com/phynics/cast-iron/issues/134): direction only |

Proposals 3 to 6 are in the `post-v1` milestone. Proposals 6 and 7 have no fixed design, order or
date; they go in one format batch when that work is planned, after the haunt kind
([#618](https://github.com/phynics/cast-iron/issues/618), #662) showed what a new kind costs. Main is
now corpus format 4 and schema 7; a further new kind still needs a schema bump (`AGENTS.md` rule 4).
