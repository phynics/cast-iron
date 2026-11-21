---
title: "Class-option power counts: two discoveries and two units"
date: 2026-11-21
status: current
superseded_by: []
issues: [537]
prs: [544]
area: corpus
---

# Class-option power counts: two discoveries and two units, 2026-11-21

Refs [#537](https://github.com/phynics/cast-iron/issues/537). This entry corrects the counts of
[2026-11-20-class-option-power-layout](2026-11-20-class-option-power-layout.md) and changes none of its
rules. It follows the review of PR [#544](https://github.com/phynics/cast-iron/pull/544), which ran the
shared classifier and validator on the pinned corpus for the first time. The pinned corpus is tag
`v0.1.32` (`c265325e`). The earlier entry names the tag `v0.1.26` for the same commit; the commit is what
the counts rest on.

## Two discoveries

The earlier entry counts 3,884 candidates. That number is the rewrite's discovery. The strict validator
finds fewer:

| Discovery | Candidates | What it finds |
|---|---|---|
| Rewrite (#88) | 3,884 | The strict candidates, plus the legacy names: an empty row that is the first block of the body or whose label ends in an ability tag, which the rewrite treats as a name written with a colon |
| Validator (`cast_iron_schema::power`, `corpus-check`) | 3,819 | A name directly followed by a row, a lead or a legacy upgrade, with a power marker in its run. A row is never a name |

The 65 difference is the legacy names: 64 magus arcana (`**Accurate Strike (Ex):**` and its kin) and one
sorcerer bloodline ability, Elemental Ray (Sp). The earlier entry says 64.

Of the 3,819 candidates, 1,720 pass validation, which matches the earlier entry, and 2,099 fail.

## Two units

The earlier queue table is headed "Powers", and two of its counts need a unit or a rule:

- **Numbered effect labels.** 59 powers contain 62 numbered rows (`Ability2`, `Ability3`). Three powers
  have two numbered rows each: Final Revelation (Bones), Natural Divination (Nature) and Personal Spire
  (Nex). The table's 62 counts rows. Counted per power the number is 59, and this validator counts per
  power.
- **More than one effect row.** A numbered label is its own reason and is not an effect row, so
  `Ability` followed by `Ability2` is one effect row and one numbered label (the Coven fixture). With
  that rule the validator finds 184 powers with more than one effect row, and the earlier table says
  187. The three extra are Greater Arcane Redoubt, Arcane Scent and Pool Ray, which only the rewrite's
  discovery reaches.

## What stays

Every rule, fixture and the order of the work in the earlier entry stays. The rewrite and its resolution
file still follow the earlier entry. A reader who compares counts uses this entry for the discovery and
the unit.
