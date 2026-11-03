# PSRD-Data-release: data shape survey

Source: `/tmp/ci-dl-1/PSRD-Data-release` (read-only). All counts come from scripts in `/tmp/ci-survey-scripts/` (`inventory.py`, `detail.py`, `values.py`, `html_survey.py`, `links3.py`, `structure.py`, `examples.py`, `dirmap.py`, `verify.py`, `gen_tables.py`). Raw outputs are in `/tmp/ci-survey-scripts/out/`.

## 0. Inventory

- 25,342 `.json` files, **all parse** as UTF-8 JSON. Top-level value of every file is a **dict** (0 lists).
- 19 `.db` files (SQLite; not opened, per instruction): `index.db` (15.6 MB) and `book-{acg,apg,arg,b1,b2,b3,b4,cr,gmg,ma,mc,npc,ogl,tech,uc,ucampaign,ue,um}.db`. `book-ogl.db` (160 KB) is the only OGL-named artifact.
- Text files: `urllist.txt` (30,737 lines, 30,667 distinct), `supportedurllist.txt` (29,370 lines, 27,895 distinct). Not parsed further.

### 1. Files per book and subdirectory

Book dirs = 16. Typed dirs are `<book>/<type>/`; `*/rules/` holds section trees and **copies** of typed records (see §5).

| book | total | spell | feat | creature | item | class | rules | other subdirs |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| advanced_players_guide | 2453 | 267 | 166 | - | 256 | 15 | 1535 | affliction 4, class_archetype 81, racial_trait 50, trait 78, trap 1 |
| advanced_race_guide | 1128 | 92 | 184 | - | 166 | - | 423 | affliction 1, race 37, racial_trait 225 |
| bestiary | 726 | - | 13 | 357 | 8 | - | 254 | affliction 55, animal_companion 23, race 13, trap 3 |
| bestiary_2 | 414 | - | - | 330 | 9 | - | - | affliction 54, animal_companion 14, race 7 |
| bestiary_3 | 357 | - | - | 282 | 19 | - | - | affliction 51, race 5 |
| bestiary_4 | 288 | - | - | 263 | 15 | - | - | race 10 |
| core_rulebook | 3740 | 623 | 175 | - | 590 | 26 | 2124 | affliction 52, animal_companion 17, race 7, racial_trait 67, skill 26, trap 33 |
| game_mastery_guide | 411 | - | - | 81 | - | - | 299 | affliction 22, haunt 1, settlement 8 |
| monster_codex | 1535 | 24 | - | 207 | 50 | - | 1243 | animal_companion 4, trap 7 |
| mythic_adventures | 1475 | 9 | 162 | 47 | 104 | - | 884 | mythic_spell 269 |
| npc_codex | 902 | - | - | 310 | - | - | 592 | - |
| technology_guide | 570 | 16 | - | 3 | 127 | - | 417 | class_archetype 5, trap 2 |
| ultimate_campaign | 2520 | - | 28 | - | - | - | 1947 | army 45, drawback 17, kingdom_resource 70, resource 193, trait 220 |
| ultimate_combat | 2013 | 147 | 258 | - | 156 | 3 | 1350 | class_archetype 80, vehicle 19 |
| ultimate_equipment | 4670 | - | - | - | 2013 | - | 2655 | trap 2 |
| ultimate_magic | 2140 | 231 | 113 | 12 | 10 | 1 | 1698 | animal_companion 10, class_archetype 55, trap 10 |
| **total** | **25342** | | | | | | | |

- **Spells**: `<book>/spell/` (e.g. `core_rulebook/spell/discern_location.json`). Mythic spells: `mythic_adventures/mythic_spell/` (type `mythic_spell`, 269).
- **Feats**: `<book>/feat/`. **Items**: `<book>/item/`.
- **Creatures**: `bestiary/creature/`, `bestiary_2/creature/`, `bestiary_3/creature/`, `bestiary_4/creature/`, `monster_codex/creature/`, `npc_codex/creature/`, `game_mastery_guide/creature/`, `mythic_adventures/creature/`, `ultimate_magic/creature/`, `technology_guide/creature/`.
- **Classes**: `core_rulebook/class/{core,base,prestige,npc}/` (45 of 46 type=class records), `advanced_players_guide/class/`, `ultimate_combat/class/`, `ultimate_magic/class/`. Class archetypes: `<book>/class_archetype/`.
- **Rules sections**: `<book>/rules/**` (15,421 files total, nested directories).
- **Tables**: there is no table directory. All 605 `type: "table"` records are under `*/rules/`.

