---
title: "Class-option powers: the owner's decisions on the open layout cases"
date: 2026-11-21
status: current
superseded_by: []
issues: [537]
prs: []
area: corpus
---

# Class-option powers: the owner's decisions on the open layout cases, 2026-11-21

Refs [#537](https://github.com/phynics/cast-iron/issues/537). The owner answered five open cases of
[2026-11-20-class-option-power-layout](2026-11-20-class-option-power-layout.md) on 2026-11-21, after
the review of the corpus rewrite (phynics/cast-iron-data#136 to #143). This entry records the answers
and the exact rules that `cast_iron_schema::power` now enforces. It refines section 1 (the canonical
order) and section 2 (candidates and validation) of the earlier entry and changes nothing else in it.
The counts in the earlier entry and in
[2026-11-21-class-option-power-counts](2026-11-21-class-option-power-counts.md) are those of the old
rules; "Counts" below gives the new ones.

## The canonical order after this entry

The order is the display order, and the renderer never reorders. Positions 4, 5 and 10 are new or
changed:

1. The name: one bold span, no trailing colon.
2. `**Gained:** At Nth Level`: optional, at most one.
3. `**Flavor:**`: optional, at most one.
4. `**Choice:**` and `**Info:**`: each optional and at most one, in either order. **New position**
   (decision 4).
5. The effect: **one effect row, or a passive row followed by an action row**, each with its own
   continuation paragraphs (decisions 1 and 2).
6. `**Usage:**`: optional, at most one.
7. Upgrades: one line per level, in strictly ascending level, each above `Gained` (decision 3).
8. `**Prereq:**`, then `**Replaces:**`: optional, at most one each.
9. Other rows, in source order. The unlabeled paragraphs directly after a `Special` row continue its
   note (decision 2).

A candidate whose name is a section label (decision 5) is excluded before any of this.

## 1. A passive row and an action row

**Decision.** A power with two effect rows is valid when the first is passive text and the second is an
action. The passive text comes first, then the action paragraph as its own line. The meta line shows
the action and its uses. All text stays, in print order.

**The rule.** Count the effect rows of a candidate (a row labeled `Passive`, `Ability`, `Standard`,
`Move`, `Swift`, `Immediate`, `Free`, `Full-round` or `Benefit`; a legacy alias counts as the label it
becomes). The candidate has one effect row, or two that satisfy all of:

- the first row is labeled `Passive` or `Ability` (the passive text; `Ability` is the label that print
  does not mark as an action);
- the second row is labeled `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round` (the
  action);
- the second row comes right after the first row and its continuation, and both stand in position 5.
  `Choice`, `Info`, `Usage`, an upgrade or any other row does not stand between them.

**How the corpus marks the action paragraph.** By its label, as for a power with one effect row: the
second row's label is the action. The renderer needs no other marker. It shows the first row's text
and continuation without a label, then the action row's text and continuation as a second paragraph
on its own line. The meta line takes the action from the second row's label and the uses from `Usage`.
`Passive` and `Ability` show no label (the earlier entry, section 3). `Usage` follows the action row
and belongs to it.

Each row keeps its own continuation: unlabeled paragraphs directly after the passive row are its
continuation, and those directly after the action row are the action's.

**Example.** The Aristocracy subdomain, Noble Visage (Su), *Champions of Balance* p. 16. The corpus
source has `Replaces` between the two effect rows. The canonical form moves it after `Usage`:

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
| The action first, then the passive row | `Immediate`, then `Passive` | `action-before-passive` (new reason) |
| Another row between the two effect rows | `Passive`, `Usage`, `Standard` | `out-of-order` |

The action-first pair is its own reason because print may set the action first. The rule is the
owner's: the passive text comes first. #88's script queues such a run for print lookup and does not
reorder it; whether the layout allows an action-first pair when print sets it that way is not decided
here (15 powers on corpus `main`, 15 on the pin).

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
effect row. Each appears at most once (`second-singleton-row`), in either order. Before this entry they
were "other rows" (position 9); a power that has one after the effect, after `Usage` or after an upgrade
now fails with `out-of-order`.

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
`Passive` row. The owner can add either to the list.

## Counts

Both corpora were checked with `cargo xtask corpus-check` before this change (the shared validator of
PR #544) and after it. The pin is tag `v0.1.33` (`dc3e2020`). Corpus `main` is `826d835c`, which holds
the family rewrites #136 to #142 and not the print resolutions of #143.

| | Candidates | Failing before | Failing after |
|---|---:|---:|---:|
| Pin `dc3e2020` | 3,819 → 3,792 | 2,099 | 2,043 |
| Corpus `main` `826d835c` | 3,880 → 3,853 | 520 | 518 |

The 27 fewer candidates on both are the excluded labels: 26 Base Evolutions and 1 Discipline Powers.

**Failing powers by reason** (a power can have several reasons):

| Reason | Pin before | Pin after | `main` before | `main` after |
|---|---:|---:|---:|---:|
| `out-of-order` | 1,253 | 1,271 | 155 | 227 |
| `legacy-label` | 887 | 860 | 276 | 249 |
| `no-effect-row` | 251 | 224 | 228 | 201 |
| `multiple-effect-rows` | 184 | 121 | 184 | 121 |
| `legacy-upgrade` | 145 | 145 | 26 | 26 |
| `stray-paragraph` | 109 | 76 | 86 | 53 |
| `second-singleton-row` | 64 | 64 | 9 | 9 |
| `numbered-effect-label` | 59 | 59 | 59 | 59 |
| `action-before-passive` | 0 | 15 | 0 | 15 |
| `empty-row` | 1 | 1 | 1 | 1 |

**What changed, on `main`.** 55 powers pass that failed before, and 53 powers fail that passed:

- 27 pass because they are labels (decision 5);
- 18 pass because their two effect rows are a passive row and an action row in canonical order
  (decision 1);
- 10 pass because paragraphs after `Special` are allowed (decision 2);
- 53 fail with `out-of-order` only, because `Choice` or `Info` stood after the effect (decision 4).

The 121 `multiple-effect-rows` that remain on `main` are 107 two passive rows (the `Ability` and
`Passive` merge that the orchestrator approved for #88), 9 two actions and 5 with three or more rows.
Counting the unlabeled paragraphs after `Special`: 44 powers on `main`, the 44 of the owner's comment.

## What the data rewrite must change

`scripts/rewrite-class-option-powers.py` in phynics/cast-iron-data mirrors the validator. It follows
this entry in six places:

1. **Canonical order.** Emit `Gained`, `Flavor`, `Choice`, `Info`, then the effect. In the script's
   `validate`: `RANK` becomes `Gained 2, Flavor 3, Choice 4, Info 4, effect 5, Usage 7, upgrade 8,
   Prereq 9, Replaces 10, other 11`. `Choice` and `Info` join `SINGLETONS` (a second one queues as
   `conflicting-rows`) and leave the `tail`. Every power that has them after the effect is rewritten,
   also one that was canonical before: 53 powers on `main` and 7 on the pin were canonical and now
   fail only for this reason.
2. **Two effect rows.** `rewrite_run` accepts two effects when the first is `Passive` or `Ability` and
   the second is `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round` (an alias or a mapped
   `Action` value counts as its label). It emits the passive row, its continuation, the action row, its
   continuation, then `Usage`; rows that stood between them (`Replaces`, `Prereq`, `Usage`) move to
   their own positions. Each group of unlabeled paragraphs must directly follow an effect row (or a
   `Special` row), and a pair with a legacy `Action:` row queues, because the group cannot be assigned
   without a guess. The validator mirror gets the same pair rule and the slug `action-before-passive`.
   `multiple-effect-rows` stays the queue reason for two passives, two actions, three or more rows and
   `Benefit` beside another row. An action first queues as `action-before-passive`; the script never
   reorders it.
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

- Whether a pair with the action first is valid when print sets it that way (15 powers).
- The renderer: this entry changes the validator, not `crates/web`. The renderer pull request reads
  `Power::action`, `Power::choice`, `Power::info` and the continuation of each `other` row.
