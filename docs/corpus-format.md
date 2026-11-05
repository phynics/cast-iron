# Corpus format

The corpus is a plain-text form of the compiled rules library. One file holds
each entry, and four small files hold the reviewed source data that the entries
cannot derive. A human can edit an entry without touching Rust, and a compiler
can rebuild the library from the corpus.

This document is the format specification. `crates/corpus` is the executable
definition: `write_entry` and `read_entry` must agree for every value the schema
can hold.

## Layout

```text
entries/<id without ref/>.md    one entry per file
publications.toml               publication metadata used by the catalogue
books.toml                      curated chapter order (BookIndex)
legal.toml                      licences, Product Identity, Section 15, dataset pins
aliases.toml                    legacy-to-canonical map (optional, when the library has one)
```

The path is derived from the permanent id. `ref/spell/fog-cloud` is
`entries/spell/fog-cloud.md`; `ref/carried/core-rulebook/rules/glossary/special-abilities`
is `entries/carried/core-rulebook/rules/glossary/special-abilities.md`. Every
published id starts with `ref/`, so the rule is reversible: remove `entries/`
and `.md`, then prepend `ref/`. The file also stores `id` in the front matter, and the
reader rejects a file whose path does not match it.

Ids never change. Moving a file, or editing an id, is a data error. An upstream
id rename gets a row in `sources/psrd-data/id-overrides.toml` and never a second
meaning; the corpus does not invent a new path for an existing id.

## Entry front matter

The file starts with `+++`, then TOML, then a closing `+++` line. Keys appear in
schema order. Empty keys are omitted. `links` is never written; the reader
recomputes it from the body.

```text
+++
id = "ref/spell/fog-cloud"
kind = "spell"
title = "Fog Cloud"
source_ids = ["core-rulebook", "mythic-adventures", "potions-and-poisons"]
section = ["Spells", "Spell Descriptions"]
parent = "ref/spell"
tags = ["fog"]
provenance = [
  { dataset = "pathfinder-data-1-e", record = "fog_cloud", path = "json/spells4.json" },
]
license_refs = ["ogl-1.0a", "legal#community-use"]
relations = [
  { kind = "part", target = "ref/race/dwarf", label = "alternate" },
]

[fields]
kind = "spell"
school = "conjuration"
subschools = ["Creation"]
levels = [
  { class = "Arcanist", level = 2 },
  { class = "Druid", level = 2 },
]
casting_time = "1 standard action"
components = "V, S"
range = "medium (100 ft. + 10 ft./level)"
effects = [
  { label = "Effect", value = "fog spreads in 20-ft. radius" },
]
duration = "10 minutes/level"
saving_throw = "none"
spell_resistance = "no"
+++
```

The example is illustrative: it shows every header key, while the real Fog
Cloud entry (Core Rulebook) omits the keys it does not have and carries every
class level and the full body below.

### Header keys

| Key | Type | Rule |
|---|---|---|
| `id` | string | Permanent `ref/` id. Required. |
| `kind` | string | `EntryKind` slug: `rule`, `spell`, `feat`, `creature`, `item`, `class`, `prestige_class`, `race`, `skill`, `affliction`, `trap`, `trait`, `racial_trait`, `archetype`. Required. |
| `title` | string | Display title. Required. |
| `source_ids` | array of strings | Publication ids. Every one must exist in `publications.toml`. |
| `section` | array of strings | Breadcrumb inside the rules hierarchy. |
| `parent` | string | Containing rules entry; must resolve to another entry. Omitted when absent. |
| `tags` | array of strings | Stable labels such as `standard_race`. |
| `provenance` | array of tables | `dataset`, `record`, `path`. At least one source record normally. |
| `license_refs` | array of strings | A licence id from `legal.toml`, or `legal#community-use` when the Community Use notice applies. |
| `relations` | array of tables | `kind` (`part`, `archetype`, `prerequisite`), `target`, and optional `label` (`or`, `alternate`). |

### Typed fields

`[fields]` always has `kind`. It adds only the keys of that kind, in schema
order:

| `kind` | Keys |
|---|---|
| `spell` | `school` (required), `subschools`, `descriptors`, `levels`, `casting_time`, `components`, `range`, `effects`, `duration`, `saving_throw`, `spell_resistance` |
| `feat` | `feat_types`, `prerequisites`, `prerequisite_feats`, `benefit`, `normal`, `special` |
| `creature` | `cr`, `creature_type`, `subtypes`, `size`, `alignment`, `ac`, `hp`, `fortitude`, `reflex`, `will`, `melee`, `ranged`, `special_attacks`, `special_qualities`, `abilities` |
| `item` | `category`, `price`, `weight`, `slot`, `aura`, `caster_level`, `attributes` |
| `class` | `class_type`, `hit_die`, `skill_ranks`, `class_skills` |
| `trait` | `category` (required), `detail` |
| `racial_trait` | `alternate` (required boolean) |
| `none` | none |

