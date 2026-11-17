---
title: "Spell-list structure in spell and creature stat lines"
date: 2026-11-05
status: current
superseded_by: []
issues: [42]
prs: []
area: content
---

# Spell-list structure in spell and creature stat lines, 2026-11-05

Settles #42 against the canonical Pf Data library. The old `spell_rows` and
`creature_rows` joined a spell's classes into one string and used raw directive
keys for spell levels.

- **A spell's `Level` row groups by spell level.** Each level is one line
  ("3: Sorcerer, Wizard"), built from `Inline::Text` and `Inline::Break` nodes.
  The class/level pairs survive and the compact stat-line styling does not
  change. `spell_level_value` is shared by the canonical Pf Data renderer and
  the PSRD carry-over renderer, so carried entries get the same shape.
- **Creature spell lists keep their upstream groups and use readable labels.**
  `::mspell` level keys `l0` through `l9` become "Cantrips" or "Nth-level
  spells"; `cl`, `con`, and `pe` become "Caster level", "Concentration", and
  "Power points". The values stay in one list per level or frequency, as the
  source already grouped them.
- **No serialized contract change.** No new block or inline node is introduced,
  so `SCHEMA_VERSION` stays 3 and `INDEX_FORMAT_VERSION` stays 1. A loader that
  ignores `Inline::Break` still reads the text.
- **The PSRD-only build changes too.** `build.rs` uses the typed `levels` array
  instead of the upstream one-line `level_text`, so `cargo xtask content --psrd`
  has the same grouped shape as the canonical default.
