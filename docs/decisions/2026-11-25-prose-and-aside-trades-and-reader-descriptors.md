---
title: "Prose and aside support for trades, clause ends and scaffold variants"
date: 2026-11-25
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# Prose and aside support for trades, clause ends and scaffold variants, 2026-11-25

The reviews of data batch 2 (Advanced Class Guide and Advanced Race Guide, pull request 313) and
batch 3 (pull request 314) gave owner rulings 28, 29, 30 and 33 on #540. They extend the reader
and the checks of the earlier entries ([reader tolerance](2026-11-24-trade-sentence-reader-tolerance.md),
[granted-thing subjects](2026-11-24-granted-thing-subjects-in-trade-sentences.md)). No format
version changes, and no stored entry changes with them: the pinned corpus checks with the same
output.

## Ruling 28: prose supports a trade, narrowly

Verminous Hunter (Advanced Class Guide) keeps a stored `hunter tactics` trade with `by =
["vermin-companion"]`. The feature prints an `alters` sentence and says in prose "The hunter
tactics class ability allows a verminous hunter to grant her teamwork feats ...".

The rule: a trade with `by` is stated when its name, by the reader's name comparison, appears in
a sentence of the text of a feature that `by` names, **and that sentence is not itself a trade
sentence**. Three limits keep it narrow:

- only the features that `by` names count; the prose of another feature states nothing;
- a trade sentence never counts, so a trade whose op differs from the sentence that names it
  (the book says "alters hunter tactics", the trade says `replaces`) is still drift;
- the other rules are unchanged: every printed trade sentence must still be covered by trades,
  and a trade that the feature text never names stays drift under ruling 24's guard.

## Ruling 29: a parenthetical names a trade

"This ability replaces swift alchemy (a deep bomber never gains instant alchemy)." (Deep Bomber)
supports both trades: `replaces` `Swift alchemy`, and `replaces` `Instant alchemy` by the same
feature. R9 drops a closing parenthesis of four words or more from the phrase, so coverage is
unchanged; the reader now keeps the dropped text as `TradeSentence::aside`. A trade whose `by`
names the feature is stated when its name appears in the aside of a sentence **of the trade's own
op**.

## Ruling 30: clause ends and descriptors

- **R13.** The phrase ends at ", and " when the rest holds a clause word (a closed list: `that`,
  `which`, `who`, `also`, `modify`, `modifies`, `gain`, `gains`, `is`, `are`, `does`, `cannot`,
  `must`, `grants`, `allows`) and does not start with a verb. Eldritch Scion: "replaces arcane
  pool, and abilities that modify arcane pool also modify eldritch pool" names `Arcane pool`. A
  list "bravery, armor training, and weapon training" has no clause word and is not cut; ", and
  replaces ..." is still a second trade.
- **R14.** A class name right before "class ability" or "class feature", after another word, is a
  descriptor (R5's alternative reading, with the class names of `strip_class_names`): Naturalist
  "the aspect summoner class ability" names `Aspect`.
- **R15.** "raging song" after a name is a descriptor: Spell Warrior "the inspired rage raging
  song" names `Inspired rage`, and "the dirge of doom raging song" names `Dirge of doom`. At the
  start of a name or after a comma, an article or a connector it is the name `Raging song`.

Stored names stay short.

## Ruling 33: more scaffold headings, compared without a tag

`NOT_FEATURE_HEADINGS` gains `Weapon and Armor Proficiencies`, `Weapon Proficiencies`, `Armor
Proficiency`, `Armor Proficiencies`, `Skill Ranks per Level` and `Class Skills`. A scaffold heading
is compared without case, a trailing colon and an ability tag, so "Bonus Feats (Ex)" (Disenchanter)
is the scaffold `Bonus Feats`. Ruling 23's rule (an `alters` trade without `by` may rest on a
proficiency section that prints no `alters` sentence) covers the proficiency variants.

## Not reproduced: Strangler

The data thread reported that R10 (ruling 21: a possessor phrase before a name is dropped) hid
`Unarmed strike` in Strangler's sentence with "brawler's flurry" (Advanced Class Guide p. 88).
R10 is an alternative reading that is tried after the plain reading, so it cannot remove a match
that the plain reading finds. The tests cover six wordings of that sentence and all pass; the
heads of data pull requests 313 and 314 hold Strangler without prose sentences yet, so the exact
printed text was not available to test.
