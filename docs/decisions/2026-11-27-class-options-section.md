---
title: "Class pages: an Options section for selectable abilities"
date: 2026-11-27
status: current
superseded_by: []
issues: [540, 739]
prs: [742]
area: ui
---

# Class pages: an Options section for selectable abilities, 2026-11-27

Refs [#739](https://github.com/phynics/cast-iron/issues/739) (part 2) and [#540](https://github.com/phynics/cast-iron/issues/540). The owner chose, on 2026-11-27, how a class page offers the abilities that a class selects from (rogue talents, domains, hexes, rage powers): **a section button at the top of the page, linked from the class feature that grants the choice**. The owner passed over two other shapes: the lists inline in the Features text (a Rogue page would grow by 234 rows, and the Wizard's 28 discoveries and 18 schools would sit between the features that name them) and one page per group (a second page to reach, with the class page's switcher, filters and Back to rebuild). The look is in DESIGN.md, "Options section".

- **A stop of the switcher, after Features and before Archetypes.** `Section::Options` (`?s=options`) is a stop of a base class that has at least one option group in `option-groups.json` whose `classes` lists it, and the page draws the section itself, because the body has no heading for it. It counts as a stop towards the three that give a page a switcher. A prestige class has none: Hellknight is the one prestige class with groups, and its page does not load them. The existing `Class options` panel at the end of the page stays: it opens the whole-library list page of a group, with that page's sort and facets.
- **No new data.** The groups come from `option-groups.json`, the books from the active-book snapshot and the catalog, and a group's options from the one or two shards that hold them, read when the group is shown and kept for the page's life (`Library::read_entries`). No schema, index or compiled file changed. The shards are the ones that open when the reader opens an option, so the offline cache already has them.
- **What a row can say.** A class option's typed `fields` hold the group and nothing else, so a row reads three facts from the printed body, and shows none that the text does not print:
  - the **ability tag**, when the first line of the body is the option's own name (`Accurate Poisoner (Ex)`; the first heading of an option that prints several powers is a power, not the option, and lends no tag);
  - the **level** of a `Level:` row (278 Kineticist wild talents print one, no other group does, so only that group has a level at the right of its rows and a level menu; the menu needs two or more distinct levels, because a single uniform level is nothing to filter);
  - the **prerequisite** of a `Prereq:` or `Prerequisite:` row (961 options). One of 24 characters or fewer sits at the right of the row (`9th-level magus`), a longer one on the muted line.

  Nothing is inferred: an option with no `Level:` row has no level, so the level filter of the issue exists only where the data prints a level. A typed level, prerequisite and tag for every option is a corpus change; it follows #540's conversion of class options to format 4, when a feature head carries the tag and the level.
- **Route state.** `?s=options&group=<slug>&find=<text>&source=<book id>&level=<n>`, beside `a`, `book`, `keep` and `with`. They are `OptionChoice`, a field of the route's `ArchFilter` (which now holds the choices of both lists of a class page). A chip is one history entry that resets the filters, typing replaces the current entry, and the page is not rebuilt for either. `Route::only_options_differ` is true when the next address is at the Options stop with other options choices on the same entry, whatever stop the reader was at, so the frame leaves scroll and focus alone for a chip, a menu choice or typed text (the reader scrolled to the chips by hand). The link row scrolls to the section itself; a stop of the switcher keeps the choices and so is a section jump; Back from the section is no choice and returns to the offset the reader left. A group slug that the class lacks shows the class's first group, and the address keeps what it says.
- **The link row, and how a feature is matched to a group.** The renderer takes views that the page draws before given blocks of the body (`RichBody`'s `inserts`), so neither the section nor the rows change the body, the corpus or the heading keys. The matching rule is `option_list::option_links`:
  1. Candidates are the headings, and the paragraphs that open with a bold name, inside the Features section: from the first top-level `Class Features…` heading to the heading that starts `Alternate Capstones`, `Archetypes` or the favored class bonuses. (A caster prints `Spells Known` at the top level before its deeper features, so the end is these named sections, not the next top-level heading.)
  2. A candidate's name is its printed text in lower case, with the ability tag (`(Ex)`, `(Ex or Su)`), a trailing colon and any possessive `'s` removed.
  3. A group answers to its `label` and its `singular`, each also without the leading words that only name the class (the words that are also words of the class title): `Witch hexes` and `Witch hex` are also `hexes` and `hex` on the Witch page, `Cleric domains` is also `domains`, and `Oracle's Curse (Ex)` is `oracle curse`. A name that does not start with a word of the class title keeps its whole name, so `Arcane school` is not matched by a bare `School`.
  4. The first heading that matches a group wins; a bold name is read only when no heading matches. A heading's row goes after all the text under it, before the next heading of the same or a higher level; a bold name's row goes right after its paragraph.
  5. A group that no candidate names has no row, and its chip works.

  The rule reads printed names only, so it is the same on the imported pages (bold names and plain headings; every base class is imported today) and on converted format 4 pages (headings that list `features`): both are the same heading text. Ranger combat styles are feat lists inside the class feature, not an option group (owner), so they get no chip.
- **Coverage on the pinned corpus** (v0.1.62, `2ff82b653`): 54 groups of 39 base classes, 42 with a row and 12 with a chip only. The headings are those of the compiled library.

  | Class | Option group (options) | Feature that grants the choice |
  |---|---|---|
  | Alchemist | Alchemist discoveries (168) | Discovery (Su) (heading) |
  | Arcanist | Arcanist exploits (73) | Arcanist Exploits (heading) |
  | Barbarian | Rage powers (240) | Rage Powers (Ex) (heading) |
  | Bard | Bardic masterpieces (68) | Bardic Masterpieces: (bold name) |
  | Bard | Bardic performances (13) | Bardic Performance (heading) |
  | Bloodrager | Bloodrager bloodlines (24) | Bloodline (heading) |
  | Bloodrager | Rage powers (240) | no match (chip only) |
  | Cavalier | Orders (38) | Order (Ex): (heading) |
  | Cleric | Cleric domains (35) | Domains (heading) |
  | Druid | Animal companions (213) | no match (chip only) |
  | Druid | Druid domains (25) | Domain (heading) |
  | Gunslinger | Gunslinger deeds (17) | Deeds (heading) |
  | Inquisitor | Inquisitions (39) | no match (chip only) |
  | Investigator | Alchemist discoveries (168) | no match (chip only) |
  | Investigator | Investigator talents (67) | Investigator Talent (Ex or Su) (heading) |
  | Kineticist | Kineticist wild talents (278) | Wild Talents (heading) |
  | Magus | Magus arcana (64) | Magus Arcana (heading) |
  | Medium | Medium spirits (40) | Spirit (Su) (heading) |
  | Mesmerist | Mesmerist stares (24) | no match (chip only) |
  | Mesmerist | Mesmerist tricks (44) | Mesmerist Tricks (Su) (heading) |
  | Ninja | Ninja tricks (65) | Ninja Tricks (heading) |
  | Occultist | Occultist implements (12) | Implements (Su) (heading) |
  | Oracle | Oracle curses (41) | Oracle's Curse (Ex) (heading) |
  | Oracle | Oracle mysteries (34) | Mystery (heading) |
  | Paladin | Paladin mercies (25) | Mercy (Su) (heading) |
  | Paladin | Paladin oaths (15) | Oaths: (bold name) |
  | Psychic | Phrenic amplifications (31) | Phrenic Amplifications (heading) |
  | Psychic | Psychic disciplines (23) | Psychic Discipline (Ex or Sp) (heading) |
  | Ranger | Ranger traps (41) | no match (chip only) |
  | Rogue | Rogue talents (234) | Rogue Talents (heading) |
  | Samurai | Orders (38) | Order (Ex) (heading) |
  | Shaman | Shaman hexes (17) | Hex (heading) |
  | Shaman | Shaman spirits (18) | Spirit (Su) (heading) |
  | Shifter | Shifter aspects (30) | Shifter Aspect (Su) (heading) |
  | Skald | Bardic masterpieces (68) | no match (chip only) |
  | Skald | Rage powers (240) | Rage Powers (Ex) (heading) |
  | Slayer | Slayer talents (43) | Slayer Talents (heading) |
  | Sorcerer | Sorcerer bloodlines (51) | Bloodline (heading) |
  | Spiritualist | Emotional foci (15) | no match (chip only) |
  | Summoner | Summoner evolutions (76) | no match (chip only) |
  | Swashbuckler | Swashbuckler deeds (18) | Deeds (heading) |
  | Unchained Barbarian | Rage powers (240) | Rage Powers (Ex) (heading) |
  | Unchained Monk | Ki powers (44) | Ki Powers (Su) (heading) |
  | Unchained Monk | Style strikes (15) | Style Strike (Ex) (heading) |
  | Unchained Rogue | Rogue talents (234) | Rogue Talents (heading) |
  | Unchained Summoner | Eidolon subtypes (26) | no match (chip only) |
  | Unchained Summoner | Unchained summoner evolutions (61) | no match (chip only) |
  | Vigilante | Social talents (46) | Social Talent (heading) |
  | Vigilante | Vigilante talents (80) | Vigilante Talent (heading) |
  | Warpriest | Warpriest blessings (42) | Blessings (Su) (heading) |
  | Witch | Witch hexes (104) | Hex (heading) |
  | Witch | Witch patrons (61) | no match (chip only) |
  | Wizard | Arcane discoveries (28) | Arcane Discoveries: (bold name) |
  | Wizard | Arcane schools (18) | Arcane School (heading) |

  The 12 chip-only groups are named in no heading of their class: the Druid's animal companion is under `Companion`, the Inquisitor's inquisition inside `Domain`, the Summoner's evolutions inside `Eidolon`, the Mesmerist's stares inside `Hypnotic Stare (Su)`, the Spiritualist's emotional focus inside `Phantom`, the Witch's patron inside `Hex`, and the Skald's masterpieces, the Bloodrager's rage powers and the Investigator's alchemist discoveries are not printed under a name of their own. A looser rule (a shared word) would link the wrong feature as often as the right one: a miss costs a row, a clash a wrong link, so the rule stays strict. A later change to a class page that names the group in a heading adds the row without code.
- **Checked.** `class-options-section.spec.ts` (Rogue: the stop, the chip, the count, the rows, the tag, the name field, the book menu and a row's page; the link row; the Wizard's two groups, the address and Back; Fighter without the stop; 390 px) and the native tests of `option_list.rs` (the matching rule, the facts of a row, the filters and their counts) and `route.rs` (the address, and which moves leave the frame alone). The stops of `class-sections.spec.ts` gained Options for the Wizard and the Arcanist.