### 2. JSON tree and `type` values

Every record has a `type` (100%). 25 distinct values:

| type | files | type | files |
|---|---:|---|---:|
| section | 7801 | haunt | 2 |
| item | 6829 | kingdom_resource | 140 |
| creature | 2411 | trap | 113 |
| ability | 2035 | animal_companion | 96 |
| spell | 1454 | army | 90 |
| feat | 1107 | race | 79 |
| table | 605 | class | 46 |
| trait | 596 | vehicle | 38 |
| class_archetype | 442 | drawback | 34 |
| racial_trait | 392 | skill | 26 |
| resource | 386 | list | 21 |
| affliction | 313 | settlement | 16 |
| mythic_spell | 269 | embed | 1 |

No `type: "ogl"` value exists.

- **Subtypes**: present on some records, e.g. `creature/npc` 698, `item/ring` 113, `item/neck` 77, `affliction/poison` 199, `resource/room` 156. Full list in `out/inventory.json` (`subtype_counts`). Subtype presence is 100% for some types (racial_trait, class) and 0.5% for spell.
- **Sections nesting**: `sections` is a list of **child records**, each with its own `type`, `source`, `name`, `url`, and optionally `body` and its own `sections`. Child types: section 22,111; item 2,974; ability 2,437; table 820; creature 520; trait 298; class_archetype 224; resource 193; animal_companion 133; affliction 76.
- Files by number of nested section levels (count of `sections`-of-`sections` levels per file): 0: 16,354; 1: 5,875; 2: 2,088; 3: 743; 4: 195; 5: 75; 6: 9; 7: 3.
- **Dynamic keys**: `item.misc` is `{<sub-heading>: {<label>: <value>}}` with labels such as `Cost`, `Price`, `Gear Type`, `Construction Requirements`. Creature `spells` is a list of single-key dicts with free-text labels (e.g. `"spell-like abilities"`, `"oracle spells known"`). 1,027 distinct key names occur at some depth.

### 3. Field presence for target types

Percentages are of records of that `type` (top-level keys only). Example values are literal JSON.

#### spell (n=1454)

| field | % | kind | examples |
|---|---:|---|---|
| school | 100 | str | `"transmutation"`, `"abjuration"` (9 distinct, incl. `"universal"` 6) |
| name | 100 | str | |
| url | 100 | str | `"pfsrd://Core Rulebook/Spells/Discern Location"` |
| source | 100 | str | `"Core Rulebook"` |
| type | 100 | str | `"spell"` |
| levels | 100 | list | `[{"class": "Arcanist", "level": 8}, {"class": "Cleric", "level": 8}]` |
| level_text | 100 | str | `"Arcanist: 8; Cleric: 8; Oracle: 8; ..."` (class-name case varies: `"druid: 6; hunter: 6; ..."` in technology_guide) |
| body | 99.8 | str (HTML) | `"<p>A <i>discern location</i> spell ..."` |
| effects | 96.7 | list | `[{"text": "10-ft.-radius emanation centered on you", "name": "Target"}]` |
| components | 96.6 | list | `[{"type": "V"}, {"type": "S"}, {"text": "pinch of rust", "type": "M"}]` |
| duration | 95.8 | str | `"1 minute/level (D)"`, `"instantaneous"` |
| component_text | 95.8 | str | `"V, S, M/DF (pinch of rust)"` |
| range | 95.0 | str | `"10 ft."`, `"close (25 ft. + 5 ft./2 levels)"`, `"unlimited"` |
| casting_time | 94.8 | str | `"1 standard action"`, `"10 minutes"` |
| description | 94.4 | str (plain) | `"Reveals exact location of creature or object."` |
| saving_throw | 81.9 | str | `"none"` (320), `"Will negates (harmless)"` (161), `"Fortitude partial (see text)"` (118 distinct) |
| spell_resistance | 81.5 | str | `"yes"` (504), `"no"` (340), `"yes (harmless)"` (192), `"see text"` (10), `"no; see text"` (5) (37 distinct) |
| descriptor_text | 43.2 | str | `"acid"`, `"mind-affecting"` |
| descriptor | 41.3 | list | `["acid"]`, `["emotion", "mind-affecting"]` (90 distinct sets) |
| subschool_text | 34.9 | str | |
| subschool | 34.3 | list | `["compulsion"]`, `["creation"]` (23 distinct) |
| as_spell_id | 12.6 | int | `27`, `36`, `38` (meaning not verified) |
| sections | 5.0 | list | |
| subtype | 0.5 | str | `"mythic_spell"` (7) |

