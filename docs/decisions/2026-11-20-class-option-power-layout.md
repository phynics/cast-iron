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
on 2026-11-20. The implementation order is in "Hand-off". The fixtures in "Fixtures" are the
acceptance cases for every implementation of this spec.

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
9. Other rows (`Special`, `Penalty`, `Functions As`, …), keeping their source order.

Every row has content. A power extends to the next structural boundary (section 2). An unlabeled
paragraph is part of the power only in position 5; that position marks it as the effect's
continuation. Elixir of Life is the example: an `Ability` row, one continuation paragraph, then
`Prereq`. Its wording matches print (*Advanced Player's Guide* p. 30), which sets it as one paragraph.
Prose that does not belong to the power must be separated from it by a boundary. A legacy plain-text
upgrade (section 2) is invalid in every position.

The two changes of form are the upgrade lead and the name without a colon. The upgrade line holds
the printed wording, so the corpus produces it, not the renderer. A renderer that does not know the
layout still shows correct prose.

**The rewrite in #88** folds the import's shapes into this form only where nothing is guessed. Any
other run is queued for print review with its whole span (name to boundary) unchanged:

- **Deduplication.** Only structurally identical original `Usage` rows are deduplicated. This
  happens before any alias mapping. No other row is ever deduplicated.
- **Aliases.** `Fullround` becomes `Full-round`, `Passive Ability` becomes `Passive`, and
  `Standard Action` becomes `Standard`. Mapping never merges rows: a `Fullround` row and an equal
  `Full-round` row are two effect rows, so the run is queued.
- **Upgrades.** Old `**At Nth Level:**` rows and legacy plain-text upgrades in the colon form become
  leads when their ordinal is well-formed. The comma form and ill-formed ordinals are queued.
- **`**Action:** <value>`** uses an explicit map, with no catch-all:

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
  | `At Nth Level` | a `Gained` row only (metadata: it never takes effect text) |

  Any other value is queued. On corpus `main` these are `Move-Equivalent Action`, `Benefit`,
  `Choice`, `Free Action=` and `Info`.
- **Effect text of a mapped `Action` row.** The run must hold exactly one group of consecutive
  unlabeled paragraphs. It must come after the `Action` row, and only rows labeled `Gained`,
  `Flavor`, `Usage`, `Prereq` or `Replaces`, each with content, may stand between them. The group's
  first paragraph becomes the effect row's content, and the rest become the continuation. Anything
  else is queued: a lead, a legacy upgrade, an empty row or any other row in between, a group
  before the row, or a second group.
- **Effect text of an ordinary effect row.** Unlabeled paragraphs must form no group, or exactly
  one group directly after the effect row. Anything else is queued.
- **Always queued:**
  - an empty row anywhere after the name: a probable sub-option heading, such as Utility Shot's
    `**Blast Lock:**`;
  - a numbered effect label (`Ability2`);
  - no effect row, or more than one;
  - conflicting singleton rows;
  - two upgrades at one level;
  - an upgrade not above `Gained`;
  - an upgrade whose first word is not on the lowercase list;
  - a seeded print item (Foretell, see "Fixtures").

  A power with no effect row and no `Gained` row but with upgrades is queued with a suggestion:
  make the lowest upgrade the `Gained` level and the base effect.
- **Order of the output.** The rewrite emits section 1's order. Rows in position 9 keep their
  source order. The proposed replacement is validated, and if it fails, the original run is queued.
- **Case.** The renderer never changes the case of corpus text. The rewrite lowercases the first
  word of an upgrade only when that word is on the lowercase list.

**Not a corpus format change.** The files keep the same syntax and the same meaning. Each one is
still a paragraph with `Strong` inlines, which every reader of formats 1 to 3 accepts. Only the
content and its order change. `SCHEMA_VERSION` and the corpus `format` stay.

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

### 2. Classification, candidates and the one validation

All three are functions over typed `Block`s in `cast-iron-schema`, the crate that both
`crates/corpus` and `crates/web` already depend on. There is one implementation (see "Hand-off").

**Level grammar.**
- A well-formed ordinal is a number from 1 to 20 with no leading zero, followed by its English
  suffix: `st` for 1, `nd` for 2, `rd` for 3, and `th` for 4 to 20.
- Detection is broad: `At ` + digits + one of `st`, `nd`, `rd`, `th` + a space + `Level` or
  `level`. Conversion and validation also require a well-formed ordinal.

**Block kinds.** Each top-level block is one of the following, and the first match wins:

1. **Lead:** a normal paragraph whose first inline is a `Strong` whose plain text is
   `At <digits><suffix> <L|l>evel,`. It is canonical only when the ordinal is well-formed, the word
   is `level`, and at least one inline follows.
2. **Row:** a normal paragraph whose first inline is a `Strong` whose plain text ends with `:`. The
   label is that text without the colon. The content is the remaining inlines, possibly none: an
   empty `**Usage:**` paragraph is a row, never a name.
3. **Name:** a normal paragraph whose only inline is one `Strong` of plain text that does not end
   with `:`.
4. **Legacy upgrade:** a normal paragraph whose first inline is a `Text` whose value starts with
   `At <digits><suffix> <L|l>evel` followed by `:` or `,`. Its upgrade text is the rest of that
   `Text` after the separator and one optional space, followed by the remaining inlines.
5. **Boundary:** a heading, list, table, quote, stat block, entry link or styled paragraph.
6. **Paragraph:** any other paragraph.

**Other values.**
- An old upgrade label is a row label `At <digits><suffix> <L|l>evel`.
- A `Gained` value is canonical when the row's content is the text `At <well-formed ordinal> Level`.

**Candidate.** A name, directly followed by a row, a lead or a legacy upgrade. The run extends to
the next boundary or name. It must hold a power marker: a lead, a legacy upgrade, or a row labeled
`Gained`, `Action`, an effect label, an alias, a numbered effect label or an old upgrade label.
Affliction blocks (`Type`, `Save`, `Frequency`, `Effect`, `Cure`) hold no marker; that is also why
`Effect` is not an effect label.

**Validation.** Checks a candidate against section 1. It rejects:
- each rule there that the run breaks;
- an empty row;
- a legacy upgrade in any position;
- any legacy label (`Action`, an alias, `Ability2`, `At Nth Level:`);
- a non-canonical lead or `Gained` value;
- a missing effect row, or more than one;
- a second singleton row;
- a paragraph outside position 5.

**One function, three users.**
- The corpus check reports the candidates that fail validation: first as a warning, later as an
  error (see "Hand-off").
- #88's rewrite script mirrors the classifier and the validator. The corpus check verifies its
  output.
- The renderer lays out a candidate only when validation passes. Otherwise it renders today's
  labeled rows. It applies this only on `class_option` entries.

**Counts on corpus `main` (cast-iron-data at `c265325e`, after tag v0.1.26).**

- **Ground truth.** 3,707 upstream powers in 2,047 class-option entries. The method is in
  "Rewrite and survey algorithm". 772 class options have no upstream record with a power and are
  left out.
- **Candidates today.** 3,884 in 2,102 entries.
  - Against the ground truth they find 3,704 powers and miss 3: Utility Shot's empty sub-option rows,
    which are rows, not names.
  - The 3 extras are real powers: Heresy and Necromantic Arcana carry spell keys upstream, and
    Student of Humanity is written differently there.
- **Already valid today.** 1,720 candidates.
- **Rewrite prototype.** It rewrites 1,640 candidates and queues 524, which it leaves unchanged.
  After the rewrite, 3,360 candidates in 1,909 entries pass validation. A second pass changes
  nothing.
- **Queue reasons.** A power can have several:

  | Reason | Powers |
  |---|---|
  | No effect row (195 of them with the lowest-upgrade suggestion) | 213 |
  | More than one effect row | 187 |
  | Upgrade text keeps a capital | 164 |
  | Numbered effect label | 62 |
  | Unlabeled paragraphs not one group directly after the effect row | 45 |
  | `Action` paragraph group crosses another row | 12 |
  | Unmapped `Action` value | 5 |
  | Upgrade not above `Gained` | 5 |
  | `Action` row without one paragraph group after it | 2 |
  | Two upgrades at one level | 2 |
  | Empty row (Utility Shot, 3 rows) | 1 |
  | Seeded print item (Foretell) | 1 |

- **Validated runs after the rewrite.** 810 have `Gained`, 898 have an action label, 667 have
  `Usage`, and 643 have upgrades (1,365 lines). 388 powers have a prerequisite that is only a level;
  it stays a `Prereq:` row (owner answer 2).
- **Action wording.** Only about 20 of the 1,000 or so action-labeled effects state the action in
  their own text, so the label must show.
- **Parser agreement.** The prototype classifies blocks exactly as the compiled library of the
  pinned corpus (v0.1.26) does, for all 2,802 class options.
- **Out of scope.** The same row shape occurs in archetype entries (5,915 candidates in 1,290
  entries) and in rule entries (360 in 79).
  [#540](https://github.com/phynics/cast-iron/issues/540) extends this spec after class options ship
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
- **Counts after the rewrite prototype.** 1,603 entries have one candidate that is the first block
  and matches the title:
  - 1,483 of these pass validation and fold the head;
  - 120 are queued and keep their labeled rows.

  464 entries have several candidates. 35 have one candidate that is not the entry, and those keep
  the head: 26 eidolon `Base Evolutions`, 2 inquisition `Granted Powers`, 5 spirits, 1 kineticist
  entry, and `Black Market Connection (Ex)` under the title `Black Market Connections`.
- **The title stays the entry title.** The `h1` text never takes the ability tag or the level.
- **Facts line.** The power's facts move into the entry head, as one muted line directly under the
  metadata line (`.entry-facts`, the Metadata role: `--ink-muted`, `--step--1`, `·` separators). The
  order is the ability tag in words with the printed tag (`Supernatural (Su)`), then `Level N` from
  `Gained`, then the action and the uses: `Supernatural (Su) · Swift action · Once per rage`. The
  metadata line keeps its own role (kind and book, as links). The facts line holds only text. A
  page whose power has no tag, no `Gained`, no action and no uses has no facts line.
- **Body.** Only the name and the facts move. The body starts with the optional `Flavor` paragraph
  (without its label), then the effect, its continuation, the upgrades and the other rows, as on a
  multi-power page. Auspicious Mark keeps its `Flavor` paragraph before the effect. Of the 1,483
  validated pages, 306 have `Flavor`, 1,144 have an ability tag, 217 have an action or uses, and 11
  have blocks after the power, which render as today.

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

## Fixtures

Each row is an acceptance test for the shared classifier and validator and for #88's rewrite.
The occurrence is the position among candidates with the same original name in that entry.
`canonical` means the candidate already passes validation and is left unchanged. `rewritten` means
the rewrite produces a run that passes. `queued` means the whole span stays unchanged and goes to
print review.

| Entry id | Original name | # | Expected | Why |
|---|---|---|---|---|
| `ref/discovery/elixir-of-life` | Elixir of Life | 1 | canonical | Effect, one continuation paragraph, `Prereq`; wording matches *Advanced Player's Guide* p. 30; a one-power page |
| `ref/arcaneschool/abjuration` | Resistance (Ex) | 1 | rewritten | Upgrades move after the effect, ascending |
| `ref/arcaneschool/abjuration` | Counterspell Mastery | 1 | queued | Two effect rows (`Ability`, `Immediate`) |
| `ref/aspect/giant-wasp` | Minor Form | 1 | rewritten | Two legacy plain-text upgrades after `Passive` become leads |
| `ref/deed/utility-shot` | Utility Shot (Ex) | 1 | queued | Three empty sub-option rows (`Blast Lock`, `Scoot Unattended Object`, `Stop Bleeding`). The span runs from the name to the end of the body. Disposition: #88 decides from print whether the sub-options become names of their own |
| `ref/blrgbloodline/fey` | One with Nature (Su) | 1 | queued | `Action: Move-Equivalent Action` is not in the map |
| `ref/hex/coven` | Coven (Ex) | 1 | queued | Numbered effect label `Ability2` |
| `ref/mystery/flame` | Firestorm (Su) | 1 | rewritten | `Prereq` moves after `Usage`; the identical second `Usage` row goes |
| `ref/mystery/reaper` | Haunt Channeler (Su) | 1 | canonical | Already valid |
| `ref/mystery/reaper` | Haunt Channeler (Su) | 2 | queued | `Ability2` and two effect rows; the occurrence tells it from the first |
| `ref/arcaneschool/divination` | Foretell (Su) | 1 | queued | Seeded print item: the corpus says `1 time/day per wizard level`, while print gives a number of rounds per day equal to the wizard level, not consecutive. The Foresight School starts on *Advanced Player's Guide* p. 145, and Foretell is on p. 146 |
| `ref/arcaneschool/divination` | Forewarned (Su) | 1 | queued | Upgrade starts with `Anytime`, which is not on the lowercase list |
| `ref/ragepower/auspicious-mark` | Auspicious Mark (Su) | 1 | rewritten | `Flavor`, effect, `Usage`, then `Special`; the identical second `Usage` row goes. A one-power page that keeps its `Flavor` paragraph |
| `ref/hex/evil-eye` | Evil Eye (Su) | 1 | rewritten | `Flavor` moves before the effect; the upgrade becomes a lead; a one-power page |
| `ref/hex/flight` | Flight (Su) | 1 | queued | No effect row; suggestion: the lowest upgrade becomes the base |
| `ref/domain/fire` | Fire Resistance (Ex) | 1 | queued | No effect row; suggestion: the lowest upgrade becomes the base |
| `ref/hex/blight` | Blight (Su) | 1 | queued | The `Action` paragraph group would cross a `Special` row |
| `ref/arcana/accurate-strike` | Accurate Strike (Ex): | 1 | rewritten | The name loses its colon; a one-power page |
| `ref/talent/black-market-connections` | Black Market Connection (Ex) | 1 | rewritten | The name differs from the title `Black Market Connections`, so the head stays; the spelling is a separate print item for #88 |
| synthetic S1 | name, `**Fullround:** X`, `**Full-round:** X` | 1 | queued | Two effect rows; alias mapping never merges rows |
| synthetic S2 | name, `**Ability:** X`, `**Usage:**` (empty) | 1 | queued | An empty row; it is a row, never a name |
| synthetic S3 | a canonical run | 1 | canonical | Rerunning the rewrite on its own output changes nothing |

No corpus instance of S1 exists on `c265325e`. The only structurally identical duplicates there
are `Usage` rows, which is why deduplication stops at them.

## Owner answers, 2026-11-20

1. One-power pages: fold the power head into the title. The tag and the level are presentation near
   the title, never title text (section 4).
2. Minimum level of a revelation: keep the `Prereq:` row. The head shows a level only from `Gained`.
   The head never derives a level such as `Level 7+` from a prerequisite.
3. Action and uses: one muted meta line. `Passive` shows nothing.
4. Dotted leader: kept.
5. Archetypes: class options first; [#540](https://github.com/phynics/cast-iron/issues/540) extends the
   rule to archetypes and rule entries.

The owner approved the revised mockups and this decision on 2026-11-20. Two reviews of PR #538 on
the same day tightened the hand-off: the rewrite rules, the level grammar, one shared validation,
the fixtures and the order of the work. The display choices did not change.

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
regenerated mockups, queued powers render as labeled rows (Counterspell Mastery, Foretell,
Forewarned). Flight is shown with its suggestion accepted, to show a one-power page with a level.
The counts come from corpus `main` at `c265325e` and the upstream pin in the corpus `legal.toml`.

## Hand-off

The work runs in this order, so that one classifier and one validator exist before anything
depends on them:

1. **App pull request (#537, first part).** It adds the shared classifier, candidate test,
   validation and one-power test over typed `Block`s to `cast-iron-schema`, with native tests built
   from "Fixtures". The class-option corpus check calls them and reports failing candidates as
   warnings, which do not fail the run.
2. **phynics/cast-iron-data#88.** It rewrites the class options with a script that mirrors those
   functions, and the warning report checks the result. Its scope:
   - the rewrite and `corpus-fmt`;
   - the print-review queue: 524 powers in the prototype;
   - a print check on a sample;
   - the `Black Market Connection(s)` spelling;
   - a corpus tag.

   **Resolutions** are recorded in a committed file. Each is keyed by:
   - the entry id;
   - the original name as written in the source (with any colon);
   - the occurrence among candidates with that name in the entry;
   - the SHA-256 of the original span's source text.

   A resolution applies only when the current span matches that hash exactly. A span that already
   equals the recorded replacement is a no-op. Any other span is reported as a stale resolution and
   queued again. Entry ids never change (rule 3).
3. **Enforcement (app pin pull request).** It pins the migrated corpus tag and turns the warning
   into an error. The check then fails on any class-option candidate that fails validation, except
   the ones still queued in #88's resolution file.
4. **Renderer (#537, app layout).** `crates/web` calls the same functions for the layout gate, the
   one-power test and the heading level. It adds the `.power` and `.entry-facts` styles and one e2e
   spec on Abjuration.

Then [#540](https://github.com/phynics/cast-iron/issues/540) extends the spec to archetype features
and rule entries.

## Rewrite and survey algorithm

The prototype is not committed. Section 1 specifies the rewrite, section 2 the classifier and the
validator, and "Fixtures" the expected outcomes. This section adds what an implementer still
needs.

**Inline handling.**
- The rewrite moves whole blocks and whole inline sequences: links, emphasis and hard breaks stay
  as they are.
- Converting a legacy upgrade keeps every inline after the separator.
- Lowercasing changes only the first word of the first `Text` node.
- The corpus writer serializes the result.

**Legacy names.** For the rewrite only, an empty row is treated as a name written with a colon
when it is the first block of the body or its label ends in an ability tag. This covers the 64
magus arcana such as `**Accurate Strike (Ex):**`, and the rewrite drops the colon. The renderer
never treats a row as a name.

**Lowercase list** for the first word of an upgrade: `You`, `Your`, `The`, `This`, `These`, `That`,
`Those`, `Its`, `It`, `A`, `An`, `Any`, `All`, `Each`, `In`, `Once`, `When`, `Whenever`, `While`,
`If`, `As`, `For`, `Add`, `By`, `She`, `He`, `Her`, `His`, `They`, `Their`, `Instead`,
`Additionally`, `Also`, `Every`, `Allies`.

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
- Per entry, match names one occurrence at a time.
