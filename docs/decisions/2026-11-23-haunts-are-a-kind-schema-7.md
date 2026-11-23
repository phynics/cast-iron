---
title: "Haunts are their own kind, and SCHEMA_VERSION is 7"
date: 2026-11-23
status: current
superseded_by: []
issues: [618]
prs: []
area: content
---

# Haunts are their own kind, and SCHEMA_VERSION is 7, 2026-11-23

Refs [#618](https://github.com/phynics/cast-iron/issues/618). The owner chose a new kind over a `trap`
with a type value (see [v1 scope: owner decisions](2026-11-23-v1-scope-owner-decisions.md)). This entry
records what the app change does and why the format number moves.

## What changes

- `EntryKind::Haunt` has the slug `haunt` and the label `Haunts`. It has no typed fields, like traps: the
  CR and the other rows stay in the body.
- The enum declares `Haunt` last, so the derived order of the older kinds does not move. `EntryKind::ALL`
  lists it directly after `Trap`, and every kind order of the app follows `ALL`: the Kind menu, the Home
  hub (`Haunts and how to destroy them`, in the `More` group) and the `In this book` links.
- Haunts leave book Contents like every other listed kind (`linked_kind` is true for every kind except
  Rules). A book gets a `Haunts N` link instead.
- This change adds no haunt data. The 45 `ref/haunt/*` entries keep `kind = "rule"` until the corpus
  change sets `kind = "haunt"` by script. No id changes and no file moves, because the reader ties an id
  to its path and not to the kind.

## Why 7, and why 1 to 7 stay readable

An older client fails on a kind value that it does not know, so a library with a `haunt` entry is not
readable by it. Rule 4 therefore requires `SCHEMA_VERSION` 7. A schema-7 manifest makes an older shell
refuse the library before it downloads anything (rule 7), and the active library stays usable.

The new shell reads every older schema (`READABLE_SCHEMAS` is 1 to 7), because nothing else in the format
changes: an installed schema-6 library opens as it is, and its 45 haunts still show as rules until the
user installs the library with the new kind.

A star made on a haunt while it was a rule stores `rule` as its kind snapshot. `refresh_bookmark_kinds`
rewrites the snapshot to `haunt` after the upgrade, the way it did for class options in schema 4. The
star's id does not change.

## What does not change

- `INDEX_FORMAT_VERSION` stays. The search index stores the kind of a document as a plain string, so a
  new slug needs no index format change.
- The corpus format number stays. `kind = "haunt"` is one more value of an existing field: the corpus
  reader accepts it once this change is in, and `corpus-check` validates it. The contributor kit gets
  `examples/haunt.md` in this change; the data PR then sets the kind of the 45 entries.

## Release window

Between this change and the pin that carries the haunt data, `main` builds a schema-7 library with no
haunts. Do not promote `main` in that window: the release would force every installed library through a
schema upgrade and show nothing new. The promotion after the pin needs the upgrade check of
[releasing.md](../releasing.md) (old library, new shell, new library; an interrupted download; stars),
in Chromium and WebKit. This change needs a full CI run on its branch before merge, because it changes a
format version.