- **There is no `level` field on spells.** Level lives in `levels` (list) and `level_text` (string).
- `school`, `saving_throw` and `spell_resistance` are free-text strings; there is no enum.

#### feat (n=1107)

| field | % | kind | examples |
|---|---:|---|---|
| name | 100 | str | `"Acrobatic"` |
| url | 100 | str | `"pfsrd://Core Rulebook/Feats/Acrobatic"` |
| source | 100 | str | `"Core Rulebook"` |
| type | 100 | str | `"feat"` |
| sections | 100 | list | `[{"body": "<p class=\"stat-block-1\"> You get a +2 bonus ...", ...}]` |
| feat_types | 100 | dict | `{"feat_type": "General"}` (13 values: General 401, Combat 372, Mythic 159, Metamagic 33, ...) |
| description | 81.9 | str (plain) | `"You are skilled at leaping, jumping, and flying."` |
| body | 18.1 | str (HTML) | `"<p>Despite casting spells, you dabble in the art of wordcasting.</p>"` |

Top-level `body` is present on 18.1% of feats and `sections` on 100%. Where the text sits within `sections` was not checked further.

#### creature (n=2411)

Creature stat fields are flat strings. There are no numeric fields for `cr`, `hp`, `ac`, etc.

Key examples:
- `cr`: `"1/2"` (110), `"3"` (191), `"12"` (102). Values: 37 distinct. Non-matching value: `"&mdash"` (18 files, all under `monster_codex/`, missing semicolon).
- `hp`: `"11 (1d8+3)"`, `"139 (18d10+40)"`. 2,257 of 2,374 match `N (dice)`; others are `"108 (11d8+55); fast healing 5"` or `"51 each (6d8+24)"`.
- `ac`: `"15, touch 10, flat-footed 15 (+5 armor)"`. All 2,374 start with an integer.
- `init`: `"+0"`, `"+11"`. `speed`: `"30 ft. (20 ft. in armor)"`, `"15 ft., swim 30 ft."`.
- `xp`: `"200"`, `"19,200"` (comma-formatted). `base_attack`: `"+0"`. `cmb`, `cmd`: `"-1"`, `"9"`.
- `size`: `"Medium"` (1229), `"Large"`, ... (10 distinct). `alignment`: `"N"`, `"CE"` ... (16 distinct).
- `creature_type`: `"Humanoid"` (1092), `"Outsider"`, ... (16 distinct). `creature_subtype`: `"human"`, `"human, orc"` (184 distinct).
- `environment`: `"any land"`, `"any underground"` (351 distinct). `organization`, `treasure` (`"standard"`, `"none"`).
- `sr`: `"24"`. `dr`: `"10/-"`, `"10/magic and silver"`. `resist`: `"acid 5, cold 5, electricity 5"`.
- `level`: `"barbarian 3"`, `"11"`. `hit_dice`: `"4d12"`. `sex`: `"male"`.
- `spells`: list, e.g. `[{"oracle spells known": "(CL 10th; concentration +14)<p class=\"stat-block-2\">5th (3/day)..."}]`.

Presence (%):

