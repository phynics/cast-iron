---
title: "Corpus format 4: feature facts beside printed prose"
date: 2026-11-24
status: current
superseded_by: []
issues: [540]
prs: [677]
area: corpus
---

# Corpus format 4: feature facts beside printed prose, 2026-11-24

The survey of #540 found that the import rewrote the text of 1,179 of 1,299 archetype entries: it
dropped printed levels, replaced "This ability replaces bravery." by a `**Replaces:**` row and
invented upgrade rows. The owner decided on 2026-11-23 to keep class features as the book prints them
("I dislike the over-partitioning of class features; align to printed prose with extensions to help
app feel"), and on 2026-11-24 chose form B for the data. This entry records the corpus format that
carries form B. The renderer and the data rewrite are separate slices of #540.

## Decision

- **Form B.** A feature is a headed section of printed prose (`## Name (Ex) {#anchor}`, the
  printed paragraphs, the printed trade sentence). The facts that the prose states stay in
  `[fields]`: `features = [{ anchor, level }]` (`level` optional: a feature that has no single
  level writes none), and `trades[].by`, a list of the anchors of the features whose sentence
  states the trade (one trade names every feature that prints it).
- **Format 4.** The new keys are optional and change no existing key, so the step from format 3
  rewrites only the `format` line (`cargo xtask corpus-migrate-features`, and the generic
  `migrate`). The corpus repository applies it as one mechanical change, as for earlier formats.
- **Kinds.** `features` on `archetype`, `class_option` and `class` (base class). `by` on the
  trades of archetypes and class options, the two kinds that have trades. A rule entry has no typed
  fields (`kind = "none"`), so it carries neither key; the 33 feature-shaped rule entries are rewritten
  as printed prose and get no facts until a later decision gives them a place.
- **Checks.** Each `features` anchor is the explicit `{#anchor}` of a heading (a derived key would
  change with the heading text, and the anchors become public deep links); the list follows body
  order; `level`, when written, agrees with an "At Nth level" or "Starting at Nth level" lead when
  the feature's first paragraph has one; each `by` names anchors of `features`, in body order; no
  listed feature sits inside another's section; every section after the opening text (before the
  first listed feature) that prints a trade sentence is a listed feature or inside one.
- **Drift.** An entry with `features` compares its trades with the trade sentences of its feature
  sections instead of with `**Replaces:**` and `**Alters:**` rows: each trade's name appears in a
  trade sentence of at least one feature in its `by`, and each trade sentence of a listed feature is
  covered by the trades that name it. A trade with no `by` is a trade that the book prints in the
  opening text (before the first feature), and the drift check reads the opening text for it. An
  entry without `features` (a single-power class option has no heading) takes either today's rows or
  trade sentences anywhere in the normal part, with the same coverage rules. The reader works on the
  trade names as stored (owner ruling, 2026-11-24): numbered steps ("armor training 1, 2, 3, and 4"),
  extra words ("the paladin's spells class feature") and a possessive do not need a data rewrite. The sections come from the whole body, cut only at another
  publication's `**Sources:**` block, so a multi-publication entry with `###` feature heads is read
  in full. The reader takes the printed wordings: one name, a list, replaces and alters in one
  sentence, a possessive before the name, hyphen-suspended level lists and numbered steps; it refuses
  negated sentences and subjects that are not an ability. An entry without
  `features` keeps the row comparison, so the pinned format 3 corpus gives the same drift results, and
  the corpus can move entry by entry during the rewrite. Final state: a later change makes the
  import rows an error for these kinds (#540, last step).
- **Schema.** `features` (archetype, class option, class fields) and `by` (a trade) are additive
  optional keys, left out when empty. No reader of these types denies unknown fields and the tag-first
  reader of `TypedFields` gives the rest of the map to the variant, so an older client ignores them:
  `SCHEMA_VERSION` stays 7 and `INDEX_FORMAT_VERSION` is unchanged.

## Why not the alternatives

- Putting the trade on the feature (`features[].replaces`) would hide a trade's printed name from
  `trades` and from `trades.json`, which the compatibility filter reads. `by` keeps `trades` as
  it is, and a list lets one trade name each feature that prints it.
- Reading the trade sentence with a free-text match is too loose to guard the data. The reader
  accepts the printed wordings (one name, a list, replaces and alters in one sentence, a
  hyphen-suspended level list, a shared tail without the hyphen, numbered steps, a possessive or a
  base-class name before the feature, extra `ability` or `class feature` words) and nothing else, so
  a new wording fails loudly and is added to the reader with a test. A pronoun subject alone ("These
  replace the normal attack roll bonus ...") counts only when it names a stored trade, and a
  comparison to a spell ("functions like alter self") is not a trade.

## What the corpus repository does after this merges

Its CI builds the checker from the app's `main`, so the checker now reads formats 1 to 4 and a
format 3 corpus stays green. Format 4 starts when the corpus repository runs
`cargo xtask corpus-migrate-features <corpus>` (or `cast-iron-corpus migrate-features`) and commits the
one-line change to `corpus.toml`; the rewrite scripts need it before they write `features` or `by`,
because a format 3 corpus rejects both. The app pins the tag that holds format 4.

## 2026-11-24 addendum: the pilot holds (#540)

The Advanced Player's Guide pilot (cast-iron-data#306) held three groups of trades that the reader
did not state yet. The owner ruled on them the same day, and the reader and the drift check now follow:

- **A trade that print never states.** A stored trade with no printed trade sentence passes the drift
  check when its `by` names a listed feature that prints no trade sentence at all (druid archetypes'
  `alters = "Wild shape"`, paladin archetypes' `alters = "Divine bond"`: the feature describes the
  changed ability). Coverage still applies the other way: every printed trade sentence must be covered
  by trades. A trade without `by` and without a sentence in the opening text stays drift, as does a
  trade whose feature prints another trade sentence but not this one.
- **More ability nouns as the subject.** "This performance replaces ..." and "These performances
  replace ..." (the bard archetypes) read as trade sentences; so do the nouns of the other classes'
  choices (`trick`, `talent`, `hex`, `discovery`, `revelation`, `mercy`, `deed`, `exploit`, `arcana`).
  "These feats replace ..." is a subject word that names no ability, so that sentence is bare and counts
  only when it names a stored trade. "This bonus replaces ..." stays prose. The list is in
  `docs/corpus-format.md`.
- **"replaces both X and Y"** reads as a two-item list. "replaces either X or Y" is not read: no
  printed sentence of the pilot uses it.

## 2026-11-24 addendum: trades under a scaffold heading (#540, owner ruling 13)

A trade sentence printed under a scaffold heading (a heading of the closed list `NOT_FEATURE_HEADINGS`,
such as "Bonus Feats" or "Weapon and Armor Proficiency") counts as a trade with no `by`, wherever the
heading sits, also after the first listed feature, unless a listed feature contains it. Arcane Duelist
and Zen Archer of the Advanced Player's Guide print their trades of versatile performance and of the
monk's bonus feats there. The check no longer rejects such a sentence, and the drift check reads it
with the opening text: a trade without `by` may be stated in either place, and every sentence in
either place must be covered by trades without `by`. The scope is narrow: only scaffold headings, so
a trade sentence under any other unlisted heading after the opening text still fails.
