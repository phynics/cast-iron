---
title: "Archives of Nethys links are optional stored paths"
date: 2026-11-09
status: current
superseded_by: []
issues: [197]
prs: []
area: corpus
---

# Archives of Nethys links are optional stored paths, 2026-11-09

Refs [#197](https://github.com/phynics/cast-iron/issues/197). The harvest and data change are a
separate pull request in `phynics/cast-iron-data`; app rendering is a later step.

- **`aon` is one optional string on an entry and one on a publication.** It holds a path
  relative to `https://aonprd.com/`, such as `FeatDisplay.aspx?ItemName=Power%20Attack` or
  `SourceDisplay.aspx?FixedSource=Core%20Rulebook`. The corpus file never stores a scheme,
  host, leading slash, or fragment, so one base URL decides where the link opens.
- **Only reviewed matches are stored.** The harvest runs by hand over AoN's bulk listings and
  writes the field only for a confident kind-plus-normalized-title match. A same-name or
  variant record without a distinct AoN page gets no field, and every miss stays in the
  harvest report. The app never guesses a URL.
- **`corpus-check` owns the shape.** An entry path must name a `*Display.aspx` page with
  `ItemName`, `FixedName`, or `FinalName`; AoN uses `ItemName` for most records, `FixedName`
  for archetypes, and `FinalName` for some item pages. A publication path must name
  `SourceDisplay.aspx` with `FixedSource`. Values are percent-encoded and the query key is
  fixed, so a host, a missing escape, or another query cannot enter the corpus.
- **`SCHEMA_VERSION` stays 4 and the corpus format stays 1.** Both fields are optional and
  omitted when absent; an older reader ignores the unknown key. The tool that reads the corpus
  ships with the corpus pin, as with the `[[sources]]` migration (#357). The compiler carries
  the publication field into the optional `SourceInfo.aon` member of `catalog.json`, and the
  entry field into the serialized `RuleEntry`.
- **The app does not fetch before the tap.** Rendering is a later step, but the stored value is
  already display data: the link is a plain anchor to the fixed AoN base and implies no
  official status.