`source` 100, `type` 100, `name` 100, `url` 100, `intelligence` 99.7, `wisdom` 99.6, `charisma` 99.5, `dexterity` 99.5, `speed` 99.5, `strength` 99.5, `constitution` 99.5, `size` 99.4, `cr` 99.3, `fortitude` 98.6, `init` 98.6, `reflex` 98.5, `will` 98.5, `alignment` 98.5, `creature_type` 98.5, `ac` 98.5, `hp` 98.5, `base_attack` 98.4, `senses` 98.3, `cmd` 98.3, `cmb` 98.2, `xp` 98.1, `melee` 98.1, `skills` 96.1, `feats` 93.9, `sections` 90.7, `languages` 79.1, `special_attacks` 78.3, `creature_subtype` 70.1, `special_qualities` 67.2, `environment` 49.6, `organization` 49.6, `treasure` 49.6, `immune` 47.5, `spells` 46.9, `super_race` 45.6, `ranged` 45.1, `description` 42.2, `other_gear` 40.6, `combat_gear` 40.3, `defensive_abilities` 39.1, `space` 37.7, `reach` 37.7, `subtype` 32.1, `dr` 27.6, `racial_modifiers` 23.9, `resist` 18.0, `sr` 16.3, `weaknesses` 15.6, `aura` 12.3, `boon` 3.4, `gear` 3.3, `level` 2.7, `mr` 2.7, `breath_weapon` 1.0, `natural_armor` 1.0, `hit_dice` 1.0, `sex` 0.7, `abbrev` 0.2, `body` 0.1.

All creature fields are `str` except `sections` (list), `spells` (list). Presence mostly ~98% for core stat-block fields, and below 50% for the rest.

#### item (n=6829)

| field | % | kind | examples |
|---|---:|---|---|
| name | 100 | str | `"Acid"` |
| url | 100 | str | `"pfsrd://Core Rulebook/Rules/Equipment/Goods And Services/Special Substances and Items/Acid"` |
| source | 100 | str | `"Core Rulebook"` |
| type | 100 | str | `"item"` |
| weight | 94.1 | str | `"1 lb."`, `"2 lbs."`, `"-"` |
| misc | 91.1 | dict | `{"Arc Rifle": {"Capacity": "20", "Dmg (S)": "1d10 elec.", ...}}`; `{"null": {"Gear Type": "Special Substances and Items", "Craft": "DC 15 Craft (Alchemy) Check"}}` |
| price | 90.9 | str | `"10 gp/flask"`, `"10,000 gp"`, `"varies"` |
| body | 81.5 | str (HTML) | `"<p class=\"stat-block\">An arc pistol emits bolts ..."` |
| cl | 56.7 | str | `"11th"`, `"15th"` |
| aura | 56.0 | str | `"moderate abjuration"`, `"strong evocation"` (135 distinct) |
| sections | 24.6 | list | `[{"body": "<table><thead><tr><th>Construction</th>..."}]` |
| slot | 24.5 | str | `"ring"`, `"neck"`, `"none"` (906), `"None"` (62). Case inconsistency. |
| subtype | 10.8 | str | `"ring"` (113), `"feet"`, `"body"` |
| abbrev | 0.5 | str | `"sq. yd."`, `"per day"` |

### 4. Body text (HTML)

- `body`: 94,658 string values, of which 91,609 contain HTML tags (96.8%). `description`: 4,315, of which 40 contain tags. `text`: 2,475 strings (mostly spell `effects[].text` and `components[].text`), none contain tags.
- **Entities are not decoded** and appear raw.
  - Named (19 distinct): `&mdash;` 28,969; `&ndash;` 26,402; `&times;` 4,429; `&nbsp;` 2,713; `&ldquo;` 1,504; `&rdquo;` 1,495; `&Dagger;` 334; `&dagger;` 283; `&gt;` 123; `&lt;` 105; `&quot;` 40; `&bull;` 35; `&hellip;` 13; `&amp;` 11; `&shy;` 9; `&deg;` 4; `&ndash1;` 3 (invalid, not a real entity).
  - Numeric: `&#173;` 12, `&#160;` 5.
