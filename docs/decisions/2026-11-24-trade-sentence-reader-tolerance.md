---
title: "Trade-sentence reader tolerance and the rulings that follow it"
date: 2026-11-24
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# Trade-sentence reader tolerance and the rulings that follow it, 2026-11-24

The batches of #540 (Ultimate Magic, Ultimate Combat) hold archetypes whose printed trade sentence
the reader (`crates/corpus/src/trade_sentences.rs`) did not read as the stored trade, though the
trade is right. The owner reviewed the held cases (data pull request 312, batch 1) and ruled on
them on 2026-11-24. This entry records the reader rules and the rulings that change the check.
`docs/corpus-format.md` holds the rules in their final wording; no format version changes.

## Ruling 21: the reader rules R1 to R12

The reader tolerates the printed variants below. Each is a test in the module, with the printed
wording of the case that held it (Ultimate Magic: Mindchemist, Broodmaster, Master Summoner,
Scrollmaster, Staff Magus; Ultimate Combat: Arcane Bomber, Siege Mage, Spellslinger, Tetori, Kensai, Battle
Scout, Warden, Wild Stalker, Pistolero, Beastmorph, Sacred Shield, Dervish Dancer, Battle Fury,
Evangelist).

| Rule | Reading |
|---|---|
| R1 | `class` is dropped before `ability` as it already was before `feature`: "the mutagen class ability" names `Mutagen`. `Class skills` keeps both words. |
| R2 | `normal` and `normally` are determiners, like `standard`. **`bonus` is never dropped.** |
| R3 | A spelled ordinal, `first` to `twentieth`, reads as `1st` to `20th`, on both sides. Different ordinals stay different. |
| R5 | A class name after an `Nth-level` word is an adjective ("the 10th-level wizard bonus feat"). It is an alternative reading, so the trade may keep the class name; different levels stay different. |
| R6 | "head N to M", "head N-M" and the en dash form are the steps N to M. It needs a head, N below M and a span of at most 20. |
| R7 | "as well as" joins two names like `and`. |
| R8 | The phrase ends at ", but ". |
| R9 | A closing parenthesis of four words or more is an aside, not part of the name. Short qualifiers ("(Ex)", "(longsword)") stay. |
| R10 | A possessor phrase before a name (an article, at most two words and a possessive word: "the sacred shield's holy champion ability") is dropped as an alternative reading, never across a comma or a connector. |
| R11 | A verb joined by ", and" to another clause is read as a bare sentence. It counts only when it names a stored trade of the same op. "This bonus replaces ..." and negated clauses are still rejected. |
| R12 | A sentence whose subject is the section's own heading (tags stripped) is a sentence of an ability, not a bare one: "Sermonic performance replaces ..." under "Sermonic Performance (Su)". |

The numbering follows the design; it has no R4.

**Why `bonus` is rejected as a determiner.** A bonus is part of what the name says: "bonus spells"
are not "spells", and "bonus teamwork feats" are not "teamwork feats". An oracle's mystery gives
bonus spells and also has spells of its own. If `bonus` were dropped, a stored `Spells` trade would
cover "the oracle's mystery bonus spells", and a trade could pass the drift check without the
book printing it. `normal` carries no such meaning ("the summoner's normal eidolon ability"), so it
is dropped. Both sides are tested: `Spells` does not cover "the oracle's mystery bonus spells", and
`Teamwork feats` does not equal "bonus teamwork feats".

R12 adds a `head` argument to the reader (`trade_sentences_headed`) and to the check helper
`sentences_in`. The drift check passes the plain text of the section heading; the other callers
pass `None`.

## Ruling 22: level back-references are renamed in the data

A stored trade whose printed words point back to levels printed in the feature ("... bonus spells
at these levels"; Psychonaut, Reanimator, six oracles) is renamed to the printed words. The levels
live in the feature's Level head. No reader rule. No app change.

## Ruling 23: a Weapon and Armor Proficiency section may print no trade sentence

A trade with no `by` and the op `alters` may rest on a "Weapon and Armor Proficiency" section that
prints no `alters` sentence, wherever it sits outside a listed feature (True Primitive, Gladiator,
Tactician). It is ruling A of the pilot (a trade passes when its feature only describes the
changed ability), for the one scaffold section that books print for proficiencies. Such a change
is stored as `alters`. A `replaces` trade does not rest on the section. A section inside a listed
feature belongs to the feature. Sentences that the section does print must still be covered by
trades. The check reads the heading with case and a trailing colon ignored, as for the other
scaffold headings.

The ruling says "prints no trade sentence"; the check reads it per operation, as ruling 24 does for
features: a section that prints a `replaces` sentence may carry an unstated `alters` trade.

## Ruling 24: ruling A is per operation

A feature named in `by` that prints no sentence **of the trade's own op** may carry the trade
unstated. The Preacher prints a `replaces` sentence and carries an `alters` trade (Skirnir
likewise). Within the same op the guard stays: a feature that prints a `replaces` sentence does
not state another `replaces` trade it does not name. The printed sentences must still be covered
by trades.

This changes two older tests: a feature that prints `replaces bravery` and a stored `alters`
trade that it does not name is no longer drift. The same-op versions of those tests keep the
guard.

## Ruling 25: "changes" is an `alters` verb

"This ability changes the Arcane Pool class feature and replaces the magus arcana gained at 3rd
level." gives an `alters` trade of `Arcane Pool` and a `replaces` trade of the arcana (Bladebound).
`changes` and `change` are read as `alters` only as the first verb after the subject, not as a
verb joined later by "and", and not in a prose sentence ("A magus changes his weapon.").

## Review ruling 19 (batch 1): "Skills" and "Alignment" are scaffold headings

`NOT_FEATURE_HEADINGS` in `check_features.rs` gains `Skills` and `Alignment`. Urban Barbarian, Cad
and Sohei print a "Skills" heading, and Martial Artist prints "Alignment". The check rejects
either as a listed feature (it gets no Level head and no search alias), and a section under
either may print trades without `by`, as under the other scaffold headings. The renderer and the
search aliases read `features` only, so they need no change.

## Consequences

- Corpus check on the pinned corpus stays at 0 problems; the pull request records the check on
  the head of data pull request 312.
- The reader grew by a few word lists and no per-entry exceptions. A wording that the reader
  cannot read is still fixed by adding the wording to the reader with a test.
