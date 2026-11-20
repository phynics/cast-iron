---
title: "Class-option powers render as printed prose from canonical labeled rows"
date: 2026-11-20
status: proposed
superseded_by: []
issues: [537]
prs: []
area: ui
---

# Class-option powers render as printed prose from canonical labeled rows, 2026-11-20

Refs [#537](https://github.com/phynics/cast-iron/issues/537) and phynics/cast-iron-data#88. Status
`proposed`: the owner approves the mockups and the open questions before any implementation.

## Problem

A class-option power (school power, revelation, hex, curse, spirit ability, domain power) is a bold
name paragraph followed by bold-label rows: `**Gained:** At 1st Level`, `**Passive:**` or an action
label with the base effect, `**Usage:**`, and one `**At Nth Level:**` row per upgrade. The import
also sorted the rows by key, so 410 powers in 269 files show their upgrades before the base effect.
The owner asked for the printed form instead: the name, the gained level on its right, the base
effect, then one line per upgrade (`At 11th level, this resistance increases to 10.`).

## Decision

### 1. The corpus keeps labeled rows, in one canonical order

No new block type and no heading. A power stays a bold-only name paragraph followed by labeled
paragraphs, in this order. The order is the display order, so the renderer never reorders:

```markdown
**Resistance (Ex)**

**Gained:** At 1st Level

**Passive:** You gain [resistance](ref/umr/resistance) 5 to an energy type of your choice, chosen
when you prepare spells. This resistance can be changed each day.

**At 11th level,** this resistance increases to 10.

**At 20th level,** this resistance changes to [immunity](ref/umr/immunity) to the chosen energy
type.
```

1. The name: one bold span, no trailing colon. The ability type stays in the name: `(Ex)`, `(Su)`, `(Sp)`.
2. `**Gained:** At Nth Level`: optional, at most one.
3. `**Flavor:**`: optional.
4. The base effect: exactly one row labeled `Passive`, `Ability`, `Standard`, `Move`, `Swift`,
   `Immediate`, `Free`, `Full-round` or `Benefit`, then its unlabeled continuation paragraphs.
5. `**Usage:**`: optional, at most one.
6. Upgrades, ascending, each above the gained level: `**At 11th level,** <printed sentence>`.
7. Other rows, `Prereq` and `Replaces` first, then `Special`, `Penalty`, `Functions As` and the rest in
   their current order.

The two changes of form are the upgrade lead and the name without a colon. The upgrade line
holds the printed wording, so the corpus produces it, not the renderer. The bold lead `At 11th
level,` is the typed marker (a `Strong` inline at the start of the paragraph) that the renderer
reads. A renderer that does not know the layout still shows correct prose. The rewrite in #88 also
folds the import's other shapes into this one:

- `**Action:** Standard Action` followed by an unlabeled effect paragraph becomes `**Standard:** <effect>`.
- `Fullround` becomes `Full-round`, and `Passive Ability` becomes `Passive`.
- Plain-text `At 8th level: These bonuses …` lines become upgrade leads.
- Exact duplicate `Usage` rows are removed.
- A power with no base effect and no `Gained` row (Fire Resistance of the Fire domain prints
  `At 6th level, you gain resist fire 10.`) gets `Gained` from its lowest upgrade, and that upgrade
  becomes the base effect.
- The renderer does not change the case of corpus text. The rewrite script lowercases the first
  word of an upgrade only when that word is on a fixed list (`You`, `The`, `This`, `Your`, …). The
  script queues every other upgrade for review against print.

**Not a corpus format change.** The files keep the same syntax and the same meaning. Each one is
still a paragraph with `Strong` inlines, which every reader of formats 1 to 3 accepts. Only the
content and its order change. `SCHEMA_VERSION` and the corpus `format` stay. #88 adds the canonical
order as a corpus check for class options, so the renderer's input stays fixed.

**Rejected alternatives.**

- A dedicated `::: power` fence would type every part. It needs a new `Block` variant
  (`SCHEMA_VERSION`), a corpus format number, a `migrate` step and a mechanical migration pull
  request. The labeled rows already carry every part the layout needs. Reconsider it when a format
  batch happens for other reasons.
- Power names as corpus headings would add about 3,900 headings and fill `On this page`. They would
  also shift the derived `sec-` keys of unanchored headings that share a slug.
- Renderer wording that prefixes `At 11th level, ` and lowercases the corpus sentence fails on
  proper nouns: 136 upgrade sentences start with a word that keeps its capital (`Aberrant`, `Aeon`,
  other creature and spell names).

### 2. How the renderer recognizes a power

The renderer applies the layout only on `class_option` entries. A power is a bold-only paragraph
followed directly by a row labeled `Gained`, `Flavor` or an effect label, or by an upgrade lead.
The run continues over rows, upgrade leads and unlabeled paragraphs, up to the last row or lead.
The run must hold a `Gained` row, an effect row or an upgrade lead. Any other shape renders exactly
as today, so a power that #88 has not yet rewritten degrades to the current rows. It never shows a
broken layout.

The rule excludes the affliction blocks inside powers (`**Poison**` then `Type`, `Save`,
`Frequency`, `Effect`, `Cure`), which is why `Effect` is not an effect label. A looser rule that
accepts any power label counted 10 such blocks as powers. The same row shape occurs in archetype
entries (4,702 runs) and in 206 rule entries. They are out of scope here (see open questions).

**Counts on corpus `main` (cast-iron-data, after v0.1.26).** Ground truth is the pinned upstream
dataset: every `::ab[...]` directive that carries a gained level, an effect, an upgrade or a usage,
2,248 powers in 617 class-option entries. A prototype script applies the canonical rewrite in memory
and then runs the rule above:

| Corpus state | Recognized | Found of 2,248 | Missed | Extra | Extra that are not powers |
|---|---|---|---|---|---|
| Today, rule as written | 3,260 | 1,800 | 448 | 11 | 0 |
| After the canonical rewrite (prototype) | 3,887 in 2,106 entries | 2,248 | 0 | 30 | 0 |

- Each of the 30 extras was checked by hand. All are real powers that the upstream writes in another
  form (an `action=` attribute, or the same name for a power and its poison), so the ground truth
  misses them. The 11 extras on today's corpus are a subset.
- The 448 misses on today's corpus are mostly the import-sorted powers and the 64 magus arcana whose
  name ends in a colon. #88 fixes both.
- 1,609 recognized powers sit in entries that have no upstream `::ab` ground truth: talents, rage
  powers and hexes. In a random sample of 40, all 40 are real powers.
- The prototype queues 234 powers for review in #88: 187 with more than one effect row
  (`Ability` + `Immediate`, `Ability2`), 136 upgrades whose first word keeps its capital, and 5
  upgrades not above the gained level.

Other counts after the rewrite: 1,138 powers have `Gained`, 1,000 have an action label, 712 have
`Usage`, 770 have upgrades (1,729 lines), 388 have a prerequisite that is only a level
(`7th-level oracle`), and 1,601 entries consist of a single power whose name is the entry title.
Only about 20 of the 1,000 action-labeled effects state the action in their own text. The import
moved `As a standard action` into the label, so the label must show.

### 3. What the layout shows

- **Head.** The name on the left. On the right, `Level N` from `Gained`, or `Level N+` from a
  prerequisite that is only a level, whose row is then not repeated. A dotted leader in `--line`
  joins the name and the level. A power with neither has no right slot (most revelations).
- **Meta line.** One muted line under the head, in the Metadata role (`--ink-muted`, `--step--1`, `·`
  separators). It holds the action as words (`Standard action`, `Swift action`, `Full-round action`)
  and the `Usage` text: `Standard action · 3 + Intelligence modifier times/day`. A power with neither
  has no meta line.
- **Passive and Ability show no label.** The sketch shows none, and the effect text makes a passive
  power plain. `Flavor` also loses its label and reads as the first sentence, as in print.
- **Base effect**, then its continuation paragraphs, in body type.
- **Upgrades**, one paragraph each, with the lead `At 11th level,` in weight 600. The rest of the
  sentence is in body weight.
- **Other rows** keep today's bold label (`Special:`, `Prereq:`, `Replaces:`, `Penalty:`).
- No box, no tint and no left rule (the stat block keeps the only rule). Space separates powers:
  `--space-5` between powers, `--space-1` under the head.

### 4. Accessibility

- The name is a real heading element. Its level is one below the nearest preceding body heading.
  Before any body heading, the level is the shallowest heading level in the body, or `h2` when the
  body has none. The level is capped at `h6`. So the Abjuration powers are `h3` siblings of
  `### Banishment School`, and that school's powers are `h4`. The class `.power-name` fixes the
  look whatever the level.
- These headings are not added to `On this page`; the outline keeps the corpus headings only.
  They are not added to search or to heading keys, and they get no anchor.
- Reading order is the visual order: name, level, meta line, effect, upgrades. The level is text,
  never position only. It carries a visually hidden prefix (`.sr-only`): "Gained at Level 1", or
  "Available from Level 7+". The leader is decorative (`aria-hidden`, or a pseudo-element).
- The power is a plain `div`, not a `section`, so a mystery with 20 revelations adds no landmarks.
- Colors are existing tokens with their measured contrast. The meta line uses `--ink-muted`, the
  token that the entry metadata line already uses.

### 5. At 390 px

The head is one flex row aligned on the baseline. The name shrinks and wraps (`text-wrap:
balance`). The level never wraps and stays on the name's first line at the right edge. The leader
takes only the space left, so it disappears when the name wraps. In the mockup, "In Accordance with
the Prophecy (Su)" wraps to two lines with `Level 1` beside the first. The meta line wraps like
prose.

## Evidence

Mockups at 390×844 and 1280×900, light and dark, are attached to the pull request for this file.
The mockup HTML inlines `web/styles.css` and renders real corpus entries in the prototype canonical
form: Abjuration (*Core Rulebook* p. 79), Flame (*Advanced Player's Guide* p. 47), Divination
(*Core Rulebook* p. 80), and the Evil Eye hex (*Advanced Player's Guide*). The same pages also render as
the app does today, for comparison. The counting and rewrite prototype moves to
cast-iron-data with #88 as the rerunnable script.

## Then

1. phynics/cast-iron-data#88: the rewrite script and its review queue, a print check on a sample,
   the canonical-order check, and a corpus tag.
2. A pin pull request here. Then the renderer: recognition in `crates/web` (pure functions with
   native tests), the `.power` styles, DESIGN.md set from proposed to current, and one e2e spec on
   Abjuration.