- **Tags (count)**: td 189,183; p 123,241; i 67,648; tr 58,177; b 22,801; th 14,616; li 5,977; thead 4,055; sup 3,738; table 3,315; span 2,728; caption 2,140; a 2,033; ul 1,250; tbody 812; tfoot 615; blockquote 130; wbr 63; div 47; img 26; h2 25; big 18; h1 13; ol 6; center 6; `sup3` 4; h3 4.
- **Attributes**: `a`: href only (2,033). `p`: class 43,202, id 26,020, style 987, align 13. `td`: style 9,071, align 2,506, colspan 1,442, class 880. `th`: colspan 444, rowspan 394. `table`: id 1,819, class 20, style 13. `span`: class 2,583, style 145, xml:lang 20. `img`: src 26. `h1`/`h2`/`h3`: id. One odd attribute name `class-` (1 occurrence).
- **Common `class` values**: `odd`/`even` (table rows), `stat-block-1`, `stat-block`, `stat-block-2`, `stat-block-rp`, `stat-block-breaker`, `indent`.
- **Malformed HTML** (observed with Python `html.parser`, not a validator): unclosed `<b>` 114; stray end tags for `a`, `p`, `b` (about 28 total); unclosed `<a>` 3; unclosed `<td>` 1.
- **Tables**: 1,748 files contain `<table>`; 3,315 `<table>` tags. Typical shape: `<table><caption>Technomancer</caption><thead><tr><th>Level</th>...</thead><tbody><tr class="odd"><td>...`. 77.7% of tables (2,568 of 3,304, regex count) have no `<tbody>`. `colspan`/`rowspan` used on `td`/`th`.
- **Images**: 26 `<img src="/pathfinderRPG/prd/image/AttacksOfOpportunity.jpg">` (absolute site-root paths) in 22 files, all under `*/rules/` (e.g. `core_rulebook/rules/combat.json`). The image files are not in the dataset.

**Internal links** (`<a href>`): 2,033 total, all in `body`, all with only `href` as an attribute.

Five literal examples (raw tags from `core_rulebook/spell/summon_monster_ii.json` and others):
1. `<a href="pfsrd://Bestiary/Monsters/Rat/Dire">`
2. `<a href="pfsrd://Bestiary/Monsters/Dog">`
3. `<a href="pfsrd://Bestiary/Monsters/Dolphin/Dolphin">`
4. `<a href="pfsrd://Bestiary/Monsters/Eagle/Eagle">`
5. `<a href="pfsrd://Bestiary/Monsters/Beetle/Fire">`

Other shapes: 23 have an empty `href=""`. 566 contain spaces (not percent-encoded). One contains an apostrophe: `pfsrd://Bestiary/Monsters/Will-o'-Wisp`. None contains `&`, `#` or `?`.

**Resolution** (href value compared to every `url` in the dataset, including nested section records; 26,836 distinct url values):

| method | full population (2,033) | sample of 500 (`random.Random(42)`) |
|---|---:|---:|
| exact match to a `url` | 1,841 (90.6%) | 455 (91.0%) |
| prefix match (href is a parent of a deeper url, e.g. `Bestiary/Monsters/Dog` to `Bestiary/Monsters/Dog/Dog/Dog`) | 137 | 33 |
| empty href | 23 | 5 |
| unresolved (e.g. `pfsrd://Bestiary/Monsters/H`) | 32 | 7 |

- **Resolve rate**: 97.6% of the sample (488/500), 97.3% of the full population (1,978/2,033). Exact-only: 91.0% of the sample.
- **href to file**: no file path is encoded. Resolution is via a url index. A file path cannot be derived from the url by a simple transform (not tested further).

### 5. Identity

- `url`: present on **100%** of top-level records. Format `pfsrd://<Book Title>/<Section>/.../<Name>` (book titles like `Core Rulebook`, not dir names).
- **Not unique.** 25,342 files, 20,397 distinct top-level `url` values (26,836 if nested `sections` records are included).
  - 4,944 duplicate groups covering 9,889 files (one group has 3 files).
  - Every group is within one book.
  - 4,943 groups are byte-identical after canonical JSON. 1 group differs: `pfsrd://Monster Codex/Rules/Drow/Example Drow/Abrakarn Viper/Abrakarn Viper` in `monster_codex/creature/abrakarn_viper.json`, `monster_codex/item/abrakarn_viper.json`, `monster_codex/rules/drow/example_drow/abrakarn_viper/abrakarn_viper.json`.
  - Example pair: `technology_guide/spell/antitech_field.json` and `technology_guide/rules/technology_in_the_world/spells/antitech_field.json` both have `url` `pfsrd://Technology Guide/Rules/Technology in the world/Spells/Antitech Field`.
  - **Pattern**: each duplicate group has one copy under `*/rules/` (4,944 of 4,944 groups). Among non-rules files, `url` is unique for every type (0 duplicates).
  - Stable key: `url` on non-rules files. Plan for deduplication (e.g., skip `*/rules/` copies of typed records).
