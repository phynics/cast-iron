# Corpus format

The corpus is a plain-text form of the compiled rules library. One file holds
each entry, and a few small files hold the reviewed source data that the entries
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
books.toml                      curated chapter order (BookIndex); formats 1 and 2
books/<publication-id>.toml     one curated book; format 3 (replaces books.toml)
legal.toml                      licences, Product Identity, per-publication bases, Section 15, dataset pins
exclusions.toml                 what the import left out and why; rows are only appended, a later row can supersede one
history.toml                    frozen results of the import; never edited
aliases.toml                    legacy-to-canonical map (optional, when the library has one)
option-groups.toml              class-option vocabulary (optional, schema 4)
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

Front-matter strings hold plain characters, not HTML entity references. `corpus-check`
fails on a named or numeric reference (`&mdash;`, `&times;`, `&#39;`, `&#x2014;`) in the
title, the tags, or a typed field. Write the character (`—`, `×`), or omit an optional key
when the value means "none" (a `weight` printed as a dash). Body text is not scanned for it.

```text
+++
id = "ref/spell/fog-cloud"
kind = "spell"
title = "Fog Cloud"
aon = "SpellDisplay.aspx?ItemName=Fog%20Cloud"
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

[[sources]]
publication = "core-rulebook"
page = "221"

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
| `kind` | string | `EntryKind` slug: `rule`, `spell`, `feat`, `creature`, `item`, `class`, `prestige_class`, `race`, `skill`, `affliction`, `trap`, `trait`, `racial_trait`, `archetype`, `class_option`. Required. |
| `title` | string | Display title. Required. |
| `aon` | string | Optional Archives of Nethys page, as a path relative to `https://aonprd.com/`, such as `FeatDisplay.aspx?ItemName=Power%20Attack`. It names a `*Display.aspx` page with `ItemName`, `FixedName`, or `FinalName`; no scheme, host, leading slash, or fragment. |
| `source_ids` | array of strings | Publication ids. Every one must exist in `publications.toml`. |
| `section` | array of strings | Breadcrumb inside the rules hierarchy. |
| `parent` | string | Containing rules entry; must resolve to another entry. Omitted when absent. |
| `tags` | array of strings | Stable labels such as `standard_race`. |
| `race_group` | string | Optional header key for a `race` entry: `core_race`, `featured_race`, `uncommon_race`, or `other_race`. The first three are the Advanced Race Guide's printed chapters; `other_race` is a race printed outside them. `corpus-check` requires it on every entry of kind `race` once any race carries it, so the corpus pinned before the data still compiles; the pin and that check therefore land in the same change. The key is rejected on every other kind. |
| `provenance` | array of tables | Optional. `dataset`, `record`, `path` of the upstream record the entry came from. **Frozen history: do not edit it, and leave it out of a new entry.** |
| `license_refs` | array of strings | Omitted when it equals `[defaults] license_refs` in `corpus.toml` (today `["ogl-1.0a", "legal#community-use"]`). Written only for an entry that differs; an empty list is written as `license_refs = []`. Each value is a licence id from `legal.toml`, or `legal#community-use`. |
| `relations` | array of tables | `kind` (`part`, `archetype`, `prerequisite`, `variant`, `addon`, `revision`), `target`, and an optional `label`. Labels are closed per kind: `or` on `prerequisite`; `alternate` on `part`; exactly one of `mythic`, `stamina`, `changes` on `addon`; none, `removed`, or `merged` on `revision`; none on `variant` and `archetype`. A feat's prerequisite relations are **derived** from `prerequisite_feats`, one per listed feat, in list order, so they are not stored. Only a prerequisite that the list cannot state stays here: one with a `label` (`or`) or one whose target is not in the list. |
| `sources` | repeated `[[sources]]` table | Optional. One row per printed citation, in print order. `publication` is a publication id that must also appear in `source_ids`; `page` is optional. See [`[[sources]]`](#sources). |
| `expanded_in` | repeated `[[expanded_in]]` table | Optional. A book that expands the entry without printing it: `publication` (must not be in `source_ids`), optional `page`, optional `via` entry id. Never membership. See [`[[expanded_in]]`](#expanded_in). |

### `[[sources]]`

A printed citation, one row per publication in the order the book prints it. The rows
replace a `**Sources:** Book pg N` line that the import used to leave in the body; the
compiler carries them to the client as the optional `sources` entry field.

```toml
[[sources]]
publication = "ultimate-combat"
page = "221"

[[sources]]
publication = "advanced-players-guide"
page = "12-14"
```

`publication` is a publication id from `publications.toml`, and `corpus-check` also requires
it in `source_ids`. Keep `source_ids`: it is the stable, ordered list that active-book
filtering uses. `page` is optional. It is a string, not a number, because printed pages are
not always one number: `"12-14"`, `"263-4"` and `"inside cover"` all occur. The writer puts
the rows in list order and leaves `page` out when the citation has none.

### `[[expanded_in]]`

A credit for a book that expands an entry without printing it, for when the expanding text
has no entry of its own. It is **not** membership: the book is not in `source_ids`, and
membership, counts, filters, search extra sources and page order never read the rows. When the
expanding text has its own entry, use a relation (`variant`, `addon`) instead.

```toml
[[expanded_in]]
publication = "monster-summoners-handbook"
page = "18"
via = "ref/spell/summon-monster-ii"
```

`publication` is required. `page` is optional and a string, as in `[[sources]]`. `via` is optional
and is an entry id. The rows follow `[[sources]]` in the header, in list order, with the keys in
the order shown; the writer leaves out an absent `page` or `via`. `corpus-check` requires that:

- the publication exists in `publications.toml`;
- the publication is not in the entry's `source_ids`;
- `via` resolves to an entry whose `source_ids` include the publication.

The compiler carries the rows to the client as the optional `expanded_in` entry field. It is
additive, so the schema version does not change.

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
| `archetype` | `trades` (format 2; see [Trades](#trades)). Format 1: `replaces`, `alters` (printed feature names, in print order), `replaces_features`, `alters_features` (heading anchors of the base class, in print order) |
| `class_option` | `group` (required slug from `option-groups.toml`), then the same trade keys as `archetype`, with the group's classes as the base classes |
| `none` | none |

`levels`, `effects`, and `attributes` are arrays of inline tables. `cr` is the
printed fraction (`1/2`, `1/8`) or a whole number. `abilities` is six strings in
STR, DEX, CON, INT, WIS, CHA order, with `-` where none applies.

### Trades

An archetype or class option trades features of its base class: it replaces them or alters
them. The corpus holds the trades as printed, with an optional reference to the feature each one
reaches. An archetype's base class is its `archetype` relation target; a class option's base
classes are the `classes` of its group in `option-groups.toml`.

**Format 2** (#450) writes one flat `trades` list, one inline table per trade, in print order:

```toml
[fields]
kind = "archetype"
trades = [
  { replaces = "Trap sense (Ex)", feature = "trap-sense" },
  { alters = "Bonus feats", feature = "bonus-feats", levels = [4, 8] },
  { replaces = "Acidic ray", feature = "bloodline-powers", power = "Warp Touch" },
  { replaces = "3rd-level bonus feat" },
]
```

| Key | Rule |
|---|---|
| `replaces` or `alters` | Exactly one. The feature name as the book prints it. Kept as written, and the drift check compares the list of each key with the `**Replaces:**` and `**Alters:**` lines. |
| `feature` | Optional. A heading anchor of a base class entry. A trade without it is unresolved: it keeps its printed name, and no compatibility filter treats it as compatible with anything. |
| `class` | Optional, only with `feature`. The base class entry whose anchor it is. Required when more than one base class of the entry holds the anchor (an archetype of several classes, a class option whose group has several classes); otherwise the compiler drops it. |
| `levels` | Optional, only with `feature`. The class levels at which the trade applies: 1 to 20, ascending, each once. No `levels` means every level of the feature. |
| `power` | Optional, only with `feature`. A stable named component inside the feature, such as a bloodline power, a domain power, a bardic performance or a gunslinger deed, written the same in every entry. By corpus convention never a progression step (see below); the reader accepts any text and the check does not validate its meaning. Compared by exact text. |

**Progression steps are grant levels.** This is a corpus authoring convention. `levels` identifies a progression step by the base-class grant levels, read from the class feature table of the base class: `Armor training 2` is `levels = [7]`, `Trap sense +3` is `[9]`, `Judgment 1/day` is `[1]`, and a list of steps names the levels of its steps (`Weapon training 1 and 3` is `[5, 13]`). `power` names a component only when the level alone is not enough, and a step is never a `power`. The adopted component names are:

- a bardic performance or a gunslinger deed, by its name;
- for `weapon-and-armor-proficiencies`: `weapons`, `armor`, `light armor`, `medium armor`, `heavy armor` and `shields`;
- for a spell-list substitution: `feature = "spells"` with `power = "spell list"`.

The migration of the corpus repository (#452) normalized the step powers that `migrate-trades` wrote (`power = "2"`, `power = "+4"`) into levels, with documented exceptions in its adjustment log (Proselytizer keeps `power = "major"`, because `[10]` would duplicate its alteration of the whole feature). `migrate-trades` itself still writes a step as a `power` (the qualifier rule below) until a later change.

**Two different comparisons.** The duplicate check below compares exactly: two trades of one key overlap only when they name the same anchor, share a level and have the same `power` (no `power` matches only no `power`). It stays exact. The compatibility filter of #170 part D needs a different matcher, for trades of the same base-class feature: keeping the whole feature matches a replacement of any of its levels or named components, and keeping a level or a component also matches a replacement of the whole feature. That matcher is not built yet; #170 part D tracks it.

`corpus-check` holds these rules for the trades:

| Rule | Result |
|---|---|
| A trade has exactly one of `replaces` and `alters`, with a text that is not blank; `levels`, `power` and `class` need a `feature`. | The reader fails the file (format 2). |
| The printed names of the trades with each key agree with the `**Replaces:**` and `**Alters:**` lines of the normal part, and an entry with such lines has trades for that key. | Drift: fails (see [Drift between fields and body](#drift-between-fields-and-body)). |
| A `feature` is a heading anchor of a base class, and a `class` is a base class that holds the anchor. | Fails. |
| `levels` run from 1 to 20, ascending, each once. | Fails. |
| No two trades of one key (two `replaces`, or two `alters`) name the same anchor with a shared level (no `levels` shares every level) and the same `power`. An archetype may alter a feature and replace one instance of it. | Fails in format 2, warns in format 1. |
| A printed name resolves confidently to another anchor than the `feature`. | Warns. |
| Unresolved trades (no `feature`) are counted. A format 2 trade that is unresolved although [its printed name proves a feature](#migrating-to-format-2) names the feature. | One count per run, and one warning for each provable trade in format 2. A format 1 corpus gets the count and how many trades have a provable match that `migrate-trades` would write, and no per-trade warning, so a format 1 corpus stays green. |

The compiler lowers the trades (see [Library files](#stored-and-derived-data)): it fails in format 2 when an
anchor several base classes hold has no `class` (in format 1 it leaves that trade unresolved), and it writes the trades into `trades.json` only.

#### Migrating to format 2

`cargo xtask corpus-migrate-trades <dir> [--review <file>]` (`cast-iron-corpus migrate-trades`;
the generic `migrate` runs the same step without the review file, and then the layout step of
format 3) converts a format 1 corpus
in place and writes `format = 2` to `corpus.toml`. It is idempotent: a second run on a migrated
corpus changes no byte, and it never changes a trade that already has a `feature`, so a fix from
the review queue survives. Run it on a clean checkout, then `check` and `fmt --check`.

For each archetype and class option it merges the printed names with what the base-class headings
prove:

1. A trade that the format 1 reader linked by slug is judged again by the rules below (the
   reader proves less: `Trap sense +3` and the anchor `trap-sense-3`). The same feature stays, a
   better proof replaces it, and no proof leaves the trade unresolved. A feature written in
   format 2 is never judged.
2. A trade without a feature is resolved only when the printed name proves it. Each rule needs
   exactly one heading to match, and the name is read without an ability tag, article, or the base
   class's own name:
   - **exact:** the name slugs to the slug of a heading's text, or to a heading anchor without a number in it (`trap-sense-3` may be a suffix that kept two headings apart, so it never proves `Trap sense +3`);
   - **tokens:** the name and the heading have the same words in any order, a plural read as its
     singular (`Investigator talents` is `Investigator Talent (Ex or Su)`);
   - **levels:** the name states ordinal levels (`The discovery gained at 6th level`,
     `2nd-, 12th-, 16th-level discoveries`); every ordinal must belong to a level expression (a run
     of ordinals joined by `and`, followed by `level` or `levels`), or the whole name stays
     unresolved (`2nd favored enemy gained at 5th level`); the rest is the token set of one
     heading, and the levels (1 to 20) become `levels`;
   - **qualifier:** the name is a feature plus one number or one trailing parenthesis
     (`Poison resistance +4`, `Touch treatment (minor)`); it becomes `power`.
3. When two trades of one key would name the same feature at a shared level and power (a book
   that prints `Smite evil` twice, or two overlapping lists of bonus feats), the later one stays
   unresolved, so the migrated corpus passes the overlap rule. Only a feature that this run
   resolved, or that the format 1 reader resolved, gives way; a format 2 feature stays as written.
4. An entry whose normal part prints `**Replaces:**` or `**Alters:**` lines for a key that has no
   trade gets one unresolved trade per printed name first, then step 2.

A position in a list, the length of a list, a name that only starts with or contains a feature
name (`Improved evasion` is not `Evasion`), a list of numbers (`Armor training 1, 2, 3, and 4`),
and an anchor that two base classes hold, whatever their heading texts (an archetype of two classes), prove nothing. Those trades
keep `feature` absent. They go to the review queue, a TSV (`generated/trade-review.tsv` for the
xtask command, `trade-review.tsv` in the current directory for the binary; never inside the
corpus) with one row per unresolved trade in entry id and trade order:

| Column | Content |
|---|---|
| `id`, `kind`, `trade`, `op`, `printed` | The entry, its kind, the trade's 1-based position, `replaces` or `alters`, the printed name. `trade` is 0 for a line that has no trade. |
| `reason` | `no-heading` (no heading has the name's words), `ambiguous-heading`, `ambiguous-class`, `unusable-qualifier`, `level-out-of-range`, `ordinal-is-not-a-level`, `duplicate-feature` (the same feature as an earlier trade of the key), `no-base-class`, `variant-only` (a class option prints the line only below its normal part, in a variant such as a subdomain, so a trade would disagree with the drift check), or `old-anchors-only` (an old anchor list with no printed name). |
| `candidates` | For an ambiguous reason, the headings (`class#anchor`) or classes. |
| `unclaimed_old_anchors` | The anchors of the old format 1 list for the same key that no trade of the entry claims. They are not guessed at; they are the reviewer's lead. |

**Format 1** wrote four lists instead: `replaces` and `alters` (the printed names, in print
order) and `replaces_features` and `alters_features` (heading anchors, in print order). The
reader still accepts format 1 and lowers the lists to trades. A trade resolves only when its whole
printed name, without an ability tag, slugs to an anchor of its list: `Bravery (Ex)` resolves to
`bravery`. Position and length prove nothing. The old lists left out every printed name that no
heading matched, so two lists of equal length can pair the wrong names, and a composite name such
as `Hunter's bond, 2nd favored enemy` next to `hunter-s-bond` is only partly that feature. Every
other trade stays unresolved, because a wrong link is worse than none. When the anchors of the
resolved trades are not exactly the list as read (a different order, or an anchor that no name
reaches), the reader keeps the list so that `fmt` writes the file back unchanged, and
`corpus-check` still validates it. An anchor that several base classes hold cannot name a class in
format 1, so the compiler leaves that trade unresolved rather than fail. Format 1 cannot write a
`class`, `levels` or `power`, a feature that is not the slug of its printed name, or a list that mixes `replaces` and `alters` in other than that
order; `corpus-check` rejects such a trade in a format 1 corpus. The command above migrates
a corpus (#451). The corpus repository applied it in v0.1.33 (#452); the pinned corpus is
format 2, and this tool still reads format 1.

## `corpus.toml`

```toml
format = 1

[defaults]
license_refs = ["ogl-1.0a", "legal#community-use"]
```

`format` is the corpus format version. This tool reads formats 1, 2 and 3. Format 2 changes the trade
keys (see [Trades](#trades)); format 3 changes the book layout (see [Books](#books-format-3)). A reader refuses a corpus whose format it
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

## `publications.toml`

Each `[[publication]]` row names a book. A row may carry `suffix`, the id segment that new
add-on and revision entries from that publication take:

```toml
[[publication]]
id = "the-elephant-in-the-room"
title = "The Elephant in the Room"
copyright = "…"
third_party = true
suffix = "teitr"
```

A suffix is one id segment (lowercase letters, digits, and hyphens) and must be unique across
the corpus. `corpus-check` rejects a duplicate or an invalid one, and an add-on or revision
that cannot name its publication's suffix.

A row may carry `aon`, the Archives of Nethys source page, as a path relative to
`https://aonprd.com/`:

```toml
aon = "SourceDisplay.aspx?FixedSource=Core%20Rulebook"
```

The value names `SourceDisplay.aspx` with the AoN source name in `FixedSource`. It has no
scheme, host, leading slash, or fragment, and it is omitted when no confident match exists.
`corpus-check` validates it, and the compiler carries it into `catalog.json` as the optional
`SourceInfo.aon` field.

`third_party = true` marks a publication that Paizo did not publish. Absent means a Paizo
book, and the canonical writer omits the field when it is false. The marker decides the
"Third party" book tier and keeps the book out of every preset except
"Everything incl. 3rd party". `corpus-check` rejects `third_party = true` beside a copyright
line that names Paizo, because the two contradict each other. It does not require the marker
when a copyright line omits Paizo: several Paizo books still have a pending Section 15
notice, so absence is not evidence of a third-party publisher.

### Series fields

Adventures can carry an optional series grouping (#159). Every field is optional, so the
schema version does not change, and a publication without `series_kind` belongs to no
series. `series_kind` takes one of three values, and the other fields depend on it:

```toml
# An Adventure Path volume: the path name and the volume within it.
series_kind = "adventure-path"
series = "Rise of the Runelords"
series_number = "1"

# A standalone Module: no other field; the client orders modules by publication_date.
series_kind = "module"

# A Pathfinder Society scenario: the season, and the scenario number within it.
series_kind = "society"
series_number = "3-01"
season = 3
```

`corpus-check` enforces the shape:

- `series_kind` is `adventure-path`, `module` or `society`.
- `adventure-path` requires a non-empty `series` and a positive integer `series_number`, and
  rejects `season`.
- `module` rejects `series`, `series_number` and `season`, because publication date orders the
  line.
- `society` requires `season`; `series_number`, when present, must be `<season>-NN`, and
  `series` is rejected.
- A row with any series field but no `series_kind` is rejected.

The canonical writer emits the fields in the order above, after `publication_date` and before
`third_party`, and omits every unset one. The fields are carried into `catalog.json` as
optional `SourceInfo` fields; the grouping UI is a later change.

## `option-groups.toml`

The class-option vocabulary is optional. A missing file means no groups. Each table is a slug
that a `class_option` entry's `group` names:

```toml
[groups.arcane-discovery]
label = "Arcane discoveries"
singular = "Arcane discovery"
classes = ["ref/class/wizard"]
```

`label` is the plural display label, `singular` the label for one option, and `classes` the
class entries that take the group. Every id must resolve, and a declared group must hold at
least one `class_option` entry; `corpus-check` reports both failures. The compiler writes the
same data, plus the group's entries, to `option-groups.json`.

## Stored and derived data

The compiler rebuilds everything it can. A stored file exists only when it is
reviewed source material that an entry cannot express.

| Library file | Decision | Reason |
|---|---|---|
| `entries/*.json` | **Stored** as `entries/**/*.md` | The rules text and typed metadata are the corpus. |
| `catalog.json` | **Partly stored** | Publication metadata is reviewed source data in `publications.toml`. Entry stubs, `source`, `shard`, and `entry_count` are derived from the entries. |
| `books.json` | **Partly stored**: formats 1 and 2 store every book in `books.toml`; format 3 stores only the curated books in `books/<publication-id>.toml` | A printed chapter order is a curated decision, not a function of the body text. A page-order list is a function of the `[[sources]]` pages, so the compiler derives it (see [Books](#books-format-3)). |
| `legal.json` | **Stored** as `legal.toml` | Licences and notices are reviewed legal text. |
| `aliases.json` | **Stored** as `aliases.toml` | The legacy map is a reviewed compatibility decision. |
| `relations.json` | **Derived** | It is exactly the entries' `relations` edges, sorted, with counts. A feat's prerequisite edges come from `prerequisite_feats`. Each edge is `[source, kind, target]`, or `[source, kind, target, label]` when it carries a label. |
| `option-groups.json` | **Derived** | It joins `option-groups.toml` with the `class_option` entries that name each group. |
| `trades.json` | **Derived** | One `{ id, trades }` row per archetype or class option that has trades, sorted by id: the lowered trades (`class` filled only where the anchor is ambiguous). The entry shards hold no trades. It replaces `replaces.json` (schema 4 and 5, printed `replaces` names only), which a newer client still reads for an older library. |
| `lists.json` | **Derived** | It indexes `SpellFields::levels`; the entries stay the source of truth. |
| `coverage.json` | **Derived**, from the entries, `exclusions.toml`, and `history.toml` | Counts follow the entries, so an edit cannot leave them stale. The exclusion, rejection and resolved rows, and a few results of the import, are stored. |
| `backlinks.json` and shards | **Derived** | They are a reverse index over the entries' body links. |
| `search.bin` | **Derived** | Built from titles, aliases, sections, and body text. |
| `manifest.json` | **Derived** | Hashes and byte counts describe build output. |
| `publications` registry | **Stored** as `publications.toml` | Titles, categories, dates, and Section 15 lines are reviewed metadata; counts are not. |

The rule of thumb: if two people could disagree about it, store it; if a
deterministic pass over the entries can produce it, derive it.

### Book nodes (`books.toml`, `books/<publication-id>.toml`)

A curated book is a `BookTree`: a tree of chapters in the curated order. Formats 1 and 2 store every
book in one `books.toml`, a `BookIndex` with one `[[books]]` block per publication. Format 3 stores
only the curated books, one block in each `books/<publication-id>.toml` (see [Books](#books-format-3)).
A `[[books.chapters]]` row (or a nested `children` row) has a `title`, an optional `entry`, an
optional `anchor`, an optional `review` note, and `children`.

`collapse = true` is an optional authored marker. It is additive and omits when false, so older
readers ignore it and no `SCHEMA_VERSION` bump is needed. A marked node shows as one row in
Contents at any level, including a top-level chapter. The row keeps the muted count and links to
the node's entry; a marked node with no entry links to its chapter page. An unmarked top-level
chapter keeps its nested sections and chapter page. Below chapter level the compiler's automatic
rule still applies: a node that is itself an entry and whose body lists every child entry as an
`EntryLink` collapses without the marker.

`corpus-check` warns when a marked node has an entry and at least one child entry, and the body
does not link every child entry. Collapsing such a row would hide entries that its own page does
not show. A marker on a heading with no entry has no body to check.

`RuleEntry::links` is the set of body links. It is derived, never stored:

- an `Inline::Link` adds its target and anchor;
- an `EntryLink` block adds its target with no anchor;
- links inside emphasis, headings, tables, lists, quotes, captions, and stat rows count;
- the result is sorted and deduplicated.

The corpus reader recomputes this list from the body when it reads the entry, so a
hand edit to a body cannot leave a stale link list.

### Books (format 3)

Format 3 (#425) stores the printed chapter structure of the curated books only. Of the 439 books of
corpus v0.1.25, 16 have one (the printed contents of Core Rulebook, Bestiary, and so on). The other 423
were page-order lists that the page of each entry already determines: 2.49 MB of the 4.40 MB
`books.toml`, which the compiler can rebuild. The compiler now rebuilds them.

```text
books/<publication-id>.toml     one curated book: exactly one [[books]] block, `source` = the file stem
```

The file keeps the `[[books]]` block of formats 1 and 2 (`source`, `title`, `basis`, `review`,
`[[books.chapters]]`). A curated file may not hold a top-level chapter titled `Page order`, `Unpaged` or
`New entries (page order)`: those titles belong to the compiler, and `corpus-check` rejects them. A
format 3 corpus has no `books.toml`, and a corpus holds one layout or the other.

**What the compiler builds.** `book_index` (in `crates/corpus`, called by the compiler and by
`corpus-check`) gives every publication one book, in this order:

1. **A curated file wins.** Its chapters are kept as written. The compiler adds one last, hidden chapter,
   `New entries (page order)`, that holds the members the curated tree does not place (formats 1 and 2
   stored that chapter in the file). The app never shows it; reachability reads it.
2. **Every other publication is derived.** The *members* are the entries whose `source_ids` name the
   book. The *page* of a member is its first `[[sources]]` row for that book that has a page, read as
   its leading integer: `12-14` and `12, 14` give 12; `inside cover` gives none. A member with a page
   goes to the chapter `Page order`; the rest go to `Unpaged`. A chapter with no member is left out.
3. **Order inside a chapter:** page, then the kind slug, then the lowercased title, then the id. Entries
   with no page sort after the paged ones by the last three keys. The id makes the order total, so the
   output never depends on hash order or file order. Entries on one page are not in printed order, so
   the tie-break is a convention, not evidence.
4. **Nesting.** A member whose `parent` chain reaches another member nests under the nearest one, in
   that member's chapter, in the same order.
5. **Texts.** A derived node is `{ title = <entry title>, entry = <id> }`, with no anchor and no marker.
   `basis`, the book `review` and the chapter notes are fixed texts in the compiler, chosen by which
   chapters exist.
6. **Book order:** the curated books first, then the derived ones; each group by the title folded to
   lowercase ASCII letters and digits, then the publication id. The title of a derived book is its
   publication title.

`corpus-check` gains these rules for format 3:

- a curated file's name is its `source`, the `source` has a row in `publications.toml`, and the file has
  exactly one block;
- the reserved chapter titles are rejected in a curated file;
- the placement rule of formats 1 and 2 still holds for the curated nodes (the book must be in the
  entry's `source_ids`); a derived placement is a member by construction;
- a `[[sources]]` `page` must be `N`, `N-N`, a comma list of those, or `inside cover`. In the
  single `books.toml` layout a page that is not one only warns, because the stored lists do not read
  pages. In the split layout it is a problem, whatever the `format` number says (a split corpus below
  format 3 is a half-migrated one).

`cargo xtask corpus-migrate-layout <dir>` (or `cast-iron-corpus migrate-layout`) cuts a `books.toml` into
the curated files and removes it: a book with a top-level chapter other than the three above is curated;
every other book gets no file. It is idempotent, and a crash leaves `books.toml` in place, so a rerun
converges. It declares format 3 only for a format 2 corpus. A format 1 corpus still has the trades
migration to run, and the reader accepts the split layout in a format below 3 with a warning. The
registered `migrate` chain is 1 to 2 (trades), then 2 to 3 (this cut, writing `format = 3`), so
`migrate` takes a format 1 corpus to format 3 in one run.

The library does not change: `books.json` keeps its shape, so neither `SCHEMA_VERSION` nor
`INDEX_FORMAT_VERSION` moves. The content version changes with the data, as for any pin.

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

**`exclusions.toml`** holds what the import left out. It has three kinds of row:

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

When a later change ships a record that an `excluded` or `rejected` row left out, do not edit that
row. Append a `resolved` row after the `rejected` rows:

```toml
[[resolved]]
path = "json/spells2.json#breeze"
entries = ["ref/spell/breeze"]
reason = "Ships from the Ultimate Cantrips blog post. Refs #36."
```

`path` is the exact `path` of the `excluded` or `rejected` row it supersedes. `entries` lists the
entry ids that now hold the record (at least one; each must exist), and `reason` says why the
exclusion no longer applies and cites the issue. The newest word on a path wins: the coverage
report moves the row out of `excluded` or `rejected` into its `resolved` list (path, the kind of
the old row, entry ids, reason) and counts the record as shipped. The old row stays in the file as
history. Appending the `resolved` rows after the other rows is a convention. The reader checks the
whole file, so a `resolved` row finds its row wherever it stands.

A `resolved` row supersedes exactly one row. Several rows can share a `path`: an `excluded` and a
`rejected` row (the Death and Repose domains have both), or two `rejected` rows with different
reasons (`json/main04.json#traits_race` has two). A path that more than one row has, across both
tables, needs `supersedes`: the full `reason` of the row it resolves, copied exactly (string
equality, not a prefix):

```toml
[[resolved]]
path = "json/class_ability_domains.json#death"
supersedes = "converter native exclusion (ref/domain/death): <the rest of the reason>"
entries = ["ref/domain/death"]
reason = "The domain ships since the @list directive reads. Refs #103."
```

The selection rule. N is the number of rows, in both tables, whose `path` is the `path` of the
`resolved` row:

| N | `supersedes` | `kind` | Result |
|---|---|---|---|
| 0 | any | any | Error: supersedes nothing |
| 1 | none | none | The row (unique-path shorthand) |
| 1 | none | its table | The row |
| 1 | none | the other table | Error: supersedes nothing |
| 1 | its exact reason | none, or its table | The row |
| 1 | its exact reason | the other table | Error: supersedes nothing; the message says the reason exists in the other table |
| 1 | not its exact reason | any | Error: supersedes nothing |
| 2 or more | none | any (none, `excluded`, `rejected`) | Error: ambiguous, add `supersedes` (`kind` alone never selects a row on a shared path) |
| 2 or more | the exact reason of one row | none, or that row's table | That row |
| 2 or more | the exact reason of one row in the other table than `kind` | a table | Error: supersedes nothing; the message says the reason exists in the other table |
| 2 or more | no row's exact reason | any | Error: supersedes nothing |
| 2 or more | the exact reason of a row in each table | none | Error: ambiguous, add `kind` |
| 2 or more | the exact reason of a row in each table | a table | The row of that table |

"Exact reason" is string equality with the whole `reason`, including a blank or whitespace-only
reason: a row whose `reason` is `""` or `"  "` is selected by `supersedes = ""` or `supersedes =
"  "`. A `supersedes` that is only a start of a reason selects nothing. A row that another
`resolved` row already supersedes is a second error: duplicate resolved row.

The identity of a row is its table, its `path` and its `reason`; the reader refuses a repeated
`path` and `reason` within one table. `supersedes` with `path` therefore selects one row, unless an
`excluded` and a `rejected` row have the same `path` and `reason`. Then add `kind = "excluded"` or
`kind = "rejected"` to name the table. A `kind` is never needed otherwise, and a row may carry it
anyway when the path is unique or `supersedes` is given. A reason that is the start of another
reason on the same path (`unsupported directive` and `unsupported directive @list`) needs nothing
more: each full reason selects its own row.

The reader matches each `resolved` row against the whole file, not only the rows above it. So an
`excluded` or `rejected` row appended later under a path that already has a `resolved` row can
make that resolution ambiguous (for example, it then needs `supersedes` or `kind`), and
`corpus-check` fails. Do not add a row under a resolved path: a resolved record that must be
excluded again gets an `excluded` row under a new path (see below).

The other rows with the path stay in the report. To resolve several rows that share a path, append
one `resolved` row for each, each with its own `supersedes`. A `path` that two rows share needs one
`resolved` row for each of them: the Death and Repose domains need two (the converter `rejected`
row and the Pharasma-friendly blog `excluded` row). When only one row has the path, leave
`supersedes` out. The keys are optional and additive: a file with no `resolved` row reads and
writes back byte for byte as before, a report with no `resolved` row has no `resolved` key and no
"Resolved records" section, and the corpus format does not change. A row is resolved once; to
exclude a record again, add an `excluded` row under a new path and say why in a decision.

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

#### Footnotes

A footnote is a level-six heading, not a separate block. The body has no
footnote block, and only a heading carries an anchor that a body link can name.
A note that a reference links to therefore takes a heading, and level six is the
deepest level, so the note sits below the section text. A new block type would
change the serialized body and need a `SCHEMA_VERSION` increase, so the current
format keeps the heading (#137).

```markdown
- [Homunculus](ref/monster/homunculus)^[5](ref/ability/improved-familiars#footnote-h)^

###### ^5^ {#footnote-h}

The master must first create the homunculus.
```

The superscript holds the link, so the marker is small and still navigable. The
heading text is the marker, usually a number or a letter. The paragraph after
the heading is the note. A plain superscript such as `^B^` marks a note without
linking it. The link target is the entry's own id, not a bare fragment. The
`footnote-` prefix is a convention; the reader treats the anchor like any other
heading anchor. This example is from Improved Familiars (Core Rulebook).

Keep the `{#footnote-h}` anchor. Body links name it, and `corpus-check` fails
when a link anchor is not a heading key in the target entry. Editing the note
text does not move the anchor. If you change the marker label, change each
reference label to match and leave the anchor alone. Changing the anchor breaks
every inbound link, so treat that edit as a data change.

Do not use Markdown footnote syntax such as `[^5]` or `[^5]: ...`. The reader
accepts neither.

`corpus-check` fails in two cases:

- a footnote heading (level six, anchor starting `footnote-`) has no marker in the
  entry body. A marker is a superscript with the heading label (`^B^`) or a link
  to the anchor (`^[1](id#footnote-1)^`). The error names the line of the
  heading. An orphaned note reads as a stray paragraph, so add the marker or
  remove the note (`docs/research/editorial-footnotes-audit.md`, "Orphaned
  markers").
- the body holds converter residue such as `\|X~`, a pipe after a backslash, one
  to three letters or digits, and a tilde. The tilde may be escaped, as in
  `\|X\~`. The importer left it where it did not
  split a multi-note `::mfn`. Split it into one note per marker.

A footnote heading is still a heading. The parser treats it as one, the renderer
emits `<h6>`, and when the page shows its "On this page" list, the list includes
the note with every other heading. Use the short marker, the
`footnote-` anchor, and the position after the passage to tell a note from an
article section. Some deeply nested skill sections already use level six, so the
level alone does not prove a heading is a footnote. Give a section a title, not
a marker, and give it no `footnote-` anchor.

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
- the body holds a left-over `p!` import marker, which the upstream import wrote where a
  paragraph or list break had been. The same letters inside a word, as in `Help!`, are real text;
- a creature entry holds a left-over spell-list marker, `&Q`, `&EM`, `&EX` or `&S`, in a spell-list
  item. The upstream import wrote them for the metamagic and summon entries of monster
  spell-like ability lists; write `quickened`, `empowered`, `extended` or `summon (level N, ...)`,
  as the books print them. A spell-list item is an item of a bulleted or numbered list under a
  `**Spell-Like Abilities**`, `**Spells Known**`, `**Prepared Spells**`, `**Psychic Magic**` or
  `**Spellcasting**` heading, up to the next heading. It holds the lines that the corpus reader
  gives the item: the indented lines after it, a nested list, and the text after a blank line
  when the next line is indented. The check skips code spans, link targets, bare URLs (they may
  hold balanced parentheses and end at whitespace), URI and email autolinks, and escaped text
  (`\&Q`). It does not read a longer word (`&Sword`, `&Séance`) or the upstream token `&Q&`
  (reported as a token) as a marker, and `<&Q>` is plain text, so it counts;
- a `source_id` is not in `publications.toml`;
- a `license_ref` is not in `legal.toml` and is not `legal#community-use`;
- a parent, body link, prerequisite feat, relation target, book node, or alias
  target names an unknown entry;
- a relation carries a label outside its kind's closed vocabulary, a `variant` is
  not spell to spell, an entry holds more than one `variant`, or `variant` edges
  form a cycle;
- a `revision` points at a revision or at another kind, shares a book with its
  target, repeats a target in one publication, is named by a body link or
  `prerequisite_feats`, or is `removed`/`merged` with non-empty fields or a
  `merged` body that links no feat;
- a `mythic` add-on is not a feat whose `feat_types` include `Mythic`;
- a `class_option` names a group missing from `option-groups.toml`, a declared
  group names a class that does not resolve, or a group holds no option;
- an add-on or revision does not take the id suffix its publication declares, or
  an id under an existing entry ends in a known suffix but holds no matching edge;
- two publications share a suffix, or a suffix is not one id segment;
- a publication is marked `third_party = true` but its copyright line names Paizo;
- a publication's `series_kind` is not one of `adventure-path`, `module` or `society`, or a
  series field is missing or present against its kind (see "Series fields" above);
- a body link or a `books.toml` anchor is not a heading key of the target entry
  (the message names the linking file, the target file and its headings, and
  the fix);
- a `books.toml` node places an entry in a book that is not in the entry's `source_ids`. This holds
  for every node with an entry, in a printed chapter, a catch-all, `Page order` or `Unpaged`
  chapter alike. The message names the line, the book, the node title, its top-level chapter, the entry and its
  `source_ids`. Remove the placement, or add the book to `source_ids` when it prints the entry;
- a `[[books]]` block has no row in `publications.toml`. A book is a printed publication, so a
  web post or comic issue gets no book;
- `exclusions.toml` repeats a row, resolves one row twice, has a `resolved` row that matches no
  `excluded` or `rejected` row of the file (an orphan), has one on a path with several rows but no
  `supersedes` (`kind` alone is not enough), or `kind` is missing for a path and reason that both
  tables hold (ambiguous), names a `supersedes` that is not the exact full reason of a row, leaves `entries` or `reason` empty, or names an
  entry that does not exist;
- a publication in `publications.toml` has no `copyright`, or is covered by neither a
  `section_15` notice in `legal.toml` (the notice text is part of the publication's
  `copyright`) nor a title in `notice_pending`. The message names `publications.toml`, the
  line, and the publication id. This is the licensing gate for adding a book: its
  Section 15 notice, or a recorded pending decision, goes in the same change.

`corpus-check` also **warns**, and still succeeds, when third-party content can escape its
book:

- a body link in an entry with no third-party source targets an entry whose `source_ids`
  are all third-party;
- an entry mixes third-party and Paizo `source_ids`.

The second rule is a heuristic. The corpus has no per-paragraph source attribution. The
split form is an `addon` or `revision` entry that carries only the third-party source beside
an unchanged Paizo base. A mixed entry is either a missed split or a shared-printing claim
that a reviewer must confirm. The warnings stay non-failing while genuine shared printings
remain; a clean corpus can promote them to errors.

`corpus-check` also **warns** once per run about book memberships that lack a structured citation
(#461): an entry lists a book in `source_ids` other than its first, and the entry has no
`[[sources]]` row for that book. A row without a `page` satisfies the rule, so the rule asks for
the row, not for a page. The first book is the entry's home and needs no row. Every later book
claims that it prints the entry or a section of its body, and the `[[sources]]` row is the
evidence that the check can read. A body `**Sources:**` line does not count, even when someone
verified it against print: a reviewer converts a verified body citation to a `[[sources]]` row. An
[`expanded_in`](#expanded_in) credit sits outside membership and does not count either. The warning
gives the number of memberships and of entries, and three examples, each as
`path:line: entry id in book` with the line of `source_ids`. It measures what remains of the
cleanup of memberships that a book does not print: each batch either adds the row (the book prints
the entry), removes the book, or, for a book that only expands the entry, moves it to
`expanded_in`. When the count reaches 0 the warning becomes an error, as the footnote findings did
(#531).

## Drift between fields and body

Some values live twice in an entry: in `[fields]`, and in the body. `corpus-check`
compares each pair that states a value in both places and **fails** on every
disagreement, which names both places:

```text
entries/feat/accursed-hex.md:12: [fields] prerequisites = "Accursed Hex." disagrees with the `**Prerequisites:**` line = "Hex class feature." (line 20)
```

It prints the count per kind and every disagreement. The full list is not stored in the
repository; print it with `--drift-report`:

```sh
cargo xtask corpus-check .cache/sources/cast-iron-data --drift-report /tmp/corpus-drift.md
```

The file holds the count per kind, then one line per disagreement. The check is an
error: the pinned corpus `v0.1.2` has 0 disagreements, and `corpus-check` and
`cargo xtask content` exit non-zero on the first new one. Fix the wrong copy.

| Kind | Typed field | Body |
|---|---|---|
| spell | `school`, `subschools`, `descriptors`, `levels`, `casting_time`, `components`, `range`, `effects`, `duration`, `saving_throw`, `spell_resistance` | the matching rows of the stat block (`School`, `Level`, `Range`, `Target`/`Effect`/`Area`, ...) |
| creature | `cr`, `creature_type`, `size`, `alignment`, `ac`, `hp`, `fortitude`, `reflex`, `will`, `melee`, `ranged`, `special_attacks`, `special_qualities`, `abilities` | the `CR`, `Type`, `Size`, `AC`, `hp`, `Fort`, ... rows |
| item | `price`, `weight`, `slot`, `aura`, `caster_level` | the `Price`, `Weight`, `Slot`, `Aura`, `CL` rows |
| feat | `prerequisites`, `benefit`, `normal`, `special` | the bold `**Prerequisites:**`, `**Benefit:**`, `**Normal:**`, `**Special:**` lines |
| archetype | the `replaces` and `alters` trades | every bold `**Replaces:**` and `**Alters:**` line, in body order |
| class_option | the `replaces` and `alters` trades | every bold `**Replaces:**` and `**Alters:**` line, in body order |

The feature of a trade has no second copy in the body. It is checked against the class entry's
heading anchors, and a trade warns when its printed name resolves confidently to another anchor
(see [Trades](#trades)). The warning does not fail the check, because a per-power trade or a
level-qualified name resolves by hand.

The comparison reads both sides as words: it ignores case, repeated blanks, an
end period, `*` and `[label](target)` markers, and a few entities (`&times;`). A
spell's `School` row may add the subschool and descriptors. A feat field agrees
when any `**Label:**` line in the body says the same, because a feat with a
mythic variant states the label twice. For the `replaces` and `alters` trades, an entry may
print several lines, one per traded power; the printed names of the trades with that key are the concatenation
of every line in body order, and a comma-separated line contributes one item per
part. A value stated in only one place is not drift. The two copies are not
unified: the typed fields feed search and facets, and the body is what a reader
sees.

## Class-option powers

A class-option power (a school power, revelation, hex, curse, spirit ability or domain power)
is a bold name paragraph followed by labeled rows. The decision
[`2026-11-20-class-option-power-layout`](decisions/2026-11-20-class-option-power-layout.md)
fixes one canonical order for the rows, so that the app can show the power as printed prose
without reordering anything:

```markdown
**Resistance (Ex)**

**Gained:** At 1st Level

**Passive:** You gain resistance 5 to an energy type of your choice.

**At 11th level,** this resistance increases to 10.
```

The order is: the name (one bold span, no colon); an optional `Gained` row (`At Nth Level`); an
optional `Flavor` row; exactly one effect row (`Passive`, `Ability`, `Standard`, `Move`, `Swift`,
`Immediate`, `Free`, `Full-round` or `Benefit`); the effect's continuation, as unlabeled
paragraphs directly after the effect row and nowhere else; an optional `Usage` row; the upgrades,
each `**At Nth level,** sentence` in strictly ascending level and above `Gained`; an optional
`Prereq` row, then an optional `Replaces` row; every other row in its source order. Every row has
content. A legacy label (`Action`, `Fullround`, `Passive Ability`, `Standard Action`, a numbered
label such as `Ability2`, or `At Nth Level:`) and a plain-text upgrade (`At 8th level: ...`) are not
canonical. The text and the order are the only change from the old import: the files keep the same
syntax, so this is not a corpus format change.

`corpus-check` runs the shared classifier and validator
(`cast_iron_schema::power`) on every entry of kind `class_option`. A *candidate* is a name
directly followed by a row, a lead or a legacy upgrade, with a power marker in the run that
extends to the next boundary or name. A boundary is a heading, list, table, quote, stat block,
entry link or styled paragraph (`statline` or `indented`). Each candidate that fails
validation is reported as a **warning** with its path and line, the entry id, the power name and
every reason:

```text
warning: entries/hex/evil-eye.md:15: [powers] entry `ref/hex/evil-eye`: the power `Evil Eye (Su)` is not in the canonical form: `Flavor` is out of order [out-of-order]; the legacy label `At 8th Level:` [legacy-label]
```

A last warning counts the failing powers and the entries that hold a power. The rule stays a
warning until the corpus repository has rewritten its class options (phynics/cast-iron-data#88).
The app pull request that pins the rewritten corpus turns it into an error, except for the powers
that #88 still queues for print review in its resolutions file.

### Class-option residue

The retired upstream import left eight defects in class-option entries. The corpus repository fixed
them (phynics/cast-iron-data#95), and `corpus-check` now fails when one comes back. Each is an error
that names the path, line, entry id and defect slug:

| Slug | What it is |
|---|---|
| `skill-code` | A `Gained class skills:` or `Lost class skills:` value that is an upstream skill code (`kd`, `stl`, `perf\|dance`) |
| `spell-no-level` | A `Spells:` row that does not end in its granted level, such as `(3rd)` |
| `spell-order` | A run of `Spells:` and `Cantrips:` rows whose stated levels are not ascending |
| `key-tilde` | A `**Key:** ~` row: the import's line-break marker |
| `misplaced-title` | A `Title:` row directly after a `Details:` row |
| `bonus-feats-glue` | A `Details:` row followed by a `Title:` row whose feat links have no separator |
| `import-key` | An `Id:`, `From:` or `Save: domain` row |
| `glued-modifier` | A `Usage:` row such as `Charisma3 modifier times/day` |

Only top-level paragraphs of `class_option` entries are scanned. A literal `~` inside prose and a
numbered effect label such as `Ability2` are not residue guards: the first is report-only in the
corpus survey, and the power validator above reports the second.

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

A TOML file (`corpus.toml`, `publications.toml`, `option-groups.toml`, `books.toml`, `legal.toml`,
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
five commands: `check [DIR] [--drift-report FILE]`, `fmt [--check] [DIR]`,
`migrate [DIR]`, `migrate-trades [DIR] [--review FILE]`, and `migrate-layout [DIR]`. It depends only on the
corpus and schema crates. The same code runs behind `cargo xtask corpus-check`,
`corpus-fmt`, `corpus-migrate`, `corpus-migrate-trades`, and `corpus-migrate-layout`.

## Determinism

The writer is deterministic: the same `RuleEntry` gives byte-identical text. It uses a
fixed key order, fixed block separation, no timestamps, and no hash-map iteration, so
`fmt` rewrites a file to the same bytes every time.


### Per-publication legal basis

`legal.toml` may include `[[publication_licenses]]` rows with a publication id, exact title, and license references. These rows record reviewed exceptions to the legacy OGL default and are additive for older readers. A publication without a row remains OGL-based. Paizo blog text used under the Community Use Policy must name `legal#community-use`; when a publication also contains OGL material, it must name both. OGL-based publications still require their exact Section 15 notice or a reviewed `notice_pending` record. The compiler checks row uniqueness, title and publication references, and license references.