`levels`, `effects`, and `attributes` are arrays of inline tables. `cr` is the
printed fraction (`1/2`, `1/8`) or a whole number. `abilities` is six strings in
STR, DEX, CON, INT, WIS, CHA order, with `-` where none applies.

## Stored and derived data

The compiler rebuilds everything it can. A stored file exists only when it is
reviewed source material that an entry cannot express.

| Library file | Decision | Reason |
|---|---|---|
| `entries/*.json` | **Stored** as `entries/**/*.md` | The rules text and typed metadata are the corpus. |
| `catalog.json` | **Partly stored** | Publication metadata is reviewed source data in `publications.toml`. Entry stubs, `source`, `shard`, and `entry_count` are derived from the entries. |
| `books.json` | **Stored** as `books.toml` | Chapter order is a curated decision, not a function of the body text. |
| `legal.json` | **Stored** as `legal.toml` | Licences and notices are reviewed legal text. |
| `aliases.json` | **Stored** as `aliases.toml` | The legacy map is a reviewed compatibility decision. |
| `relations.json` | **Derived** | It is exactly the entries' `relations` edges, sorted, with counts. |
| `lists.json` | **Derived** | It indexes `SpellFields::levels`; the entries stay the source of truth. |
| `coverage.json` | **Derived** | It reports what happened during a compile, not source data. |
| `backlinks.json` and shards | **Derived** | They are a reverse index over the entries' body links. |
| `search.bin` | **Derived** | Built from titles, aliases, sections, and body text. |
| `manifest.json` | **Derived** | Hashes and byte counts describe build output. |
| `publications` registry | **Stored** as `publications.toml` | Titles, categories, dates, and Section 15 lines are reviewed metadata; counts are not. |

The rule of thumb: if two people could disagree about it, store it; if a
deterministic pass over the entries can produce it, derive it.

`RuleEntry::links` is the set of body links. It is derived, never stored:

- an `Inline::Link` adds its target and anchor;
- an `EntryLink` block adds its target with no anchor;
- links inside emphasis, headings, tables, lists, quotes, and stat rows count;
- the result is sorted and deduplicated.

The current compiled library has five entries whose stored `links` contains a
reference that the legal screen removed from the body. The corpus cannot store
that invisible reference without inventing body content, so the round-trip
checker recomputes links on both sides and reports those five separately. Every
stored field is still compared exactly.

## Body

The body is a strict Markdown subset. It maps one-to-one to `Block` and `Inline`.
There is no raw HTML: `<` and `>` are escaped text, never markup.

Blocks are separated by one blank line. The writer emits exactly one blank line
between blocks and never trails a file with a blank line. A paragraph's source
line breaks are soft and read back as a space; use `\n` for `Inline::Break`.

### Headings

```markdown
## Extraordinary Abilities {#extraordinary-abilities}
```

Two to six `#`, one space, the heading text, and an optional ` {#anchor}`.
Level 1 is reserved for the entry title, which is not a block. A heading with no
anchor omits the braces. The anchor is `[A-Za-z0-9_.-]+`. This example is from
Special Abilities (Core Rulebook).

### Paragraphs

```markdown
A bank of fog billows out from the point you designate. The fog obscures all
sight, including darkvision, beyond 5 feet.
```

A leading `-`, `+`, `:::`, `#`, `>`, `|`, or `N.` would start another block, so
the writer escapes the first character when needed. This keeps any paragraph
text lossless.

`ParagraphStyle::StatLine` and `ParagraphStyle::Indented` use a fence:

```markdown
::: paragraph statline
**School** conjuration (creation) [acid]
:::
```

`statline` is the compact stat-block line style; `indented` is upstream's
`indent` style. The examples are from the Words of Power rules (Ultimate Magic).

### Lists

```markdown
- Adventurer:

  - [Battle Mage](ref/npc/adventurer-battle-mage)
  - [Monster Hunter](ref/npc/adventurer-monster-hunter)
- Brigand:

  - [Bandit Lord](ref/npc/brigand-bandit-lord)
```

`-` starts an unordered item and `1.` starts an ordered item. The writer uses
`1.` for every ordered item because the schema stores only the ordered flag.
An item's continuation blocks are indented two spaces. A blank line ends the
list unless the next line is indented item content, which is how nested lists
and multi-block items stay inside their item and adjacent lists stay separate.
This example is from A List of NPCs (GameMastery Guide).

### Quotes

```markdown
> **Serpentine Eagle:** This Andoren variant of the serpentine owl can be
> transformed into an [eagle](ref/monster/eagle).
```

