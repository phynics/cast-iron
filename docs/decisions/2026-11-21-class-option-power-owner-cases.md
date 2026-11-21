---
title: "Class-option powers: the owner's decisions on the open layout cases"
date: 2026-11-21
status: current
superseded_by: []
issues: [537]
prs: [552]
area: corpus
---

# Class-option powers: the owner's decisions on the open layout cases, 2026-11-21

Refs [#537](https://github.com/phynics/cast-iron/issues/537). The owner answered five open cases of
[2026-11-20-class-option-power-layout](2026-11-20-class-option-power-layout.md) on 2026-11-21, after
the review of the corpus rewrite (phynics/cast-iron-data#136 to #143), and two more answers (A and B,
below) after the review of PR #552. This entry records the answers and the exact rules that `cast_iron_schema::power` now enforces. It refines section 1 (the canonical
order) and section 2 (candidates and validation) of the earlier entry and changes nothing else in it.
The counts in the earlier entry and in
[2026-11-21-class-option-power-counts](2026-11-21-class-option-power-counts.md) are those of the old
rules; "Counts" below gives the new ones.

## The canonical order after this entry

The order is the display order, and the renderer never reorders. Positions 4, 5 and 9 are new or
changed:

1. The name: one bold span, no trailing colon.
2. `**Gained:** At Nth Level`: optional, at most one.
3. `**Flavor:**`: optional, at most one.
4. `**Choice:**` and `**Info:**`: each optional and at most one, in either order; the power keeps
   their source order. **New position** (decision 4).
5. The effect: **one effect row, or a passive row and an action row in either order** (print's
   order), each with its own continuation paragraphs (decisions 1 and 2, and answer A).
6. `**Usage:**`: optional, at most one.
7. Upgrades: one line per level, in strictly ascending level, each above `Gained` (decision 3).
8. `**Prereq:**`, then `**Replaces:**`: optional, at most one each.
9. Other rows, in source order. The unlabeled paragraphs directly after a `Special` row continue its
   note (decision 2).

A candidate whose name is a section label (decision 5) is excluded before any of this.

## 1. A passive row and an action row

**Decision.** A power with two effect rows is valid when one is passive text and the other an action.
The passive text comes first, then the action paragraph as its own line. The meta line shows the
action and its uses. All text stays, in print order.

