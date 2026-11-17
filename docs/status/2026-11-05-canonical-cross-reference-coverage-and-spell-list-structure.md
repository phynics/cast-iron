---
title: "Canonical cross-reference coverage and spell-list structure"
date: 2026-11-05
kind: record
---

# Canonical cross-reference coverage and spell-list structure, 2026-11-05

The canonical library at `generated/content` is content version
`v8b314f5f8dcf7565`: 31,106 entries with schema 3 and search format 1. The
compiler reports 0 out-of-selection links and 0 broken links. The compiled tree
has 141,081 body link nodes in 22,297 entries (71.7%).

`scripts/measure-reference-links.py` is the rerunnable measurement for #20 and
#22-#26. It reads the compiled content tree, so it can be rerun against
`generated/psrd` with `--content generated/psrd`. Local numbers for the
canonical build:

| Category | Links to targets | Entries with such a link | Bare exact italic/bold spans | Pf | Carried | Self |
|---|---:|---:|---:|---:|---:|---:|
| Spells (spell bodies only, #20) | 1,583 | 668 | 398 | 236 | 162 | 44 |
| Feats (#24) | 18,307 | 8,944 | 262 | 235 | 27 | 25 |
| Special abilities (#25) | 5,878 | 3,862 | 935 | 663 | 272 | 19 |
| Conditions (#26) | 4,447 | 2,623 | 59 | 47 | 12 | 0 |

The largest spell residue is *protection from evil* (51), *magic circle* (41),
and *magic circle against evil* (21). The largest special-ability residue is
*spell resistance* (284), *spell-like abilities* (169), and *invisibility* (93).
These are exact-title hits, not confirmed references: *consumption* and
*renown* are mass-combat or class terms that share a feat title. Cloudkill
(`ref/spell/cloudkill`, a Pf Data entry) names *fog cloud* twice in plain text,
not italic, so an italic-only pass cannot satisfy that old acceptance example.

#42 is fixed in the compiler. All 3,135 spells now group their level row by
spell level (3,022 Pf and 113 carried); before the change all 3,022 Pf spells
and 113 carried spells were one flat string. 346 Pf creatures carry a
class/level spell list rendered with readable labels; 1,086 carry
frequency-only lists. The carried library adds 114 class/level spell lists
and 2 frequency-only lists. Raw `l0`..`l9`, `Cl:`, and `Con:` labels in
creature stat blocks fell from 1,432 Pf entries to 0.

Local verification: `cargo xtask check` passes; two `cargo xtask content` runs
produced the same file-hash manifest (`39cdced315f96d2de9ac12234c69f15c46b03dfc3cc1566a2908e0e2db31fb70`);
`check-publication.py generated/content` and
`check-library-upgrade.py generated/psrd generated/content` exit 0; the
Chromium `reference.spec.ts` suite passes (14 tests), including the new spell and
creature spell-list checks. This is local evidence only; no deployment is
claimed.
