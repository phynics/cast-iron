---
title: "Subjects that name a granted thing in trade sentences"
date: 2026-11-24
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# Subjects that name a granted thing in trade sentences, 2026-11-24

Ruling 22 of [the reader tolerance entry](2026-11-24-trade-sentence-reader-tolerance.md) renames a
level back-reference trade to the printed words, for example `Mystery bonus spells at these
levels`. The six oracle archetypes (Dual-Cursed Oracle, Enlightened Philosopher, Planar Oracle,
Possessed Oracle, Seer, Stargazer; Ultimate Magic pp. 58 to 59) print the sentence under their
Bonus Spells heading as "These bonus spells replace the oracle's mystery bonus spells at these
levels." The reader did not accept "These bonus spells" as a subject, so the renamed trade found
no sentence.

## Decision

A subject of the form "This" or "These", then an optional `bonus`, then a noun that names what a
feature grants, is a trade-sentence subject. The list is closed:

- `bonus` is a subject word only right before one of `spell`, `feat`, `ability`, `power`,
  `talent`, `trick`, `hex`, `skill`, `revelation`, `arcana`, `discovery` or `evolution` (singular
  or plural). "This bonus replaces ..." and "This bonus alters ..." (a numeric bonus) stay prose,
  as do other nouns ("These rounds replace ...").
- `spell`, `skill` and `evolution` join `feat` as subject words that name no ability. A sentence
  with one of them is **bare**: it counts only when it names a stored trade of the same op, since
  "These spells replace ..." may be prose about a spell list. The ability nouns that were already
  subjects (`power`, `talent`, `trick`, `hex`, `discovery`, `revelation`, `arcana`) keep their
  status, with or without `bonus`.

No format version changes. The pinned corpus and the heads of data pull requests 312 and 313 check
with the same output before and after, because no stored entry prints one of these sentences in a
feature yet.
