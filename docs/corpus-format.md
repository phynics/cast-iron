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
corpus.toml                     format version and the defaults entries omit
entries/<id without ref/>.md    one entry per file
publications.toml               publication metadata used by the catalogue
books.toml                      curated chapter order (BookIndex)
legal.toml                      licences, Product Identity, Section 15, dataset pins
exclusions.toml                 what the import left out and why; rows are only appended
history.toml                    frozen results of the import; never edited
aliases.toml                    legacy-to-canonical map (optional, when the library has one)
```

The path is derived from the permanent id. `ref/spell/fog-cloud` is
`entries/spell/fog-cloud.md`; `ref/carried/core-rulebook/rules/glossary/special-abilities`
is `entries/carried/core-rulebook/rules/glossary/special-abilities.md`. Every
published id starts with `ref/`, so the rule is reversible: remove `entries/`
and `.md`, then prepend `ref/`. The file also stores `id` in the front matter, and the
reader rejects a file whose path does not match it.

Ids never change. Moving a file, or editing an id, is a data error. A legacy id gets a
row in `aliases.toml` and never a second meaning; the corpus does not invent a new path
for an existing id.

## Entry front matter

The file starts with `+++`, then TOML, then a closing `+++` line. Keys appear in
schema order. Empty keys are omitted. `links` is never written; the reader
recomputes it from the body. `license_refs` is written only when it differs from
the default in `corpus.toml`, and `relations` never holds a prerequisite that
`prerequisite_feats` already states (see the key table).

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
| `provenance` | array of tables | Optional. `dataset`, `record`, `path` of the upstream record the entry came from. **Frozen history: do not edit it, and leave it out of a new entry.** |
| `license_refs` | array of strings | Omitted when it equals `[defaults] license_refs` in `corpus.toml` (today `["ogl-1.0a", "legal#community-use"]`). Written only for an entry that differs; an empty list is written as `license_refs = []`. Each value is a licence id from `legal.toml`, or `legal#community-use`. |
| `relations` | array of tables | `kind` (`part`, `archetype`, `prerequisite`), `target`, and optional `label` (`or`, `alternate`). A feat's prerequisite relations are **derived** from `prerequisite_feats`, one per listed feat, in list order, so they are not stored. Only a prerequisite that the list cannot state stays here: one with a `label` (`or`) or one whose target is not in the list. |

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

## `corpus.toml`

```toml
format = 1

[defaults]
license_refs = ["ogl-1.0a", "legal#community-use"]
```

`format` is the corpus format version. A reader refuses a corpus whose format it
does not know, and the message names the command that converts it
(`cast-iron-corpus migrate`). `[defaults]` declares, once, the values an entry
omits. Today that is `license_refs`.

### Format versions

The format number changes when the files mean something different: a renamed
key, a new default, a different escape or layout rule. Adding an optional key
that an old reader would reject also counts. A format change always ships with:

1. a `migrate` step in the tooling that converts a corpus from the previous
   format and rewrites `corpus.toml`;
2. one mechanical pull request that applies the migration to the whole corpus
   and does nothing else, followed by `fmt` so every file is canonical.

Never mix a migration with an edit to content. Reviewers can then check the
migration PR by running the step and comparing.

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
| `relations.json` | **Derived** | It is exactly the entries' `relations` edges, sorted, with counts. A feat's prerequisite edges come from `prerequisite_feats`. |
| `lists.json` | **Derived** | It indexes `SpellFields::levels`; the entries stay the source of truth. |
| `coverage.json` | **Derived**, from the entries, `exclusions.toml`, and `history.toml` | Counts follow the entries, so an edit cannot leave them stale. The exclusion and rejection rows, and a few results of the import, are stored. |
| `backlinks.json` and shards | **Derived** | They are a reverse index over the entries' body links. |
| `search.bin` | **Derived** | Built from titles, aliases, sections, and body text. |
| `manifest.json` | **Derived** | Hashes and byte counts describe build output. |
| `publications` registry | **Stored** as `publications.toml` | Titles, categories, dates, and Section 15 lines are reviewed metadata; counts are not. |