- `name` is not unique: 5,223 (type, name) groups have more than one record.
- **`source`**: present on 100% of records. 17 distinct values:

| source | count | source | count |
|---|---:|---|---:|
| Ultimate Equipment | 4670 | Bestiary | 679 |
| Core Rulebook | 3738 | Technology Guide | 567 |
| Ultimate Campaign | 2516 | Bestiary 2 | 457 |
| Advanced Player's Guide | 2451 | Game Mastery Guide | 410 |
| Ultimate Magic | 2137 | Bestiary 3 | 360 |
| Ultimate Combat | 2010 | Bestiary 4 | 288 |
| Monster Codex | 1535 | PFSRD | 21 |
| Mythic Adventures | 1473 | | |
| Advanced Race Guide | 1128 | | |
| NPC Codex | 902 | | |

`PFSRD` appears on 21 files, e.g. `technology_guide/rules/equipment.json`, `ultimate_campaign/rules/campaign_systems.json`.

### 6. Difficult files

Found by `examples.py` and `structure.py`.

- **Big tables** (by `<tr>` count):
  1. `ultimate_equipment/rules/appendix.json`: 262 KB, 2,083 `<tr>`, 50 `<table>`.
  2. `core_rulebook/rules/magic_items.json`: 1.0 MB, 1,161 `<tr>`, 62 `<table>`.
  3. `ultimate_campaign/rules/character_background.json`: 499 KB, 594 `<tr>`, 59 `<table>`.
- **Stat blocks**:
  1. `bestiary/creature/aasimar.json`: 42 keys; 4 `stat-block` strings in the file.
  2. `monster_codex/creature/abrakarn_viper.json`: 40 keys, 3 sections, flat fields.
  3. `monster_codex/creature/ancient_gravedigger.json`: 40 keys, 12 `stat-block` strings in the file.
  - 8,752 files contain a `stat-block` class (item 4,709, section 1,525, feat 818, creature 1,517, and others).
- **Deep nesting**: max JSON nesting 15 nodes, in 3 files:
  1. `ultimate_magic/rules/spellcasting_class_options.json`
  2. `monster_codex/rules/vampires.json`
  3. `monster_codex/rules/goblins.json` (`$.sections[].sections[]...`, 7 section levels)
- **Malformed JSON**: **none**. 0 files fail strict parse; 0 have a BOM; 0 contain duplicate keys in an object; 0 contain `NaN`/`Infinity`.

### 7. OGL and Product Identity markers

- No `ogl` key, no `type: "ogl"`, and no key containing `ogl`, `license` or `identity`, at any depth.
- Text search (all string values) for `Open Game License`, `Section 15`, `Product Identity`, `OGL` (as a word) and `Copyright Notice`: **0 matches** each.
- **OGL section 15 records are not stored in the JSON at all**, as far as the text search shows. The only OGL-named artifact is `book-ogl.db` (159,744 bytes). I did not open it (per instruction), so how section 15 is stored there is **unverified**.

## Helper scripts

All in `/tmp/ci-survey-scripts/` (run as `python3 -I <script> /tmp/ci-dl-1/PSRD-Data-release`): `inventory.py` (types, parse errors, depth), `detail.py` (field presence, url/source identity), `values.py` (value samples), `html_survey.py` (tags, attrs, entities), `links3.py` (href resolution), `structure.py` (sections depth, markers), `examples.py` (tables, stat blocks, deep files), `dirmap.py` (dir to type), `verify.py` (dup placement, value shapes), `gen_tables.py` (§1 table).

## Not verified

- Contents of the 19 `.db` files, including `book-ogl.db` (not opened per instruction). The OGL section 15 storage question is therefore only partly answered.
- Meaning of `as_spell_id` on spells. Only its type (int) and presence (12.6%) were checked.
- Whether a url-to-file-path slug rule exists (not tested).
- HTML counts come from Python `html.parser`, not a validator.
- `urllist.txt` and `supportedurllist.txt` were only checked as sets against the JSON `url` values (87.5% and 91.7% of distinct entries are present). Their purpose is unknown.
