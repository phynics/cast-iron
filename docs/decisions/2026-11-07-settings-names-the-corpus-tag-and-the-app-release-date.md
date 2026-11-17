---
title: "Settings names the corpus tag and the app release date"
date: 2026-11-07
status: current
superseded_by: []
issues: [258]
prs: []
area: install-offline
---

# Settings names the corpus tag and the app release date, 2026-11-07

Refs [#258](https://github.com/phynics/cast-iron/issues/258).

- **The tag is additive and optional.** `ContentManifest` gains `corpus_tag`, filled from
  `sources/corpus.toml` by the compiler. An old library has no field and reads as `None`; an
  old reader ignores the field, because the manifest does not use `deny_unknown_fields`.
  `SCHEMA_VERSION` stays 4: display information is not a breaking change.
- **The app date is the commit date.** `cargo xtask build-site` stamps
  `git log -1 --format=%cs HEAD` next to `SHELL_VERSION`. A build without git leaves it empty.
  The wall clock is never read, so the same commit gives the same `sw.js`.
- **Settings groups the version facts.** A "This app" section holds Corpus, App, Content
  format and Installed; Library keeps Entries and Size, and App keeps the install offer.
  Hashes stay in parentheses, shortened to 8 hex characters, and dates use the fixed English
  month table in `fmt.rs` rather than the browser locale. Updates still move first while one
  waits (#206).
