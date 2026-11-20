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
3. `**Flavor:**`: optional, at most one.
4. The base effect: exactly one row labeled `Passive`, `Ability`, `Standard`, `Move`, `Swift`,
   `Immediate`, `Free`, `Full-round` or `Benefit`.
5. The effect's continuation: unlabeled paragraphs, only here, directly after the effect row.
6. `**Usage:**`: optional, at most one.
7. Upgrades, in strictly ascending level, each above the `Gained` level: `**At 11th level,** <printed
   sentence>`.
8. `**Prereq:**`, then `**Replaces:**`: optional, at most one each.
9. Other rows (`Special`, `Penalty`, `Functions As`, …) in their printed order.

A power extends to the next structural boundary: the next name, a heading, a list, a table, a fence,
a quote, a stat block, an entry link, or the end of the body. An unlabeled paragraph is part of the
power only in position 5. That position is how the canonical form marks a trailing continuation
(Foretell has three effect paragraphs). Prose that does not belong to the power must be separated
from it by a boundary. A paragraph anywhere else in the run makes the run invalid (section 2).

The two changes of form are the upgrade lead and the name without a colon. The upgrade line holds
the printed wording, so the corpus produces it, not the renderer. The bold lead `At 11th level,` is
the typed marker (a `Strong` inline at the start of the paragraph) that the renderer reads. A
renderer that does not know the layout still shows correct prose. The rewrite in #88 also folds the
import's other shapes into this one, but only where nothing is guessed. Anything else is queued for
print review, and its run stays exactly as it is until a reviewer resolves it:

- `**Action:** <value>` uses an explicit map, with no catch-all:

  | Value | Becomes |
  |---|---|
  | `Ability`, `ability` | `Ability` |
  | `Passive Ability` | `Passive` |
  | `Standard Action`, `Standard`, `Standard.` | `Standard` |
  | `Move Action` | `Move` |
  | `Swift Action` | `Swift` |
  | `Immediate Action` | `Immediate` |
  | `Free Action` | `Free` |
  | `Full-Round Action`, `Full-round.` | `Full-round` |
  | `At Nth Level` | a `Gained` row |

  Any other value is queued. On corpus `main` these are `Move-Equivalent Action` (One with Nature,
  fey bloodline), `Benefit`, `Choice`, `Free Action=` and `Info`. The effect text of an `Action`
  row is the run's one group of unlabeled paragraphs after it. The first paragraph becomes the
  effect row's text, and the rest become its continuation. A run with no such group, or with more
  than one group, is queued.
- `Fullround` becomes `Full-round`, `Passive Ability` becomes `Passive`, and `Standard Action`
  becomes `Standard`.
- Old `**At Nth Level:**` rows and plain-text `At 8th level: These bonuses …` paragraphs become
  upgrade leads.
- Rows that repeat an earlier row exactly are removed (the import doubles some `Usage` rows).
- Queued, never guessed:
  - a numbered effect label (`Ability2`, as in Coven);
  - no effect row, or more than one;
  - conflicting singleton rows (two different `Gained`, `Flavor`, `Usage`, `Prereq` or `Replaces`
    rows);
  - unlabeled paragraphs that are not one group directly after the effect row;
  - two upgrades at one level, or an upgrade not above `Gained`;
  - an upgrade whose first word is not on the lowercase list.

  A power with no effect row and no `Gained` row but with upgrades (Fire Resistance of the Fire
  domain prints `At 6th level, you gain resist fire 10.`) is queued with a suggestion: make its
  lowest upgrade the `Gained` level and the base effect.
- The renderer never changes the case of corpus text. The rewrite lowercases the first word of an
  upgrade only when that word is on a fixed list (see "Rewrite and survey algorithm").

**Not a corpus format change.** The files keep the same syntax and the same meaning. Each one is
still a paragraph with `Strong` inlines, which every reader of formats 1 to 3 accepts. Only the
content and its order change. `SCHEMA_VERSION` and the corpus `format` stay. #88 adds the canonical
validation (section 2) as a corpus check for class options, so the renderer's input stays fixed.

**Rejected alternatives.**

- A dedicated `::: power` fence would type every part. It needs a new `Block` variant
  (`SCHEMA_VERSION`), a corpus format number, a `migrate` step and a mechanical migration pull
  request. The labeled rows already carry every part the layout needs. Reconsider it when a format
  batch happens for other reasons.
- Power names as corpus headings would add about 3,900 headings and fill `On this page`. They would
  also shift the derived `sec-` keys of unanchored headings that share a slug.