**Answer A (owner, 2026-11-21, after the review of PR #552).** Print's order is authoritative. The
validator accepts a passive row and an action row in either order, and the power keeps print's order.
The 15 powers that have the action first (see "Counts") are checked against print one by one: where
print puts the passive first, the data rewrite reorders them; where print puts the action first, the
corpus keeps print's order. The validator has no `action-before-passive` reason; the script, not the
validator, holds those 15 back for the print lookup (see "What the data rewrite must change").

**The rule.** Count the effect rows of a candidate (a row labeled `Passive`, `Ability`, `Standard`,
`Move`, `Swift`, `Immediate`, `Free`, `Full-round` or `Benefit`; a legacy alias counts as the label it
becomes). The candidate has one effect row, or two that satisfy all of:

- one row is labeled `Passive` or `Ability` (the passive text; `Ability` is the label that print does
  not mark as an action);
- the other row is labeled `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round` (the
  action);
- the second row comes right after the first row and its continuation, and both stand in position 5.
  `Choice`, `Info`, `Usage`, an upgrade or any other row does not stand between them.

**How the corpus marks the action paragraph.** By its label, as for a power with one effect row: the
row whose label names an action is the action. The renderer needs no other marker. It shows the rows
in print order, each with its own continuation, the passive row without a label and the action row as
its own line. The meta line takes the action from the action row's label and the uses from `Usage`.
`Passive` and `Ability` show no label (the earlier entry, section 3). `Usage` follows the second row
and belongs to the action. The validated `Power` keeps the rows in print order (`effect`, then
`second_effect`); `Effect::is_action` and `Power::action` tell the renderer which one is the action.

Each row keeps its own continuation: unlabeled paragraphs directly after a row are that row's.

**Example.** The Aristocracy subdomain, Noble Visage (Su), *Champions of Balance* p. 16. The corpus
source has `Replaces` between the two effect rows. The canonical form moves it after `Usage`. A power
whose print order is the action first has `**Immediate:** ...` before `**Passive:** ...` and is valid
as written:

```markdown
**Noble Visage (Su)**

**Passive:** You gain a bonus on Charisma checks and Charisma-based skill checks against creatures
that share your creature type (and subtype, if you are a humanoid).

**Standard:** You can touch an ally to grant her your noble visage bonus against creatures that
share her creature type (or subtype) for 1 minute. When you use this ability, you lose your noble
visage bonus for 1 minute.

**Usage:** 3 + Wisdom modifier times/day

**Replaces:** The inspiring word ability of the Nobility domain
```

**Stays invalid** (reason `multiple-effect-rows`, unless noted):

| Combination | Example | Reason |
|---|---|---|
| Two actions | `Standard`, then `Swift` | `multiple-effect-rows` |
| Two passive rows | `Ability`, then `Passive` (107 powers on corpus `main`) | `multiple-effect-rows` |
| Three or more effect rows, also after a valid pair | `Passive`, `Standard`, `Swift` | `multiple-effect-rows` |
| A `Benefit` row beside another effect row | `Passive`, then `Benefit` | `multiple-effect-rows` |
| Another row between the two effect rows | `Passive`, `Usage`, `Standard` | `out-of-order` |

## 2. Prose after `Special`

**Decision.** The paragraphs after `Special` stay with the Special note, in print order. The validator
allows continuation paragraphs after `Special` as well as after the effect.

**The rule.** An unlabeled paragraph directly after a row labeled `Special` is a continuation of that
row. So are the unlabeled paragraphs after it, until the next row, upgrade lead or boundary. The
paragraphs keep their order and belong to the `Special` row that they follow; a run may hold several
`Special` rows, each with its own continuation. An unlabeled paragraph after any other row
(`Penalty`, `Functions As`, `Prereq`, `Replaces`, `Usage`, `Flavor`, an upgrade) is still stray
(`stray-paragraph`), as is one before the effect.

**Example.** The magus arcana Natural Spell Combat (Ex): `Ability`, one paragraph, `Special`, then two
paragraphs that continue the Special note.

## 3. Progressions

**Decision.** One upgrade line per level stays the rule. The corpus does not use print's compact
wording.

**The rule, unchanged.** Each level is one `**At Nth level,** <printed sentence>` line, in strictly
ascending level and above `Gained`. Two lines at one level (`upgrade-at-same-level`) and a line below
the one before it (`upgrades-not-ascending`) stay invalid. A progression such as the Summon Nature's
Ally ladder is one line for each of its levels: `At 12th level, this now behaves as summon nature's ally
VI.` The validator has no rule for the compact wording, so there is no change to validation.

## 4. `Choice` and `Info`

**Decision.** `Choice` and `Info` rows move before the effect, right after `Flavor`.

**The rule.** `Choice` and `Info` stand in position 4: after `Gained` and `Flavor`, before the first
effect row. Each appears at most once (`second-singleton-row`), in either order. The valid power keeps
their source order (`Power::preface`, a list of rows labeled `Choice` or `Info`), because the order is
the display order. Before this entry they were "other rows" (position 9 of the earlier entry); a power
that has one after the effect, after `Usage` or after an upgrade now fails with `out-of-order`.

**Example.** Basic Magic (Sp), *Ultimate Magic* p. 76:

```markdown
**Basic Magic (Sp)**

**Flavor:** An eidolon learns to cast a basic spell as a spell-like ability.

**Choice:** Select one spell from the following list: [acid splash](ref/spell/acid-splash), ...

**Ability:** This spell can be cast once per day as a spell-like ability. ...

**At 4th level,** this spell can be cast three times per day by spending 2 additional evolution points.

**Prereq:** Eidolon has Charisma 10+

**Special:** This evolution can be selected more than once. ...
```

## 5. Blocks that are not powers

**Decision.** Section labels such as Base Evolutions, Patron Spells, Discipline Powers and Bonus Spells
are excluded from the layout and render as today. The real powers under them keep the layout.

**The rule.** `candidates` never returns a candidate for a name block that is on this list:

> `Base Evolutions`, `Bonus Spells`, `Discipline Powers`, `Domain Spells`, `Patron Spells`,
> `Replacement Domain Spells`

The match is on the plain text of the name block, trimmed, without a trailing colon, with runs of white
space folded to one space, ignoring ASCII case. It is an exact match: `Base Evolutions (Su)`,
`Bonus Spells Known` and `Granted Powers` are names of powers. The list is `NON_POWER_LABELS` in
`cast_iron_schema::power`; a name is added to it only with evidence from the corpus and a line in a
decision. A label is still a name for the run before it: it ends the previous power's run, as every
name does. The rows after a label are no power's rows and render as today. The powers after it are
candidates of their own.

**Why these six.** Base Evolutions (the eidolon's evolutions by level, 26 entries) and Discipline
Powers (the section of the psychic discipline Faith) are the labels that the discovery found as
candidates on the pin: their rows are old upgrade labels (`**At 1st Level:**`) with no effect row.
`Bonus Spells`, `Domain Spells`, `Patron Spells` and `Replacement Domain Spells` are section labels of
the same kind in the corpus (the discipline and bloodline bonus spells, the domain spell list, a witch
patron's spells and the subdomain spell replacements). They hold spell rows and no marker today. They
are listed so that a corpus rewrite
that adds a `Gained` row or an upgrade to such a block cannot turn it into a candidate. `Patron Spells`
is also a row label in the witch patron entries (`**Patron Spells:** This unique patron grants ...`),
which is a row of the patron's power and not a name; the row stays an "other row" of that power.

Two blocks look like labels and stay powers. `Granted Powers` of the black powder inquisition holds an
`Ability` row with the granted bonus feats. `Class Skill` and `Bonus Feat` of the Glory subdomain hold a
`Passive` row. The owner decided on 2026-11-21 (answer B) that `Granted Powers`, `Class Skill` and
`Bonus Feat` stay powers and are not on the list.

**Survey of the candidates without an effect row (answer B).** The owner asked for the candidates that
still fail `no-effect-row` on corpus `main` to be checked for plain section names: no ability tag, and
rows that are only old upgrade labels, such as a `Bloodline Powers` block. The survey read the 228
candidates without an effect row on corpus `main` `7da4deff` before the exclusion (the validator's
201 after it). It found no further section label. After the 27 excluded blocks (26 Base Evolutions and
1 Discipline Powers):

- 105 names carry an ability tag (`Acid Resistance (Ex)`, `Adumbration (Su)`, ...): powers whose effect
  text has no effect row yet, the "lowest upgrade" and legacy-label cases of #88.
- 89 names end in `(minor)` or `(major)`: the powers of the `blessing` entries, with an
  `At 1st Level:` or `At 10th Level:` row (a few with more) and no effect row.
- 7 names have neither: `Emotionally Distant` (the Bleaching discipline), `Elemental Imbalance`,
  `God-Meddled`, `Lich` and `Vampirism` (oracle curses, each the entry's own curse), `Unarmed Strike`
  (an `order` entry with a progression) and `Rogue and Ninja Advanced Talents` (a slayer talent).
  Each is a power or the entry's own feature and stays a candidate.

`Bloodline Powers` and `Granted Powers` are not in the survey as section labels: the first is followed
by `Repeat At` and `Special` rows with no marker and is no candidate, and the second has an effect row.
No name was added to the list. A later survey that finds one lists it for the owner first.

## Counts

Both corpora were checked with `cargo xtask corpus-check` before this change (the shared validator of
PR #544) and after it. The pin is tag `v0.1.33` (`dc3e2020`). Corpus `main` was `826d835c` for the first
run and `7da4deff` for the last; both hold the family rewrites #136 to #142 and not the print resolutions
of #143. Both give the same candidate count and, under the first revision of this change, the same 518
failing powers. The "before" column is the validator of PR #544.

| | Candidates | Failing before | Failing now |
|---|---:|---:|---:|
| Pin `dc3e2020` | 3,819 → 3,792 | 2,099 | 2,035 |
| Corpus `main` `826d835c` / `7da4deff` | 3,880 → 3,853 | 520 | 510 |

The 27 fewer candidates on both are the excluded labels: 26 Base Evolutions and 1 Discipline Powers.

**Failing powers by reason** (a power can have several reasons):

| Reason | Pin before | Pin now | `main` before | `main` now |
|---|---:|---:|---:|---:|
| `out-of-order` | 1,253 | 1,271 | 155 | 227 |
| `legacy-label` | 887 | 860 | 276 | 249 |
| `no-effect-row` | 251 | 224 | 228 | 201 |
| `multiple-effect-rows` | 184 | 121 | 184 | 121 |
| `legacy-upgrade` | 145 | 145 | 26 | 26 |
| `stray-paragraph` | 109 | 76 | 86 | 53 |
| `second-singleton-row` | 64 | 64 | 9 | 9 |
| `numbered-effect-label` | 59 | 59 | 59 | 59 |
| `empty-row` | 1 | 1 | 1 | 1 |

**What changed, on `main`.** 63 powers pass that failed before, and 53 powers fail that passed:

- 27 pass because they are labels (decision 5);
- 18 pass because their two effect rows are a passive row and an action row, passive first, in canonical
  order (decision 1);
- 8 pass because their two effect rows are an action row and a passive row, in print's order (answer A);
- 10 pass because paragraphs after `Special` are allowed (decision 2);
- 53 fail with `out-of-order` only, because `Choice` or `Info` stood after the effect (decision 4).

On the pin, 71 pass that failed and 7 fail that passed (the same four groups, with the 7 of decision 4).
The first revision of this change counted an action-first pair as a failure of its own
(`action-before-passive`, 15 powers on both corpora). Answer A removed the reason: 8 of the 15 are valid
as written, and 7 fail for other reasons only.

The 121 `multiple-effect-rows` that remain on `main` are 107 two passive rows (the `Ability` and
`Passive` merge that the orchestrator approved for #88), 9 two actions and 5 with three or more rows.
Counting the unlabeled paragraphs after `Special`: 44 powers on `main`, the 44 of the owner's comment.

**The 15 action-first pairs** (answer A), on corpus `main` and the pin alike. They need the print lookup
whether or not they pass today, because a valid action-first run might not be print's order:

| Entry | Power | Now |
|---|---|---|
| `ref/domain/liberation`, `ref/domain/strength` | Perfected Form (Su) | valid |
| `ref/hkorder/order-of-the-chain` | 3: True Identities (Su) | valid |
| `ref/hkorder/order-of-the-scourge` | 3: Reason to Fear (Su) | valid |
| `ref/inquisition/anger` | Rage (Ex) | valid |
| `ref/kipower/ki-metabolism` | Ki Metabolism (Su) | valid |
| `ref/ragepower/greater-ancestor-totem` | Greater Ancestor Totem (Su) | valid |
| `ref/ragepower/unrestrained-rage` | Unrestrained Rage | valid |
| `ref/mystery/battle` | Final Revelation | fails: `legacy-label`, `out-of-order` |
| `ref/mystery/bones` | Final Revelation | fails: `numbered-effect-label`, `out-of-order` |
| `ref/order/order-of-the-ennead-star` | Subjugate (Ex) | fails: `out-of-order` |
| `ref/order/order-of-the-lion` | Shield of the Liege (Ex) | fails: `out-of-order` |
| `ref/order/order-of-the-tome` | Defensive Knowledge (Ex) | fails: `out-of-order` |
| `ref/sorcbloodline/harrow` | See It Coming (Su) | fails: `legacy-label`, `legacy-upgrade`, `out-of-order` |
| `ref/sorcbloodline/imperious` | Heroic Echo (Su) | fails: `legacy-label`, `out-of-order` |

## What the data rewrite must change

`scripts/rewrite-class-option-powers.py` in phynics/cast-iron-data mirrors the validator. It follows
this entry in six places:

1. **Canonical order.** Emit `Gained`, `Flavor`, then `Choice` and `Info` **in their source order**
   (the power keeps it; either order is valid), then the effect. In the script's `validate`, `RANK`
   becomes `Gained 2, Flavor 3, Choice 4, Info 4, effect 5, Usage 7, upgrade 8, Prereq 9, Replaces 10,
   other 11`. These are the validator's code ranks, not the item numbers of the list above. `Choice`
   and `Info` join `SINGLETONS` (a second one queues as `conflicting-rows`) and leave the `tail`.
   Every power that has them after the effect is rewritten, also one that was canonical before: see
   "Counts" for the numbers that were canonical and now fail only for this reason.
2. **Two effect rows.** `rewrite_run` accepts two effects when one is `Passive` or `Ability` and the
   other is `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round` (an alias counts as the
   label it becomes). It emits the first row, its continuation, the second row, its continuation, then
   `Usage`; rows that stood between them (`Replaces`, `Prereq`, `Usage`) move to their own positions.
   - **Passive first.** The script rewrites it as above.
   - **Action first (answer A).** The script queues every such pair, with its whole span unchanged, as
     a script-only reason `action-first-pair`, for a print lookup. It never reorders one. A
     resolution that print settles replaces the span with either order, and `validate` accepts both.
     A candidate that already validates is left alone by the rewrite, so the 8 action-first powers
     that are valid today (see "Counts") need a resolution only where print puts the passive first;
     the script's print check covers all 15.
   - **A legacy `Action:` row in the pair.** The pair always queues. The mapped value of the `Action`
     row only chooses the queue reason (`multiple-effect-rows` or the existing `Action` reasons such
     as `action-without-one-group` and `unmapped-action`); it never produces output, because the
     group of unlabeled paragraphs cannot be assigned to a row without a guess.
   - Each group of unlabeled paragraphs must directly follow an effect row (or a `Special` row).
   - The `validate` mirror gets the same pair rule in either order and **no** `action-before-passive`
     slug. `multiple-effect-rows` stays the reason for two passives, two actions, three or more rows
     and `Benefit` beside another effect row.
3. **`Special` continuation.** An unlabeled paragraph after a `Special` row, and those after it until
   the next row, are the Special's continuation. They no longer count as a paragraph group, so they
   no longer queue as `unlabeled-paragraphs-not-after-effect` or break an `Action` group count. The
   output writes each `Special` row with its paragraphs, in source order. The validator mirror accepts
   the same paragraphs. Only `Special` has this rule.
4. **Progressions.** No change. A progression stays one `**At Nth level,**` line per level; two
   upgrades at one level still queue.
5. **Section labels.** `candidates` (and `legacy_names`) skip a name that matches `NON_POWER_LABELS`
   under the rule above: trimmed, no trailing colon, folded white space, ASCII case-insensitive. The
   27 spans (26 Base Evolutions, 1 Discipline Powers) leave the queue file and
   `class-option-power-resolutions.toml`. A resolution for an excluded name is reported as obsolete,
   not stale, and removed. The text that the print resolutions already rewrote in those spans stays as
   it is.
6. **Fixtures and self-test.** Add the fixtures of `crates/schema/tests/power_fixtures.rs`
   (Noble Visage, Natural Spell Combat, Basic Magic, the discipline `Discipline Powers`, the eidolon
   `Base Evolutions`, the witch patron `Patron Spells`, and the combinations that stay invalid) to
   `FIXTURES`. `--check` then has nothing to rewrite and `corpus-check` reports no warning that the
   queue does not name.

## Not decided here

- The renderer: this entry changes the validator, not `crates/web`. The renderer pull request reads
  `Power::effect`, `Power::second_effect` (print's order), `Power::action()`, `Power::preface` and the
  continuation of each `other` row.
