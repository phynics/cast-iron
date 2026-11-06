# Editing the corpus

This directory is the contributor kit. It is copied into the `cast-iron-data` repository,
where it sits next to the corpus. Everything here works with the `cast-iron-corpus` tool
and a text editor; you do not need the Cast Iron app, a browser, or a WebAssembly
toolchain.

The corpus is one Markdown file per rules entry (`entries/...`) and a few TOML files. The
full specification is `docs/corpus-format.md` in the Cast Iron repository. This page is the
short version: three rules, the tool, and one recipe for each common edit.

## The three rules

1. **Enter inside a paragraph is a space.** A paragraph may wrap over many lines. A blank
   line starts a new paragraph. A backslash at the end of a line is a line break.
2. **Never change an `id`, and never move a file.** An id is permanent, because readers'
   bookmarks store it. The path is the id without `ref/`.
3. **Run the tool before you commit.** `cast-iron-corpus check` finds broken references, and
   `cast-iron-corpus fmt` puts every file in its one canonical form. A pull request must pass
   both.

## The tool

Install it once, from the revision this corpus pins (see `.github/workflows/corpus.yml`):

```sh
cargo install --locked --git https://github.com/phynics/cast-iron --rev <REVISION> cast-iron-corpus
```

Then, from the root of this repository:

```sh
cast-iron-corpus check .          # read every file; list broken links, ids, sources
cast-iron-corpus fmt .            # rewrite files into canonical form
cast-iron-corpus fmt --check .    # change nothing; name the first line that is not canonical
```

`check` fails with a message like `entries/rule/ambush.md:14: unknown link target
`ref/skill/stealh``. The message names the file and line. It also lists fields that disagree
with the body as warnings; those do not fail the run.

`fmt` is safe to run at any time. It re-wraps paragraphs, trims table cells, renumbers lists
to `1.`, and adds a `{#anchor}` to a heading that a link reaches. Run it after every edit,
then look at the diff.

## Recipes

### Fix a typo

Open the entry, fix the word, save. If the same text also appears in the front matter
(`benefit = "..."` for a feat, or a stat row in the body), fix both copies:
`check` warns when they disagree.

```diff
-A bank of fog billows out from the point you desginate.
+A bank of fog billows out from the point you designate.
```

Then `cast-iron-corpus fmt .` (it re-wraps the paragraph if the line length changed) and
`cast-iron-corpus check .`.

### Fix a link

A link is `[text](ref/...)`. The part in parentheses is the permanent id of the target entry,
which is its path without `entries/` and `.md`, plus an optional `#anchor` for a heading.

```diff
-See [darkvision](ref/rule/darkvison) for details.
+See [darkvision](ref/rule/darkvision) for details.
```

To find the right id, look for the file: `entries/rule/darkvision.md` is `ref/rule/darkvision`.
A link to a heading looks like `[fog](ref/spell/fog-cloud#effects)`; the heading in
`entries/spell/fog-cloud.md` must be written `## Effects {#effects}`. If `check` says a heading
does not exist, it names the target file and lists the headings that do.

### Split a paragraph

Put a blank line where the paragraph should end:

```diff
 You can make exceptionally deadly melee attacks by sacrificing accuracy for strength.
-This bonus to damage is increased by half if you use a two-handed weapon.
+
+This bonus to damage is increased by half if you use a two-handed weapon.
```

To start a new line without a new paragraph, end the line with a backslash:

```markdown
**Prerequisites:** Str 13, base attack bonus +1.\
**Benefit:** You can choose to take a -1 penalty on all melee attack rolls.
```

### Add a table row

A simple table is a header row, a `| --- |` row, and body rows. Add a row with the same number
of cells as the header. Spaces around cells do not matter; `fmt` removes them.

```diff
 |Distance|Check|
 | --- | --- |
 |10 feet|DC 10|
+|20 feet|DC 12|
 |30 feet|DC 15|
```

A literal `|` in a cell is written `\|`. A table with a caption, merged cells, or a header
column is written as a `::: table` block; see `examples/rule.md` and the format document.

### Change a stat value

A stat appears in the typed front matter (search and filters read it) and in the stat block
(readers see it). Change both:

```diff
 [fields]
 kind = "creature"
-ac = "13, touch 12, flat-footed 12 (+1 Dex, +1 natural, +1 size)"
+ac = "14, touch 12, flat-footed 13 (+1 Dex, +2 natural, +1 size)"
```

```diff
 ::: statblock
-AC: 13, touch 12, flat-footed 12 (+1 Dex, +1 natural, +1 size)
+AC: 14, touch 12, flat-footed 13 (+1 Dex, +2 natural, +1 size)
 :::
```

`check` prints a drift warning with both line numbers if you change only one.

### Add an entry

1. Copy the example for the entry's kind from `examples/` (there is one per kind).
2. Save it as `entries/<kind folder>/<slug>.md`. The id is `ref/` plus that path without
   `entries/` and `.md`. For `entries/spell/spark-needle.md` the id is `ref/spell/spark-needle`.
3. Set `id`, `kind`, `title`, and `source_ids`. Every source id must be listed in
   `publications.toml`.
4. Delete the comment lines (the lines that start with `#` in the front matter). Do not add a
   `provenance` key and do not add `license_refs`: the defaults in `corpus.toml` apply.
5. Write the body. Link other entries with `[text](ref/...)`.
6. Run `cast-iron-corpus fmt .`, then `cast-iron-corpus check .`.

A feat's prerequisites are `prerequisites = "..."` (the words) and `prerequisite_feats = [...]`
(the ids of the feats it names). The prerequisite relations are derived from that list; do not
write them in `relations`.

## What not to edit

- `history.toml` holds frozen results of the original import. Do not edit it.
- `exclusions.toml` is append-only. Add a row; never edit or remove one.
- A `provenance` key in an existing entry is history. Leave it as it is.
- Do not put HTML, HTML comments, or carriage returns (Windows line ends) in a file. The tool
  rejects them. A literal `<` in text is written `\<`.

## When the format changes

`corpus.toml` says which format the corpus is in. A newer tool refuses an older corpus and
names the command: `cast-iron-corpus migrate`. A format change is applied as one mechanical
pull request that changes nothing else, followed by `fmt`.
