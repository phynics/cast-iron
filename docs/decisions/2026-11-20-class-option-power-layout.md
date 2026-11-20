---
title: "Class-option powers render as printed prose from canonical labeled rows"
date: 2026-11-20
status: current
superseded_by: []
issues: [537, 540]
prs: [538]
area: ui
---

# Class-option powers render as printed prose from canonical labeled rows, 2026-11-20

Refs [#537](https://github.com/phynics/cast-iron/issues/537) and phynics/cast-iron-data#88. The owner
answered the open questions on 2026-11-20 (see "Owner answers" below) and approved the revised design
on 2026-11-20. The implementation order is in "Hand-off".

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
entries (4,702 runs) and in 206 rule entries. They are out of scope here. Class options ship first;
[#540](https://github.com/phynics/cast-iron/issues/540) extends the same rule and corpus rewrite to
archetypes and rule entries (owner answer 5).

**Counts on corpus `main` (cast-iron-data, after v0.1.26).** Ground truth is the pinned upstream
dataset: every `::ab[...]` directive that carries a gained level, an effect, an upgrade or a usage,
2,248 powers in 617 class-option entries. A prototype script applies the canonical rewrite in memory
and then runs the rule above (algorithm in "Rewrite and survey algorithm"):

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
`Usage`, 770 have upgrades (1,729 lines), and 388 have a prerequisite that is only a level
(`7th-level oracle`). That prerequisite stays a `Prereq:` row (owner answer 2).
Only about 20 of the 1,000 action-labeled effects state the action in their own text. The import
moved `As a standard action` into the label, so the label must show.

### 3. What the layout shows

- **Head.** The name on the left. On the right, `Level N` from `Gained`, and from nothing else. A
  prerequisite such as `11th-level oracle` stays a `Prereq:` row and never sets the level. A dotted
  leader in `--line` joins the name and the level. A power without `Gained` has no right slot (most
  revelations). The level is a presentation detail that groups and orders the powers of a page; it is
  never part of the power's name.
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

### 4. One-power pages

1,606 class-option entries are one power: a talent, rage power, hex or discovery whose page is that
power. On such a page the power head folds into the title, so the name does not appear twice.

- **Recognition.** A page is a one-power page when all three hold, after the rewrite:
  1. The body has exactly one recognized power.
  2. The power's name paragraph is the first block of the body.
  3. The name without its ability tag equals the entry title, ignoring case and apostrophe style.
     The ability tag is a trailing `(Ex)`, `(Su)` or `(Sp)`, or a combination joined by `, `, ` or `
     or ` and ` (`(Ex or Su)`, `(Sp and Ex)`).

  Any other page is a multi-power page and keeps every power head. On corpus `main` after the
  prototype rewrite: 2,106 entries hold a power, 1,606 are one-power pages, 465 have several powers,
  and 35 have one power that is not the entry. Those 35 keep the head: 26 eidolon `Base Evolutions`,
  2 inquisition `Granted Powers`, 5 spirits, 1 kineticist entry, and `Black Market Connection (Ex)`
  under the title `Black Market Connections`. The last one is a corpus mismatch for #88 to check
  against print.
- **The title stays the entry title.** The `h1` text never takes the ability tag or the level.
- **Facts line.** The power's facts move into the entry head, as one muted line directly under the
  metadata line (`.entry-facts`, the Metadata role: `--ink-muted`, `--step--1`, `·` separators). The
  order is the ability tag in words with the printed tag (`Supernatural (Su)`), then `Level N` from
  `Gained`, then the action and the uses: `Supernatural (Su) · Swift action · Once per rage`. The
  metadata line keeps its own role (kind and book, as links). The facts line holds only text, so the
  two lines never mix links with plain facts. A page whose power has no tag, no `Gained`, no action
  and no uses has no facts line.
- **Body.** The body starts with the effect. It has no power heading, no leader and no power meta
  line. The upgrades and the other rows follow, as on a multi-power page. Blocks after the power
  (76 pages) render as today.
- Of the 1,606 pages, 1,229 have an ability tag, 19 have `Gained`, and 250 have an action or uses.

### 5. Accessibility

- The name is a real heading element. Its level is one below the nearest preceding body heading.
  Before any body heading, the level is the shallowest heading level in the body, or `h2` when the
  body has none. The level is capped at `h6`. So the Abjuration powers are `h3` siblings of
  `### Banishment School`, and that school's powers are `h4`. The class `.power-name` fixes the
  look whatever the level.
- These headings are not added to `On this page`; the outline keeps the corpus headings only.
  They are not added to search or to heading keys, and they get no anchor.
- Reading order is the visual order: name, level, meta line, effect, upgrades. The level is text,
  never position only. It carries a visually hidden prefix (`.sr-only`): "Gained at Level 1". The
  leader is decorative (`aria-hidden`, or a pseudo-element).
- On a one-power page, the reading order is: title, metadata line, facts line, then the body. The
  facts line spells out the tag ("Supernatural (Su)"), so a screen reader does not have to read "S u".
- The power is a plain `div`, not a `section`, so a mystery with 20 revelations adds no landmarks.
- Colors are existing tokens with their measured contrast. The meta line uses `--ink-muted`, the
  token that the entry metadata line already uses.

### 6. At 390 px

The head is one flex row aligned on the baseline. The name shrinks and wraps (`text-wrap:
balance`). The level never wraps and stays on the name's first line at the right edge. The leader
takes only the space left, so it disappears when the name wraps. In the mockup, "In Accordance with
the Prophecy (Su)" wraps to two lines with `Level 1` beside the first. The meta line and the facts
line wrap like prose.

## Owner answers, 2026-11-20

1. One-power pages: fold the power head into the title. The tag and the level are presentation near
   the title, never title text (section 4).
2. Minimum level of a revelation: keep the `Prereq:` row. The head shows a level only from `Gained`.
   The head never derives a level such as `Level 7+` from a prerequisite.
3. Action and uses: one muted meta line. `Passive` shows nothing.
4. Dotted leader: kept.
5. Archetypes: class options first; [#540](https://github.com/phynics/cast-iron/issues/540) extends the
   rule to archetypes and rule entries.

The owner approved the revised mockups and this decision on 2026-11-20.

## Evidence

The owner reviewed mockups at 390×844 and 1280×900, light and dark, on 2026-11-20. They are not
committed. The mockup HTML inlined `web/styles.css` and rendered real corpus entries in the prototype canonical
form: Abjuration (*Core Rulebook* p. 79), Flame (*Advanced Player's Guide* p. 47), Divination
(*Core Rulebook* p. 80), and three one-power pages: the Evil Eye and Flight hexes (*Advanced Player's
Guide*) and the Auspicious Mark rage power (*Ultimate Combat*). Abjuration, Flame and Evil Eye also
render as the app does today, for comparison. The counts come from corpus `main` at `c265325e`
(after tag v0.1.26) and the upstream pin in the corpus `legal.toml`.

## Hand-off

1. **phynics/cast-iron-data#88** rewrites the class-option powers into the fixed row order of section
   1, with the printed upgrade sentences behind a bold lead, using a rerunnable script. Its scope:
   - the rewrite below, applied to every class option, then `corpus-fmt`;
   - the hand-review queue: the prototype found 234 powers, namely 187 with more than one effect row,
     136 upgrades whose first word stays capitalized, and 5 upgrades not above the gained level.
     Each is checked against print. The script records the resolution, so a rerun does not ask again;
   - the title mismatch `Black Market Connection (Ex)` under the entry title `Black Market
     Connections` (`talent/black-market-connections`). Print decides which spelling is right. The
     entry id never changes (rule 3);
   - a print check on a sample of rewritten powers;
   - the canonical-order check for class options, and a corpus tag.
2. **This repository, #537**: a pin pull request for that tag, then the renderer. It adds pure
   functions with native tests in `crates/web` (power recognition, the one-power test, the heading
   level), the `.power` and `.entry-facts` styles, and one e2e spec on Abjuration.
3. **[#540](https://github.com/phynics/cast-iron/issues/540)**: archetype features (4,702 runs in 1,234
   entries) and rule entries (206 runs in 41 entries), after class options ship.

## Rewrite and survey algorithm

The prototype is not committed. It belongs in cast-iron-data with #88, next to
`scripts/survey-class-option-residue.py`. This section specifies it closely enough to reimplement.

**Blocks.** Read the body after the closing `+++`. Split it into blocks on blank lines. Join the lines
of a block with one space. A line ending in an odd number of `\` ends in a line break. Classify each
block, first match wins:

| Kind | Test |
|---|---|
| heading | the first line starts with `#` |
| other | the first line starts with `- `, `1. `, `\|`, `:::` or `>` |
| head | the whole block matches `^\*\*([^*]+)\*\*$` |
| up (canonical lead) | `^\*\*At (\d+)(st\|nd\|rd\|th) level,\*\* (.*)$` |
| row | `^\*\*([^*]+?):\*\*(?:\s+(.*))?$`: label and text |
| para | anything else |

**Vocabulary.**
- Effect labels: `Passive`, `Ability`, `Standard`, `Move`, `Swift`, `Immediate`, `Free`,
  `Full-round`, `Benefit`.
- Aliases: `Fullround` → `Full-round`, `Passive Ability` → `Passive`, `Standard Action` → `Standard`.
- Old upgrade label: `^At (\d+)(st|nd|rd|th) Level$`.
- Plain-text upgrade paragraph: `^At (\d+)(st|nd|rd|th) level: (.*)$`.
- `Gained` value: `^At (\d+)(st|nd|rd|th) Level$`.

**Find today's runs.** At each head, take the following rows, leads and paragraphs:
- A paragraph stays in the run when it directly follows an `Action` row, or when the next block
  after its group of consecutive paragraphs is a row or a lead. A plain-text upgrade paragraph always
  counts as a row.
- When the run holds an `Action` row whose value is not a `Gained` value, the run also takes the
  paragraphs after its last row, because that form puts the effect text last.
- The run ends after its last row.
- It is a power run when the block after the head is a row or a lead, and the run holds a
  plain-text upgrade or a label that is `Gained`, `Action`, an effect label, an alias, or an old
  upgrade label.

**Rewrite one run.**
1. Strip a trailing `:` from the name.
2. Map aliases.
3. Handle each `Action` row:
   - A value that is a `Gained` value becomes a `Gained` row.
   - Otherwise the row becomes an effect row. `Standard Action`, `Swift Action`, `Move Action`,
     `Immediate Action`, `Free Action`, `Full-Round Action` and `Passive Ability` map to their label;
     any other value maps to `Ability`. The row's text is the first later unlabeled paragraph of the
     run, which is removed from where it was. With no such paragraph, queue the power.
4. Turn old upgrade labels and plain-text upgrade paragraphs into upgrades (level, text).
5. Drop rows that repeat an earlier row exactly.
6. If the run has no effect row, no `Gained` row and at least one upgrade, the lowest upgrade becomes
   `Gained: At Nth Level` plus an `Ability` row with that upgrade's text.
7. Emit, in order:
   1. the name;
   2. `Gained`;
   3. `Flavor`;
   4. the effect rows, with all the run's unlabeled paragraphs directly after the first one (after
      `Flavor` when the run has no effect row);
   5. `Usage`;
   6. the upgrades by ascending level, as `**At Nth level,** <text>`;
   7. `Prereq`;
   8. `Replaces`;
   9. every other row in its original order.

   Lowercase the first word of an upgrade's text only when that word is one of `You`, `Your`, `The`,
   `This`, `These`, `That`, `Those`, `Its`, `It`, `A`, `An`, `Any`, `All`, `Each`, `In`, `Once`,
   `When`, `Whenever`, `While`, `If`, `As`, `For`, `Add`, `By`, `She`, `He`, `Her`, `His`, `They`,
   `Their`, `Instead`, `Additionally`, `Also`, `Every` or `Allies`. Queue every other upgrade.
8. Queue the power when it has more than one effect row, or an upgrade whose level is not above the
   `Gained` level.

**Recognize (the renderer's rule, on rewritten text).** A head is a power when the next block is a
lead or a row labeled `Gained`, `Flavor` or an effect label. The run extends over rows, leads and
paragraphs and ends at its last row or lead. The run must hold a `Gained` row, an effect row or a lead.
The one-power test is in section 4.

**Ground truth.** Read every `json/class_ability_*.json` of the upstream pin. In each record's
`description`, a `::ab[Name]{attrs}` line is a power when its attribute keys include any of `l`,
`passive`, `ability`, `standard`, `swift`, `immediate`, `free`, `move`, `fullround`, `benefit`,
`usage`, `imp<N>`, `use<Anything>`, `increment<anything>` or `l<N>`, and no `s<N>` (a spell list).
Match a corpus entry to its records through its `provenance` `path` and `record`. Compare names
case-insensitively, with `’` read as `'`, runs of spaces folded, and a trailing `:` dropped. Count
found, missed and extra names per entry.