The rule of thumb: if two people could disagree about it, store it; if a
deterministic pass over the entries can produce it, derive it.

`RuleEntry::links` is the set of body links. It is derived, never stored:

- an `Inline::Link` adds its target and anchor;
- an `EntryLink` block adds its target with no anchor;
- links inside emphasis, headings, tables, lists, quotes, captions, and stat rows count;
- the result is sorted and deduplicated.

The corpus reader recomputes this list from the body when it reads the entry, so a
hand edit to a body cannot leave a stale link list.

### Coverage

The compiled `coverage.json` is rebuilt from three sources.

**Derived at read time** from the entries, the publication list, and the book
index, so a hand edit never leaves a count stale:

- counts by kind and by publication (included entries, with the title from
  `publications.toml`);
- link totals and relation edge counts per kind (a feat's prerequisite edges come
  from `prerequisite_feats`);
- spell-list coverage;
- backlink coverage.

**`exclusions.toml`** holds what the import left out. It has two kinds of row:

```toml
[[excluded]]
path = "json/faiths8.json"
records = 420
reason = "Product Identity"

[[rejected]]
path = "json/archetypes_barbarian.json#totem_warrior"
reason = "uses compileFrom; recipe expansion is not implemented"
```

`excluded` names an upstream path that was not selected, with the number of
records it holds. `rejected` names a record that was selected but dropped.
Nothing derives these rows from the entries, because the entries do not contain
what is absent. Rows are only appended: a pull request that edits or removes a
row is wrong, and a duplicate `path` plus `reason` is a read error. The file may
be empty.

**`history.toml`** holds the results of the one-time import that the entries
cannot state, kept as they were at the cut:

- `input_digest`, the digest of the upstream inputs;
- `[[publications]]`, the upstream record count per publication (a `title` only
  where the report's title differs from the catalogue's);
- `[prerequisites]`, how the import read feat prerequisite wording;
- `[[unplaced_parts]]` and `[[unpromoted]]`, what the import could not place or
  promote;
- `[book_order]`, how upstream page numbers placed entries in books.

Do not edit the file. These values describe the import, not the corpus. The
upstream dataset pins stay in `legal.toml`.

`cargo xtask content` derives the report from these files and writes it to
`coverage.json`.

## Body

The body is a strict Markdown subset. It maps one-to-one to `Block` and `Inline`.
There is no raw HTML: `<` and `>` are escaped text, never markup.

Blocks are separated by one blank line. Only an ASCII space or tab is blank;
Unicode spaces such as a non-breaking space are paragraph content. The writer
emits exactly one blank line between blocks and never trails a file with a blank
line.

Three keystrokes mean three different things inside a paragraph:

| You type | You get |
|---|---|
| Enter | A space. The paragraph continues on the next line. |
| Enter, blank line | A new block. |
| `\` then Enter | A line break (`Inline::Break`). |

The writer wraps a paragraph at about 100 columns. It breaks only at a single
space between two words, and never inside a link `[label](target)`, an escape,
or where the next line would start with a block marker. A reader joins the lines
with one space after it trims the blanks at both ends of each line, so a wrapped
paragraph and the same paragraph on one line mean the same. `corpus fmt`
re-wraps.

### Headings

```markdown
## Extraordinary Abilities {#extraordinary-abilities}
```

Two to six `#`, one space, the heading text, and an optional ` {#anchor}`.
Level 1 is reserved for the entry title, which is not a block. A heading with no
anchor omits the braces. The anchor is `[A-Za-z0-9_.-]+`; any other suffix in
braces is an error, not heading text. This example is from Special Abilities
(Core Rulebook).

**Linked headings carry an explicit anchor.** A heading that any entry body link
or any `books.toml` node reaches carries `{#anchor}`. Only a heading that nothing
links to may omit it; the client then derives the URL key `sec-<slug of the
text>` (with `-2`, `-3` for repeats). The rule keeps a link alive when someone
edits a heading's text: the anchor does not move with the words. The anchor
string is the key the heading already had, so a section URL does not change.
The content compiler writes the anchors; `corpus fmt` adds one when an edit links
to a heading that has none.