Every line of the quote starts with `>`; a bare `>` is a blank line between
inner blocks. The inner text is parsed as blocks, so a quote can hold lists and
paragraphs. This example is from Figurine of Wondrous Power (Core Rulebook).

### Tables

A simple table uses pipes:

```markdown
|Lever (d8)|Lever Function|
| --- | --- |
|1|Uncover/cover forward porthole|
|7|Open/close eyes with _continual flame_ inside|
```

The first row is `head` (cells are headers), the `---` row is the delimiter, and
the rest is `body`. A table stays in pipe form only when it has one header row,
no caption, no foot, no header cells in the body, no spans, and the same number
of cells in every row. This example is from the Bronze Sentinel (Secrets of the
Sphinx).

Anything else uses a fenced table:

```markdown
::: table
caption = "Table: Special Ability Types"
head = [
  [
    { text = "", header = true },
    { text = "Extraordinary", header = true },
  ],
]
body = [
  [
    { text = "Dispel" },
    { text = "No" },
  ],
]
foot = [
  [
    { text = "Colspan and rowspan use the same form.", colspan = 2 },
  ],
]
:::
```

The fenced body is TOML. `caption` is inline text. `head`, `body`, and `foot`
are arrays of rows; a row is an array of cells. A cell has `text` (inline text),
optional `header`, `colspan`, and `rowspan`. This example is from Special
Abilities (Core Rulebook).

### Stat blocks

```markdown
::: statblock
title = "Spell statistics"
School: conjuration
Level: 2: Arcanist, Druid, Hunter, Magus
Casting time: 1 standard action
:::
```

`title` is optional and uses TOML quoting. Every other line is `Label: value`;
the first unescaped colon splits the label from the inline value. A label
escapes `\` and `:` with a backslash. This example is from Fog Cloud (Core
Rulebook).

### Entry links

```markdown
::: entry-link
target = "ref/carried/core-rulebook/rules/glossary/special-abilities/curses/baleful-polymorph-spell"
title = "Baleful Polymorph Spell"
:::
```

`EntryLink` is a rules-tree child that has its own entry. It is a block, not an
inline link, because it stands alone and is not part of a sentence.

## Inline

| Construct | Source form | Notes |
|---|---|---|
| Text | `plain text` | Escape the special characters below. |
| Emphasis | `_text_` or `*text*` | Nested emphasis alternates the delimiter. |
| Strong | `**text**` | |
| Superscript | `^1^` | |
| Subscript | `~2~` | |
| Break | `\n` | One `Inline::Break`. |
| Link | `[label](ref/spell/fog-cloud#effects)` | Target must be a canonical id; `#anchor` is optional. |

The writer starts with `_` for the outermost emphasis and alternates `_` and `*`
for deeper emphasis, so `_outer *inner* text_` is unambiguous. Strong always
uses `**`.

### Escaping

A backslash makes the next character literal. The writer escapes these
characters everywhere in text:

```text
\ * _ ^ ~ [ ] # > | { } <
```

Parentheses and underscores in prose stay readable; only `_` is special, and
the writer escapes it. The following sequences are structural:

| Sequence | Meaning |
|---|---|
| `\n` | `Inline::Break` |
| `\N` | A literal newline character in text |
| `\R` | A literal carriage return in text |
| `\0` | Text-node boundary: flush the current text so the next text becomes a separate `Text` node |
| `\e` | An empty `Text` node |

`\0` and `\e` exist only because the schema keeps adjacent and empty `Text`
nodes as separate values. They occur in a handful of entries. A literal
backslash is `\\`; therefore a literal `\n` in text is `\\n`.

Unescaped `*`, `_`, `^`, `~`, `[`, `]`, or `:` where syntax expects one of them
is an error with a line number. An unclosed emphasis span, link, fence, or table
is also an error.

## Readers and errors

`read_entry(path, text)`:

- rejects a file that does not start with `+++` or does not close it;
- rejects an unknown or missing front-matter key;
- rejects an unknown typed-fields key for the declared `kind`;
- rejects an id whose path does not match `entries/<id without ref/>.md`;
- rejects invalid ids, kinds, challenge ratings, and link targets;
- reports the file path and a 1-based line for every error.

`cargo xtask corpus-check <dir>` reads every entry and stored file and fails,
naming the file and line, when:

- an id is duplicated or a path does not match its id;
- a `source_id` is not in `publications.toml`;
- a `license_ref` is not in `legal.toml` and is not `legal#community-use`;
- a parent, body link, relation target, book node, or alias target names an
  unknown entry.

## Determinism

The writer is deterministic: the same `RuleEntry` gives byte-identical text, and
the same library gives byte-identical corpus files. It uses a fixed key order,
fixed block separation, no timestamps, and no hash-map iteration. The export
command clears its output directory first, so a stale file cannot survive a
second export.