- Renderer wording that prefixes `At 11th level, ` and lowercases the corpus sentence fails on
  proper nouns: 164 upgrade sentences start with a word that keeps its capital (`Aberrant`, `Aeon`,
  `Anytime`, other creature and spell names).

### 2. Candidates and the one canonical validation

Detection and validation are separate steps. Both read the parsed corpus blocks, never text split
on blank lines:

- **Block kinds.** Each top-level block of the body is one of the following. The first match wins,
  so an empty `**Usage:**` paragraph is a row and never a name.

  | Kind | Test |
  |---|---|
  | Lead | a normal paragraph whose first inline is a `Strong` with exactly the text `At Nth level,` |
  | Row | a normal paragraph whose first inline is a `Strong` whose text ends with `:`. The label is that text without the colon, and the row's content is the remaining inlines, possibly none |
  | Name | a normal paragraph whose only inline is one `Strong` of plain text that does not end with `:` |
  | Boundary | a heading, list, table, quote, stat block, entry link or styled paragraph |
  | Paragraph | any other paragraph |

- **Candidate.** A name directly followed by a row or a lead. The run extends to the next boundary
  or name. It must hold a power marker: a lead, a plain-text upgrade paragraph, or a row labeled
  `Gained`, `Action`, an effect label, an alias, a numbered effect label or an old upgrade label.
  Affliction blocks (`Type`, `Save`, `Frequency`, `Effect`, `Cure`) hold no marker, so they are not
  candidates. That is also why `Effect` is not an effect label.
- **Validation.** One function checks a candidate against the canonical order of section 1. It
  rejects:
  - each rule in section 1 that the run breaks;
  - any legacy label (`Action`, an alias, `Ability2`, `At Nth Level:`);
  - a missing effect row, or more than one;
  - a second singleton row;
  - a paragraph outside position 5.

  Firestorm today (`Flavor`, `Prereq`, `Standard`, `Usage`, `Usage`) fails it. After the rewrite
  (`Flavor`, `Standard`, `Usage`, `Prereq`) it passes.