A link anchor must name a heading anchor in the target entry. The check names
the file with the link, the target file, the headings the target has, and the
fix.

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
optional `header`, `colspan`, and `rowspan`. Spans must be at least 1; an
explicit 0 is an error. This example is from Special Abilities (Core
Rulebook).

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
| Break | `\` at the end of a line | One `Inline::Break`. The next line starts after the break. |
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

Parentheses in prose stay readable. A backslash may precede any ASCII
punctuation character, digit, space, or tab; the writer uses a leading `\` to
keep a line start from reading as a block marker (`\- text`, `\1. text`) and to
keep a space at the start or end of a line (`\ `). Two sequences are
structural:

| Sequence | Meaning |
|---|---|
| `\N` | A literal newline character in text |
| `\R` | A literal carriage return in text |

Any other backslash followed by a letter is an error with a line number. A
literal backslash is `\\`.

A backslash at the very end of a line is a line break. In a heading, a pipe
table cell, and a fenced table string, which cannot hold a second line, the break
is written `\n`, and the reader accepts `\n` anywhere. A stat block row is one
line unless it ends in `\`; then the value continues on the next line.

There is no spelling for two adjacent `Text` nodes or for an empty one. The
writer merges and drops them, and the content compiler never produces them.

Unescaped `*`, `_`, `^`, `~`, `[`, `]`, or `:` where syntax expects one of them
is an error with a line number. An unclosed emphasis span, link, fence, or table
is also an error.

## Readers and errors

`read_entry(path, text, config)`:

- rejects a file that does not start with `+++` or does not close it;
- rejects a carriage return anywhere in the file (a Windows line end);
- rejects an HTML comment (`<!--`) and any raw HTML tag, such as `<b>`; write a
  literal `<` as `\<`;
- rejects an unknown or missing front-matter key;
- rejects an unknown typed-fields key for the declared `kind`;
- rejects an entry kind that disagrees with the typed-fields kind;
- rejects an id whose path does not match `entries/<id without ref/>.md`;
- rejects invalid ids, kinds, challenge ratings, and link targets;
- reports the file path and a 1-based line for every error.

`corpus-check <dir>` reads every entry and stored file and fails, naming the
file and line, when:

- an id is duplicated or a path does not match its id;
- an entry `kind` does not match its `[fields]` kind;
- a `source_id` is not in `publications.toml`;
- a `license_ref` is not in `legal.toml` and is not `legal#community-use`;
- a parent, body link, prerequisite feat, relation target, book node, or alias
  target names an unknown entry;
- a body link or a `books.toml` anchor is not a heading key of the target entry
  (the message names the linking file, the target file and its headings, and
  the fix);
- `exclusions.toml` repeats a row;
- a publication in `publications.toml` has no `copyright`, or is covered by neither a
  `section_15` notice in `legal.toml` (the notice text is part of the publication's
  `copyright`) nor a title in `notice_pending`. The message names `publications.toml`, the
  line, and the publication id. This is the licensing gate for adding a book: its
  Section 15 notice, or a recorded pending decision, goes in the same change.

## Drift between fields and body

Some values live twice in an entry: in `[fields]`, and in the body. `corpus-check`
compares each pair that states a value in both places and lists every disagreement
as a **warning** that names both places:

```text
entries/feat/accursed-hex.md:12: [fields] prerequisites = "Accursed Hex." disagrees with the `**Prerequisites:**` line = "Hex class feature." (line 20)
```

It prints the count per kind and the first warnings. The full list is not stored in the
repository; print it with `--drift-report`:

```sh
cargo xtask corpus-check .cache/sources/cast-iron-data --drift-report /tmp/corpus-drift.md
```

