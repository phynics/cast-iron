---
title: "A sentence may start after a parenthesis; bonus feat choices as a subject"
date: 2026-11-25
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# A sentence may start after a parenthesis; bonus feat choices as a subject, 2026-11-25

Data pull request 317 held two entries of #540 that the reader of
[the prose and aside entry](2026-11-25-prose-and-aside-trades-and-reader-descriptors.md) did not
read.

## A sentence starts after ".)"

Daring Infiltrator (Advanced Class Guide p. 124) prints, under its scaffold heading Bonus Feats:
"(Those marked with an asterisk are detailed in this book.) This alters the bonus feat class
ability." The sentence splitter ended a sentence only at a period followed by whitespace. The
period here is followed by `)`, so the whole section was one sentence that starts with "In
addition to combat feats", and the check read no trade sentence ("alters: Bonus feats" against
"none").

The rule: a sentence also ends at `.)`, `!)` or `?)` followed by whitespace (or the end of the
text), when that `)` closes a `(` opened earlier in the text. It stays narrow:

- "(Ex) This ...", "(see below) This ..." and "(as normal) This ..." have no sentence mark before
  the `)`, so they end no sentence;
- a `)` that closes no `(` ends nothing;
- an aside that ends its own sentence now leaves the rest to the next sentence:
  "This ability replaces swift alchemy (a deep bomber never gains instant alchemy.) It also ..."
  gives the phrase `swift alchemy` and the aside, and no longer reads "It also ..." as part of the
  phrase.

## `choice` and `choices` after `bonus feat`

Ironskin Monk (Advanced Race Guide p. 122) prints "These bonus feat choices replace Dodge,
Mobility, and Spring Attack on his bonus feat list." The words `choice` and `choices` are a
subject word only right after `bonus feat` or `bonus feats`, same status as the other granted-thing
subjects of [the granted-thing entry](2026-11-24-granted-thing-subjects-in-trade-sentences.md): the
sentence is bare (it counts only when it names a stored trade of the same op). "These choices
replace ...", "These bonus choices replace ...", "These feat choices replace ..." and
"These bonus spell choices replace ..." stay prose. The phrase "Dodge, Mobility, and Spring Attack
on his bonus feat list" pairs with the stored trades `Dodge`, `Mobility` and `Spring Attack on his
bonus feat list`, with no further rule: the tail "on his bonus feat list" belongs to the last item.

No format version changes. The pinned corpus and the head of data pull request 317 check with the
same output before and after.
