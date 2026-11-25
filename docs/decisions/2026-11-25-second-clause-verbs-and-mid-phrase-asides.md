---
title: "Second-clause trade verbs, \", and counts as\" tails, alter self, mid-phrase asides"
date: 2026-11-25
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# Second-clause trade verbs, ", and counts as" tails, alter self, mid-phrase asides, 2026-11-25

Data pull request 314 (batch 3: Occult Adventures and Ultimate Intrigue) held five entries that
the reader of [the prose and aside entry](2026-11-25-prose-and-aside-trades-and-reader-descriptors.md)
and [the sentence start entry](2026-11-25-sentence-start-after-parenthesis-and-choice-subjects.md)
did not read. Four rules follow (numbered after R15 of the earlier entry). No format version
changes. The pinned corpus (v0.1.51) and the head of data pull request 314 check with the same
output before and after: no stored entry changes.

## R16: a second clause with a pronoun has its own verb

Held entries and their sentences:

- Sage Counselor, Feinting Flurry: "alters flurry of blows and ki pool, and it replaces the bonus
  feat gained at 10th level";
- Gray Paladin, Weakened Grace: "alters smite evil, aura of courage, aura of resolve, and aura of
  righteousness, and it replaces aura of good and divine grace";
- Guiding Blade, Daring Teamwork: "replaces bonus feats and the swashbuckler's 1st-, 7th-, and
  15th-level deeds, and it alters panache".

R11 already read ", and replaces X". The rule now reads `and`, a pronoun (`it`, `this`, `he`,
`she`, `they`), an optional `also`, and `replaces`/`alters` (`replace`/`alter` after `they`) as the
start of a second clause with that verb. The second sentence has the subject status of the first
(bare stays bare). The same form after a first clause that is no subject ("This movement provokes
attacks of opportunity as normal, and it replaces deadly performance") is read like R11's: a bare
sentence, which counts only when it names a stored trade of that op.

Negatives: ", and it gains ...", ", and it is ...", ", and it does not replace ..." are not trade
verbs (R13 still ends the phrase at ", and" for them, as the clause words `gains`, `is` and `does`
say). A noun that is no pronoun ("and the archer replaces armor training") starts no clause. A
clause word after a second clause's verb ("..., and it replaces bravery that is lost") no longer
makes R13 cut the first clause at an earlier ", and": R13 looks only up to the next second
clause. A word inside a parenthesis starts no clause and ends no phrase.

## R17: `counts` is a clause word

Mindblade, Psychic Pool (Occult Adventures): "replaces arcane pool, and counts as arcane pool for
the purpose of feats, abilities, and class features". `counts` joins the closed list of R13, so
the phrase ends at ", and counts as" and names `Arcane pool`. A list that holds no clause word
("arcane pool, and spell strike") is not cut.

## R18: a parenthesis in the middle of a phrase is an aside

Metamorph (Ultimate Intrigue p. 19): "replaces alchemy (which also includes bombs, extracts, and
mutagen) and Throw Anything". R9 dropped a closing parenthesis of four words or more; the same
rule now applies to any parenthesis of the phrase, so the phrase is `alchemy and Throw Anything`.
The dropped text is the aside (ruling 29): several asides join with "; ", and an aside that starts
with "see" leaves none (ruling 16). The aside here states the trades `Bombs`, `Extracts` and
`Mutagen` of the feature, as a closing aside does. A short parenthesis ("(Ex)", "(longsword)")
stays in the name, and a parenthesis that holds another one stays whole, as before.

## R19: "alter self" is a spell, never a verb

Flesheater (Occult Adventures p. 114: "This ability functions as alter self, beast shape IV, form
of the dragon III, or giant form II, as appropriate.") and Metamorph (Ultimate Intrigue p. 19:
"This ability functions like alter self, except as noted here."). The reader already skipped a
comparison ("functions as/like" before the verb) and both printed strings gave no sentence before
this change; the tests now state it. R19 makes the rule independent of the comparison words: a
bare `alter` followed by the word `self` is the spell name and not a verb, wherever it stands.
`alters self-sufficiency` and `alters self` (the verb `alters`) stay verbs, and so does `alter` before
any other word ("These abilities alter bravery.").

The hold reasons in the data pull request's proposal files quote "This ability alters self, ..."
as the sentence. That is not the printed text, and the checker does not read the printed text so.

## Checks

The unit tests of `trade_sentences.rs` carry the exact strings, with negatives, and one test in
`drift.rs` pairs the five held sentences with their trades through `find_drift`. `cargo xtask
corpus-check` on the pin and on the head of data pull request 314 gives "corpus ok" with the same
output as before.
