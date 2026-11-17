---
title: "Plain-text corpus, step 1"
date: 2026-11-06
status: current
superseded_by: []
issues: [127]
prs: []
area: corpus
---

# Plain-text corpus, step 1, 2026-11-06

Refs [#127](https://github.com/phynics/cast-iron/issues/127). The corpus becomes the hand-edited source of truth in a public data repository, and upstream is never re-imported. Two design reviews shaped the format below.

- **The corpus is one Markdown file per entry and a few TOML source files.** Entry paths come from the permanent `ref/` id, so `ref/spell/fog-cloud` is `entries/spell/fog-cloud.md`. `corpus.toml` (format version and defaults), `publications.toml`, `books.toml`, `legal.toml`, `exclusions.toml`, `history.toml`, and `aliases.toml` hold the rest. The format is specified in `docs/corpus-format.md`.
- **Derive before storing, except what the import decided.** `relations.json`, `lists.json`, `backlinks.json` and its shards, `search.bin`, the manifest, catalogue stubs, `entry_count`, and every coverage count are compiler output. A frozen coverage file went stale on the first edit, so the stored part is only `exclusions.toml` (excluded paths and rejected records with reasons, append-only) and `history.toml` (the upstream digest, upstream record counts, how prerequisite wording was read, parts left unplaced, how page numbers placed entries in books; never edited). The round trip rebuilds the whole coverage report from the entries and those two files and requires it to equal `coverage.json`.
- **Text is for people.** Paragraphs wrap at about 100 columns; Enter is a space, a blank line is a new block, and a trailing backslash is a line break. A reader trims blanks around every line, heading, and table cell, so a wrapped and an unwrapped paragraph mean the same. `\0` and `\e` are gone: the writer merges adjacent text and drops empty text, and the canonical compile does the same, because three compiled entries carried adjacent text nodes.
- **A link must survive an edit to a heading.** Every heading that a body link or a book node reaches carries an explicit `{#anchor}` equal to the key it already had, so URLs do not change. The compiler writes them (one heading in the current library); `fmt` adds one when an edit links to a heading without one. The key algorithm moved to the schema crate so the client, the compiler, and the tools share it.
- **Defaults are declared once.** `license_refs` is written only when it differs from `corpus.toml`. A feat's prerequisite relations are derived from `prerequisite_feats`; the 174 relations labelled `or` stay stored because the list cannot state a label. `provenance` stays optional and is frozen history for new entries to leave out. `validate_library` still requires provenance, so the step that compiles from the corpus must relax that rule for entries without it.
- **One canonical form, enforced.** `fmt` rewrites every file; `fmt --check` prints the first differing line of each file that `write(read(file))` would change. The reader rejects HTML comments, raw HTML, and carriage returns. TOML files carry no comments, because the tool rewrites them.
- **The format has a version and a migration rule.** The reader refuses an unknown `format` and names `cast-iron-corpus migrate`. A format change ships a migration step and is applied as one mechanical pull request, then `fmt`.
- **Fields and body are not unified yet.** Some values live twice, in the typed fields and in the body stat block or labelled lines. `corpus-check` compares the pairs and fails on every disagreement (0 in corpus `v0.1.2`; print the full list with `--drift-report`). It was a warning only while the freeze data still held disagreements.
- **Contributors need only Rust and a text editor.** `corpus-kit/` is copied into the data repository: a README with one recipe per common edit, one annotated example per entry kind, a workflow, and a pull request template. The `cast-iron-corpus` binary depends on the corpus and schema crates only.
- **Step 1 does not switch the build.** `corpus-export` reads the compiled library and writes the text; the compiled library changes only where the format required it (adjacent text nodes in three entries, one explicit heading anchor). Populating the data repository and pinning it in the build stay in later steps.
