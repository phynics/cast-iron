---
title: "The drift check reads creature and item statistics from stat-block rows"
date: 2026-11-23
status: current
superseded_by: []
issues: [254]
prs: []
area: corpus
---

# The drift check reads creature and item statistics from stat-block rows, 2026-11-23

Refs [#254](https://github.com/phynics/cast-iron/issues/254) and phynics/cast-iron-data#166. The data pass
of #166 writes the statistics of creatures and items as stat blocks (a `Challenge rating` row, `Armor class`,
`Touch` and `Flat-footed` rows, and so on) instead of bold-label paragraphs. The drift check
(`crates/corpus/src/drift.rs`) must read both forms before that pass lands, because the data repository
builds its checker from app `main`. Without the rules below, the pass would report about 1,651 false drifts
and would silently stop comparing `cr` for about 2,800 monsters. The check is native only, so the change
costs no WASM and changes no output: the manifest hash of the pinned corpus is the same.

## Decision

- **Challenge rating.** A `Challenge rating` row compares with `[fields] cr` by value (`ChallengeRating::parse`),
  like the existing `**Challenge rating:**` paragraph and the `CR` row. `1/2` agrees with `1/2`.
- **Type and subtypes.** A creature that has a `Subtype` row prints the type alone in `Type`: `Type` compares
  with `creature_type`, and `Subtype` compares with `[fields] subtypes` as two sets of words, so order and case do not
  matter. A creature with no `Subtype` row keeps the old rule: `Type` may add the subtypes in parentheses. A field with
  no subtypes leaves a `Subtype` row uncompared.
- **Armor class.** The `Armor class`, `Touch` and `Flat-footed` rows join to `a/t/f` and compare with `ac`. The
  `AC modifiers` row is not compared. One or two of the three rows state nothing about the whole value, so they
  are not drift.
- **Hit points.** The `hp` row, or the `Hit points` row, compares with `hp`, where `~` reads as ` ( )`: `103~9d12+45`
  agrees with `103 (9d12+45)`. A field with no `~` compares as it is.
- **Ability scores.** The comparison takes the text after each ability name, so `Str - (22 while corporeal)` agrees
  with the field value `- (22 while corporeal)`. It took the last word before, so a score with a qualifier failed.
- **Item slot.** The corpus holds a closed `slot` value and the body keeps the printed one (decision
  [affliction types and item slots are closed vocabularies](2026-11-22-affliction-types-and-item-slots-are-closed-vocabularies.md)).
  The `Slot` row agrees with the field when it states the field itself, or when it maps to the field by the
  same mapping that the corpus change used: the text before the first `;` (a price or a weight that the row repeats),
  without a trailing qualifier in parentheses, with each of the seven variant spellings mapped (`-` and `held` to
  `none`, `amulet` to `neck`, `cloak` and `shoulder` to `shoulders`, `waist` to `belt`, `eye` to `eyes`). Each part of a
  value with ` or ` maps alone. A value that print gives and that the vocabulary leaves open (`head and eyes`)
  compares by its own text. A row that states another slot still drifts. The body is never edited, so the rule
  carries the vocabulary decision instead of overriding it.
- **Paragraph form.** `ac`, `hp`, `Type`, `Subtype`, `Slot` and the ability scores had no paragraph comparison and
  still have none: a value stated in only one place is not drift. Only `cr` is compared in both forms.

No format changes: `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.

## Evidence

The pinned corpus (`v0.1.39`) has 0 drift. A scratch copy of the data repository, converted with the prototype of the
#166 script (2,787 monsters and the items that convert mechanically), has 2 drifts, and both are old data
differences that the conversion exposes (the `elemental-lord-ymeri` melee line, and the `brutally-weighted` price field,
which holds the weight too).