- **One function, two users.** The corpus check (#88) reports every candidate that fails
  validation. The renderer lays out a candidate only when validation passes. A candidate that fails
  renders exactly as today's labeled rows, so a queued power never shows a broken layout. The
  renderer applies this only on `class_option` entries.

**Counts on corpus `main` (cast-iron-data at `c265325e`, after tag v0.1.26).**

- **Ground truth.** 3,707 upstream powers in 2,047 class-option entries. The method is in
  "Rewrite and survey algorithm". 772 class options have no upstream record with a power directive
  and are left out of the comparison.
- **Candidate detection on today's text.** It finds 3,884 candidates in 2,102 entries. Against the
  ground truth it finds 3,704, misses 3 and adds 3:
  - The 3 misses are sub-options of the Utility Shot deed that the corpus writes as empty rows
    (`**Blast Lock:**`). They are rows by the precedence rule, so they are correctly not names.
  - The 3 extras are real powers. Heresy and Necromantic Arcana carry spell keys upstream, and
    Student of Humanity is written differently upstream.
- **Validation on today's text.** 1,720 candidates already pass, so they get the layout with no
  rewrite.
- **Rewrite prototype.** It rewrites 1,652 candidates and queues 512, which it leaves unchanged.
  After the rewrite, 3,372 candidates in 1,918 entries pass validation. A second pass changes
  nothing (idempotent).
- **Queue reasons.** A power can have several:

  | Reason | Powers |
  |---|---|
  | No effect row (195 of them with the lowest-upgrade suggestion) | 213 |
  | More than one effect row | 187 |
  | Upgrade text keeps a capital | 164 |
  | Numbered effect label | 62 |
  | Unlabeled paragraphs not one group directly after the effect row | 45 |
  | Unmapped `Action` value | 5 |
  | Upgrade not above `Gained` | 5 |
  | `Action` row without one effect paragraph group | 2 |
  | Two upgrades at one level | 2 |

- **Validated runs after the rewrite.** 813 have `Gained`, 902 have an action label, 671 have
  `Usage`, and 644 have upgrades (1,368 lines). 388 class-option powers have a prerequisite that is
  only a level (`7th-level oracle`); that prerequisite stays a `Prereq:` row (owner answer 2).
- **Action wording.** Only about 20 of the 1,000 or so action-labeled effects state the action in
  their own text. The import moved `As a standard action` into the label, so the label must show.

The same row shape occurs in archetype entries (5,915 candidates in 1,290 entries) and in rule
entries (349 in 79). #540 was opened with the earlier rule's counts (4,702 and 206). They are out
of scope here. Class options ship first, and
[#540](https://github.com/phynics/cast-iron/issues/540) extends the same rule and corpus rewrite
(owner answer 5).

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

Most talents, rage powers, hexes and discoveries are one power whose page is that power. On such a
page the power head folds into the title, so the name does not appear twice.

- **Recognition.** A page is a one-power page when all four hold:
  1. The body has exactly one candidate.
  2. That candidate passes validation.
  3. Its name is the first block of the body.
  4. The name without its ability tag equals the entry title, ignoring case and apostrophe style.
     The ability tag is a trailing `(Ex)`, `(Su)` or `(Sp)`, or a combination joined by `, `, ` or `
     or ` and ` (`(Ex or Su)`, `(Sp and Ex)`).

  Any other page keeps every power head, and a failing candidate keeps its labeled rows.
- **Counts after the rewrite prototype.** 2,102 entries hold a candidate. 1,603 have one candidate
  that is the first block and matches the title:
  - 1,492 of these pass validation and fold the head now;
  - 111 are queued and keep their labeled rows until resolved.

  464 entries have several candidates. 35 have one candidate that is not the entry, and those keep
  the head: 26 eidolon `Base Evolutions`, 2 inquisition `Granted Powers`, 5 spirits, 1 kineticist
  entry, and `Black Market Connection (Ex)` under the title `Black Market Connections`. The last one
  is a corpus mismatch for #88 to check against print.
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
  (15 of the validated pages) render as today.
- Of the 1,492 validated one-power pages, 1,152 have an ability tag and 220 have an action or uses.
  None has `Gained` yet: the one-power pages with a level, such as Flight, are in the queue with the
  lowest-upgrade suggestion.

### 5. Accessibility

- The name is a real heading element. A power before the first top-level body heading is `h2`.
  After it, a power is one level below the preceding top-level body heading, capped at `h6`. So the
  three Abjuration powers before `### Banishment School` are `h2`, and that school's powers are
  `h4`. The class `.power-name` fixes the look whatever the level.
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

The owner approved the revised mockups and this decision on 2026-11-20. A review of PR #538 on the
same day tightened the hand-off: the explicit `Action` map, continuation by position, one shared
validation, parser-based blocks, the survey method and the heading rule. The display choices did
not change.

## Evidence

The owner reviewed mockups at 390×844 and 1280×900, light and dark, on 2026-11-20. They are not
committed. The mockup HTML inlined `web/styles.css` and rendered real corpus entries through the
prototype. It covered:

- Abjuration (*Core Rulebook* p. 79);
- Flame (*Advanced Player's Guide* p. 47);
- Divination (*Core Rulebook* p. 80);
- three one-power pages: the Evil Eye and Flight hexes (*Advanced Player's Guide* p. 66) and the
  Auspicious Mark rage power (*Ultimate Combat* p. 26).

Abjuration, Flame and Evil Eye also rendered as the app does today, for comparison. In the
regenerated mockups, queued powers render as labeled rows (Counterspell Mastery, Forewarned). Flight
is shown with its lowest-upgrade suggestion accepted, to show a one-power page with a level.

The counts come from corpus `main` at `c265325e` (after tag v0.1.26) and the upstream pin in the
corpus `legal.toml`. The prototype's block parser was checked against the compiled library of the
pinned corpus (v0.1.26): for all 2,802 class options, it gives the same sequence of block kinds as
the corpus parser.

## Hand-off

1. **phynics/cast-iron-data#88** rewrites the class-option powers into the canonical order of
   section 1, with the printed upgrade sentences behind a bold lead, using a rerunnable script. Its
   scope:
   - the rewrite, applied to every class option, then `corpus-fmt`;
   - the print-review queue, 512 powers in the prototype (reasons in section 2). Each is checked
     against print. A queued run stays unchanged until its resolution is recorded. The script keeps
     resolutions in a committed file keyed by entry id and power name, so a rerun applies them and
     does not ask again;
   - the title mismatch `Black Market Connection (Ex)` under the entry title `Black Market
     Connections` (`talent/black-market-connections`). Print decides which spelling is right. The
     entry id never changes (rule 3);
   - a print check on a sample of rewritten powers;
   - the canonical validation of section 2 as a corpus check for class options, enabled once the
     queue is empty, and a corpus tag.
2. **This repository, #537**: a pin pull request for that tag, then the renderer. It adds pure
   functions with native tests in `crates/web`: the block kinds, the candidate test, the shared
   validation, the one-power test and the heading level. It also adds the `.power` and
   `.entry-facts` styles and one e2e spec on Abjuration. The validation is the same rule set as the
   corpus check. If the code is shared, it lives in a crate that both the corpus check and
   `crates/web` already depend on.
3. **[#540](https://github.com/phynics/cast-iron/issues/540)**: archetype features and rule entries,
   after class options ship.

## Rewrite and survey algorithm

The prototype is not committed. It belongs in cast-iron-data with #88, next to
`scripts/survey-class-option-residue.py`. This section specifies it closely enough to reimplement.

**Blocks.** Use the corpus parser (`crates/corpus`) and classify its top-level blocks as in
section 2. Do not split text on blank lines: lists with indented items, fences and quotes contain
blank lines. The rewrite moves whole blocks and whole inline sequences: links, emphasis and hard
breaks stay as they are. Lowercasing changes only the first word of the first `Text` node. The
corpus writer then serializes the result.

**Legacy names.** For the rewrite only, an empty row is treated as a name written with a colon
when it is the first block of the body or its label ends in an ability tag. This covers the 64
magus arcana such as `**Accurate Strike (Ex):**`, and the rewrite drops the colon. The renderer
never treats a row as a name.

**Vocabulary.**
- Effect labels: `Passive`, `Ability`, `Standard`, `Move`, `Swift`, `Immediate`, `Free`,
  `Full-round`, `Benefit`.
- Aliases: `Fullround` → `Full-round`, `Passive Ability` → `Passive`, `Standard Action` → `Standard`.
- Numbered effect labels: `Ability`, `Passive` or `Benefit` followed by digits.
- Old upgrade label: `At Nth Level`. Plain-text upgrade paragraph: `At Nth level: <text>`.
- `Gained` value: `At Nth Level`. Singletons: `Gained`, `Flavor`, `Usage`, `Prereq`, `Replaces`.
- Lowercase list for the first word of an upgrade: `You`, `Your`, `The`, `This`, `These`, `That`,
  `Those`, `Its`, `It`, `A`, `An`, `Any`, `All`, `Each`, `In`, `Once`, `When`, `Whenever`, `While`,
  `If`, `As`, `For`, `Add`, `By`, `She`, `He`, `Her`, `His`, `They`, `Their`, `Instead`,
  `Additionally`, `Also`, `Every`, `Allies`.

**Rewrite one candidate.** If it passes validation, leave it unchanged. Otherwise:
1. Map aliases. Old upgrade labels and plain-text upgrade paragraphs become upgrades (level and
   content). `Action` rows follow the explicit map of section 1. A numbered effect label is queued.
2. Remove rows that repeat an earlier row exactly. Queue conflicting singletons.
3. Count the effect rows, including a mapped `Action` row. Queue when there are none, adding the
   lowest-upgrade suggestion when the run has upgrades and no `Gained` row. Queue when there is
   more than one.
4. Group the unlabeled paragraphs into runs of consecutive paragraphs.
   - With a mapped `Action` row, there must be exactly one group, after that row. Its first
     paragraph is the effect text, and the rest is the continuation.
   - Otherwise there must be no group, or exactly one group directly after the effect row; that
     group is the continuation.
   - Anything else is queued.
5. Sort the upgrades by level. Queue duplicates, upgrades not above `Gained`, and upgrades whose
   first word is not on the lowercase list. Lowercase the others.
6. If anything was queued, return the candidate unchanged together with its reasons. Otherwise emit
   it in the order of section 1, validate the result, and replace the candidate.

**Ground truth (survey only).**
- Read every JSON file of the upstream pin.
- In each record's `description`, a line counts as a power when its attribute keys include any of
  `l`, `action`, `passive`, `ability`, `standard`, `swift`, `immediate`, `free`, `move`,
  `fullround`, `benefit`, `usage`, `imp<N>`, `use…`, `increment…` or `l<N>`, and no `s<N>` (a spell
  list). A line has one of three forms:
  - `::ab[Name]{attrs}`;
  - `::ab{title="Name" …}`;
  - `:::ab{title="Name" …}`.
- Drop formatting tokens such as `&L&` and `&FN&` from titles.
- An entry's records are its `provenance` rows, deduplicated by `path` and `record`. Only records
  that exist upstream and hold at least one power count. An entry with no such record stays out of
  the comparison totals.
- Normalize names: lowercase, `’` read as `'`, runs of spaces folded, a trailing `:` dropped.
- Per entry, match names one occurrence at a time: a name that appears twice upstream needs two
  candidates.
