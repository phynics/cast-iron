---
title: "Affliction types and item slots are closed vocabularies"
date: 2026-11-22
status: current
superseded_by: []
issues: [592, 594]
prs: [608]
area: corpus
---

# Affliction types and item slots are closed vocabularies, 2026-11-22

Refs [#594](https://github.com/phynics/cast-iron/issues/594) and [#592](https://github.com/phynics/cast-iron/issues/592), and phynics/cast-iron-data#171. It carries out the "Later" bullet of [filter fields have closed vocabularies](2026-11-22-filter-fields-have-closed-vocabularies.md) with the same mechanism: a constant beside the check in `crates/corpus/src/check.rs`, a warning with a count per field, and an error once the pinned corpus has none.

## Decision

- **Affliction type** (`AFFLICTION_TYPES`, six values: `addiction`, `curse`, `disease`, `infestation`, `insanity`, `poison`). The type is a tag of the entry, and every affliction has exactly one of the six. The corpus tags an affliction with the lower-case first word of its printed `Type:` line and leaves the body as printed. The check reports an affliction with no type tag or with more than one. Other tags are not checked here.
- **Item slot** (`ITEM_BODY_SLOTS`, `ITEM_TECH_SLOTS`, `ITEM_QUALITY_SLOTS` and `is_item_slot`). A value is one of the printed slots (the 14 body slots, and `weapon` and `face` after the owner decisions below), `none`, one of the four enhancement values (`weapon quality`, `armor quality`, `shield quality`, `armor/shield quality`), or two distinct body slots or `none` joined by ` or ` (`neck or shoulders`, `none or neck`). The enhancement values do not combine. Case counts. The compiler leaves `none` out of the `slot` facet (`ITEM_SLOT_NONE`); a test in the compiler pins every closed value to the facet.
- **What the corpus change does.** A variant spelling goes to its printed slot (`-` to `none` where print says `Slot none`, `shoulder` to `shoulders`, `waist` to `belt`, `eye` to `eyes`). A qualifier that is no slot (`(see text)`, a price) leaves the field only when the body prints it. A value that names no body slot and that no single slot replaces (`see text`, `special`) leaves the field. The body is never edited, so a stat block `Slot:` row that holds the old value keeps the field as it is: the drift check compares the two, and the body wins.
- **Left open.** A value that print itself prints and that the list does not hold stays in the corpus with a review row, and the check keeps warning about it, until the owner decides (see "Owner decisions"). The check becomes an error when the pinned corpus has none left.

The data repository keeps the print citation of each value in `scripts/affliction-slot-vocabulary-171.tsv`, one review row per changed or left-open item in `scripts/affliction-slot-review-171.tsv`, and the rerunnable survey and fix in `scripts/affliction-slot-171.py`. The lists in `check.rs` and the vocabulary file must be equal.

No format changes: the tag and the facet value are data, not a format, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.

## Owner decisions

The owner decided the 67 open rows of the first corpus change on 2026-11-22 (phynics/cast-iron-data#171).

- `weapon` (Planar Adventures p. 48) and `face` (Inner Sea Intrigue p. 54) join `ITEM_BODY_SLOTS`, so they may form `or` pairs.
- `ITEM_TECH_SLOTS` is a second closed list for the sockets and the cybertech of the Technology Guide (pp. 36, 52, 53): `veemod goggles`, `skillslot`, `arm`, `brain`, `ears`, `legs` and `arms (cybernetic)`. A tech slot stands alone. It never forms an `or` pair, and no magic item takes one. `arms (cybernetic)` is the spelling of the adventure volume that prints it, which the Technology Guide does not.
- The corpus maps `amulet` to `neck`, `cloak` to `shoulders` and `held` to `none`, in the field only. Archives of Nethys decided `rod`, `armor (barding)`, `neck (but see text)` and `eye`; the review file of the data repository records each finding.
- The five values that take two slots at once or three alternatives stay open: `head and eyes`, `head and headband`, `chest and neck`, `body and helm`, `neck, ring, or none`. The check keeps warning about them.
