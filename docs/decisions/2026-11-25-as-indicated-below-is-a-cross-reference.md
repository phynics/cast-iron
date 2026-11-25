---
title: "A phrase ending in \"as indicated below\" is a cross-reference"
date: 2026-11-25
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# A phrase ending in "as indicated below" is a cross-reference, 2026-11-25

Owner ruling 40 of 2026-11-25 on #540 (data pull request 324, batch 4). Raven Master (Adventurer's
Guide p. 175) prints: "This ability alters hunter's bond and replaces specific ranger abilities as
indicated below." The ruling: a phrase that ends in "as indicated below" is a cross-reference, like
a printed "(see …)". The reader ignores it and stores no trade for it; the trades come from the
sentences of the later features. The reader gets rule R20, numbered after R19 of
[the second-clause entry](2026-11-25-second-clause-verbs-and-mid-phrase-asides.md). No format
version changes.

## Reading

- The unit is the phrase: the words after one verb, as the reader already splits them (a verb
  joined by `and`, R16's ", and it"). A phrase whose last words are "as indicated below" (any case,
  a trailing comma or colon aside) is dropped whole. It names no coverage and no trade is needed
  for it.
- The sentence's other phrases are read as before. Raven Master's sentence is one `alters`
  sentence naming `Hunter's bond`; without "as indicated below" the same sentence also names
  `Specific ranger abilities` as a `replaces` phrase.
- The words only count at the end of the phrase. "replaces bravery as indicated below at 5th
  level" and "replaces bravery (as indicated below)" keep their names, because the ruling names a
  phrase that ends in them.
- The ignored phrase is dropped whole, not trimmed to the words before "as indicated below":
  "replaces specific ranger abilities as indicated below" names no ability, and a list before the
  words ("replaces bravery and armor training as indicated below") is the cross-reference too.

## Open reading, for the owner

The ruling's wording ("a phrase that ends in …", "stores no trade for it") can mean two things for
the first clause of Raven Master's sentence:

1. Only the phrase "specific ranger abilities as indicated below" is the cross-reference, and the
   `alters` clause stays a trade sentence naming `Hunter's bond` (implemented).
2. The whole sentence is the cross-reference and yields nothing.

Reading 1 follows the ruling's phrase-level wording and R16's clause split. It has one consequence
for the data: a feature that prints this sentence needs a stored `alters` trade for `Hunter's bond`
(or the entry drifts). Reading 2 needs one more change: a sentence-level end check. The owner can
switch to reading 2 in one line.

## Checks

Unit tests in `trade_sentences.rs` carry the exact Raven Master sentence, the variant without the
cross-reference (both trades), the other verb, both clause orders, and the negatives (the words
before other words, inside a parenthesis, inside a longer word). One test in `drift.rs` pairs the
sentence with its trade through `find_drift`. The pinned corpus (v0.1.54) holds no phrase that
ends in these words, so `cargo xtask content` gives the same output.
