---
title: "A printed trade sentence in a listed feature needs a stored trade"
date: 2026-11-25
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# A printed trade sentence in a listed feature needs a stored trade, 2026-11-25

Data pull request 324 (#540, batch 4) converted archetypes whose features print "This alters
class skills." or "This replaces the hunter's proficiency with medium armor and shields." with no
stored trade, and `corpus-check` passed. A review found seven missing or wrong trades by reading
the sentences, and the data script's `printed_trade_gaps` now finds them. The app check did not.

## Why the drift check missed them

`drift.rs` compares each trade sentence of a listed feature with the trades whose `by` names the
feature, as an error. The comparison is right. The gap is in what counts as a sentence. The reader
marks a sentence **bare** when its subject names no ability noun: "This alters ...", "These replace
..." (the pronoun may stand for anything, so a bare sentence is a trade sentence only when it names
a stored trade of the entry, `names_a_trade`). A trade that was never stored is named by nothing, so
the bare sentence was dropped before the comparison. The other suspected causes do not apply: the
drift is an error, not a warning; the comparison covers every listed feature (it ends at another publication's
`**Sources:**` block only, which these entries do not have); and it compares op and `by` (a wrong op
or a wrong `by` was already drift when the sentence names a trade, and was invisible only when the
sentence was bare).

I reproduced the gap on the data branch of pull request 324 before its fix round: the entries that
the round fixed (savage-warrior, roof-runner, scion-of-talmandor and the others in its diff) passed
`corpus-check` without it.

## The rule

A new check, `check_printed_trades.rs`, reports each bare trade sentence of a listed feature of an
archetype or class option that names no stored trade of its op, with the path and line of the
feature heading, the feature anchor, the printed sentence and the trade to store. Together with the
drift check, a sentence of a listed feature now needs a trade with the same op, a covered name and
`by` that names the feature. The exemptions that the rulings of #540 set stay: the opening text and
scaffold sections (rulings 13 and 28), cross-references and asides (rulings 16, 29 and 40), the
sections inside a feature (ruling 37).

Two reader readings came with it, because the bare sentences that the reader used to drop were its
false positives. `change` after the singular `This` is a noun ("This change does not alter your
Hit Dice ..."; Planar Oracle in the Advanced Class Guide, Amnesiac in Occult Adventures), so it is no
verb; "These change ..." still is (ruling 25). A phrase that is only a demonstrative ("replaces that
ability", "replaces those abilities") names nothing; it needs one trade of its op with `by` = the
feature, of any name.

## The exception list lives in the corpus

A sentence that states no concrete trade on purpose (Weretouched's "This alters her other class
features, as detailed in each relevant class feature", Advanced Race Guide) is listed in
`trade-sentence-exceptions.toml` in the corpus root, in the form of `membership-exceptions.toml`
([the membership entry](2026-11-25-uncited-membership-is-an-error.md)): optional, `[[exception]]`
rows with `entry`, `feature`, `sentence`, `reason` and `issue`, `deny_unknown_fields`. A row that
excepts nothing is an error, so the list only shrinks. The file is read only by `corpus-check`; an
absent file is an empty list.

## Warning now, error with the fixing pin

Run on the pinned corpus (v0.1.54; v0.1.55 gives the same list), the rule finds nine sentences in
five entries:

- `ref/arc-cavalier/hussar`: `alters` Class skills (`agile-warrior`);
- `ref/arc-fighter/savage-warrior`: `replaces` the normal attack roll bonus and AC penalty for
  charging (`savage-charge`);
- `ref/arc-hunter/roof-runner`: `alters` Class skills and `replaces` the hunter's proficiency with
  medium armor and shields (`skilled`);
- `ref/arc-witch/gravewalker`, `ref/arc-witch/hedge-witch`: "replaces the witch's hex gained at 4th
  (8th) level" and "replaces the patron spells" (`alters` is stored): the stored name is
  "4th-level hex", the printed one is "hex gained at 4th level".

The data branch of pull request 324 fixes the first three and adds the trade for Weretouched's
cross-reference, which needs a row in `trade-sentence-exceptions.toml`; the witch entries remain
after it (so do Flood Walker, Pact Witch, Tatterdemalion and Soul Sentinel on the branches of pull
requests 324 and 329). With the rule an error, `cargo xtask content` would fail on the pinned
corpus. So the rule reports a **warning** now, and `PRINTED_TRADES_ARE_ERRORS` in the module is
`false`. The pull request that moves the pin to the data tag that carries the fixes and the
exception file sets it to `true` and states the result of `cargo xtask content` in its body. This
is the order of #461 (a warning measured the cleanup, then the rule became an error with the pin
that carried it), with a constant in the code where #461 had a second pull request, because the
rule and the exception file arrive in one change.

No format version changes: the file is not part of the compiled library, and the entries are
unchanged.
