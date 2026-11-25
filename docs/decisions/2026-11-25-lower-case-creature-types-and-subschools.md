---
title: "Creature types and subschools are lower-case printed words; the app accepts both spellings during the transition"
date: 2026-11-25
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Creature types and subschools are lower-case printed words, 2026-11-25

The owner ruled on 2026-11-25 on phynics/cast-iron-data#287:

1. `creature_type` holds the printed words in lower case: `magical beast` and `monstrous humanoid`
   replace the identifier spellings `magicalBeast` and `monstrousHumanoid`. The other 11 types are
   single words and do not change.
2. A spell's `subschools` are lower case, like `descriptors`, as print sets them in the school
   line (`conjuration (calling) [fire]`). This changes 1,074 capitalised values.

## Staging

The data repository builds `corpus-check` from app `main`, and app `main` compiles the pinned
corpus, which still holds the old spellings. So the change lands in two app steps:

1. **Transition (this entry).** `corpus-check` accepts both spellings of the two creature types:
   `CREATURE_TYPES` lists the lower-case words and `LEGACY_CREATURE_TYPES` the identifiers. The
   client labels both as `Magical Beast` and `Monstrous Humanoid`. No code reads the case of a
   subschool: the drift check normalises case on both sides, as for `descriptors`, no facet holds
   a subschool, and no view prints one.
2. **Removal.** The pull request that pins the corpus tag holding #287 deletes
   `LEGACY_CREATURE_TYPES` and its use in the check, and the two `facet_label` arms for the
   identifier spellings. From then on an identifier spelling is an error.

## Saved state

The app keeps no creature type in a bookmark (entry ids only). A recent search stores its facet
values in the device's `meta` store, and a link in `#/browse/kind/creature?facet=creature_type:…`
carries one. After the pin moves, a recent search or a link saved with `magicalBeast` or
`monstrousHumanoid` lists no creature, until the reader clears or repeats it. The recent list holds
10 entries and the link form is the filter menu's own output, so no migration is built.