The file holds the count per kind, then one line per disagreement. The check is a
warning, not an error, because the freeze data already holds disagreements (80 in
corpus `v0.1.0`: 79 feats, 1 creature). Fix the wrong copy, or leave it.

| Kind | Typed field | Body |
|---|---|---|
| spell | `school`, `subschools`, `descriptors`, `levels`, `casting_time`, `components`, `range`, `effects`, `duration`, `saving_throw`, `spell_resistance` | the matching rows of the stat block (`School`, `Level`, `Range`, `Target`/`Effect`/`Area`, ...) |
| creature | `cr`, `creature_type`, `size`, `alignment`, `ac`, `hp`, `fortitude`, `reflex`, `will`, `melee`, `ranged`, `special_attacks`, `special_qualities`, `abilities` | the `CR`, `Type`, `Size`, `AC`, `hp`, `Fort`, ... rows |
| item | `price`, `weight`, `slot`, `aura`, `caster_level` | the `Price`, `Weight`, `Slot`, `Aura`, `CL` rows |
| feat | `prerequisites`, `benefit`, `normal`, `special` | the bold `**Prerequisites:**`, `**Benefit:**`, `**Normal:**`, `**Special:**` lines |

The comparison reads both sides as words: it ignores case, repeated blanks, an
end period, `*` and `[label](target)` markers, and a few entities (`&times;`). A
spell's `School` row may add the subschool and descriptors. A feat field agrees
when any `**Label:**` line in the body says the same, because a feat with a
mythic variant states the label twice. A value stated in only one place is not
drift. The two copies are not unified: the typed fields feed search and facets,
and the body is what a reader sees.

## Formatting

`corpus-fmt <dir>` rewrites every file into its one canonical form, and
`corpus-fmt <dir> --check` lists the files that are not canonical. For each file
the check prints the first line that differs, as `path:line: expected X, found
Y`, and fails. The rule is `write(read(file)) == file`.

The reader accepts some spellings that the writer never produces. `fmt`
normalizes them:

| You wrote | `fmt` writes |
|---|---|
| `\| a  \|  b \|` (blanks around a table cell) | `\|a\|b\|` |
| `2.`, `3.` list numbers | `1.` |
| two or more blank lines between blocks | one |
| text on one very long line | wrapped at about 100 columns |
| `*emphasis*` at the top level | `_emphasis_` |
| no newline at the end of the file | one newline |
| a link to a heading that has no `{#anchor}` | the heading gets `{#its-key}` |
| blanks at the end of a line | none |

A TOML file (`corpus.toml`, `publications.toml`, `books.toml`, `legal.toml`,
`exclusions.toml`, `history.toml`, `aliases.toml`) is read and written back by
the tool, so a comment in it does not survive `fmt` and fails `--check`.

## Contributor kit

`corpus-kit/` holds what the data repository needs to accept edits from people
who do not build Cast Iron. It is copied into `cast-iron-data`.

- `README.md`: the three rules and one recipe for each common edit (fix a typo,
  fix a link, split a paragraph, add a table row, change a stat value, add an
  entry).
- `examples/`: one annotated, valid example file per entry kind. A test reads
  every example, requires it to be canonical once its comments are removed, and
  checks the set as one corpus.
- `.github/workflows/corpus.yml`: builds `cast-iron-corpus` at a pinned Cast Iron
  revision, then runs `check .` and `fmt --check .`.
- `.github/pull_request_template.md`: the checklist for a data pull request.

The `cast-iron-corpus` binary (`cargo install --git ... cast-iron-corpus`) has
three commands: `check [DIR] [--drift-report FILE]`, `fmt [--check] [DIR]`, and
`migrate [DIR]`. It depends only on the corpus and schema crates. The same code
runs behind `cargo xtask corpus-check`, `corpus-fmt`, and `corpus-migrate`.

## Determinism

The writer is deterministic: the same `RuleEntry` gives byte-identical text. It uses a
fixed key order, fixed block separation, no timestamps, and no hash-map iteration, so
`fmt` rewrites a file to the same bytes every time.
