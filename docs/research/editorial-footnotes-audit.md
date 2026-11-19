# Editorial footnotes audit

Checked 2026-11-19 against corpus pin v0.1.28 (`phynics/cast-iron-data` commit
`24d393a23a8d4edb45f2382f12cbf9fe40c831f2`). Investigation for
[cast-iron-data #109](https://github.com/phynics/cast-iron-data/issues/109). No
corpus file was changed. This is a source and evidence audit, not legal
clearance.

**Owner rule (2026-11-19).** A footnote stays when its text appears on Archives
of Nethys (AoN) or in the Paizo PRD, whether a book printed it or an AoN or PRD
editor wrote it. A footnote that neither carries goes, with an
`exclusions.toml` row. Every footnote that stays is shown de-emphasized, so a
reader can tell it is not rules text: the app shows footnotes as a quiet Notes
group since [cast-iron #509](https://github.com/phynics/cast-iron/pull/509)
(merged). The `on_aon_prd` column applies this rule. The printed/editorial
classification stays as supporting evidence.

**Verdicts.** `yes`: one named AoN page or PRD document carries the WHOLE
note as written, and the TSV names it in `carried_by`; most of the note is
not enough. `no`: AoN does not carry it AND
the PRD check ran and found it in no document; both checks must say no.
`candidate`: needs review before anything is removed (a fuzzy or partial
match, a paraphrase, or only one of the two checks could run). `unchecked`:
neither check could run.

## Summary

- The corpus has **1,284 footnote blocks in 664 entries**. All of them came
  from the Pathfinder Data 1e dataset; none came from PSRD-Data.
- **On AoN or the PRD (owner rule): 716 yes, 138 no, 391 candidate, 39
  unchecked.** 716 blocks in 335 entries stay (shown de-emphasized). 138
  blocks in 105 entries are confirmed absent from both AoN and the PRD and
  go. 391 blocks in 302 entries need review, and 39 blocks in 12 entries could
  not be checked at all.
- The 138 confirmed "no" blocks are 73 Pathfinder Data commentary notes (all
  nine *Summon Monster* spells among them, 18 blocks), 55 page citations, 3
  pointers, 1 compiled addition, 3 printed notes, and 3 unclear notes.
- **Most candidates are "Bonus feat." legends.** 171 of the 172 blocks whose
  AoN page lacks the note but whose PRD check could not run (a note under four
  words) are standalone "Bonus feat." legends. The owner decides how to treat
  them; see [Owner decisions](#owner-decisions). Another 131 candidates have
  no usable AoN page (85 are rule entries) and a PRD "no", 43 match partly,
  42 match fuzzily (most of the note on one page, but not the whole note),
  and 3 are paraphrases.
- **Zero confirmed AoN or PRD matches for Pathfinder Data commentary.** 73
  commentary blocks are confirmed absent; the other 15 are still candidates
  (14 with no usable AoN page, 1 partial). The only editorial blocks that
  pass are page citations that AoN prints as an item's source line (323).
- As supporting evidence, each block is also classified against the owner's
  PDFs: **688 printed, 575 editorial, 21 unclear**. The editorial blocks are
  438 page citations, 88 commentary notes, 43 pointers and 6 compiled
  additions.
- **140 blocks are orphaned** (no marker in the body). 83 of them stay under
  the rule and need their marker restored; 33 go with their notes; 24 are
  candidates. The main cause is the importer, which rewrote `#D`/`#M` spell
  markers as text and still emitted the note.
- 108 paragraphs outside footnote blocks carry an editorial signal. A hand
  review found **76 editorial paragraphs across 68 entries** (6 of those
  entries also have footnote blocks). 61 of them (18 inline notes) are
  confirmed not on AoN or in the PRD; 15 (3 inline notes) are candidates.
- Licensing: the dataset's notices support two readings for its editor's
  notes (its own Open Game Content, or GPL-3.0-or-later). Removing the notes
  that AoN and the PRD do not carry removes the question for them.

## Files

| File | Contents |
|---|---|
| [`editorial-footnotes.tsv`](editorial-footnotes.tsv) | One row per footnote block (1,284 rows), with the classification, the AoN/PRD verdict, the action, and the evidence. |
| [`editorial-notes-outside-footnotes.tsv`](editorial-notes-outside-footnotes.tsv) | One row per paragraph outside a footnote block that carries an editorial signal or starts with a note marker (108 rows), with a hand review and the AoN/PRD verdict. |
| [`scripts/editorial-footnotes-nested.py`](../../scripts/editorial-footnotes-nested.py) | Inventory, text signals, orphan causes, and the merge of the checks below. Writes both TSV files. |
| [`scripts/footnote-print-check-nested.py`](../../scripts/footnote-print-check-nested.py) | Automatic check of each block against the text of the owner's PDFs. |
| [`scripts/footnote-aon-check-nested.py`](../../scripts/footnote-aon-check-nested.py) | Check of each block and outside paragraph against AoN pages and PRD documents (the corpus's `ref/carried/` entries and legacy PRD pages), one page at a time, naming the page that matched. |

The scripts are research scripts; the build does not run them. Their names end
in `-nested.py` so that the docs row of the AGENTS.md test scope covers them.

### TSV columns

`editorial-footnotes.tsv`:

| Column | Meaning |
|---|---|
| `entry_id`, `seq` | The entry and the block's position in it. Seven entries repeat an anchor, so `seq` is part of the key. |
| `category`, `kind` | The `entries/` directory and the entry kind. |
| `anchor`, `marker` | The `{#footnote-…}` anchor and the marker label. |
| `marker_in_body` | `linked` (`^[1](id#footnote-1)^`), `plain` (`^1^`), or `absent`. |
| `orphan_cause` | For an absent marker: `marker-expanded-inline`, `shared-record-copy`, or `marker-dropped` (see [Orphaned markers](#orphaned-markers)). |
| `raw_residue` | `yes` when the note keeps unparsed `\|X~` source syntax. |
| `datasets`, `records` | Provenance from the front matter. |
| `source`, `source_page` | The entry's first source. |
| `signals` | Editorial text signals, and `later:<title>` for a cited book newer than the source. |
| `auto_print` | Result of the automatic print check (verdict, note and control scores, books searched). |
| `review` | A hand verdict, when one was made. |
| `classification`, `subclass` | `printed`, `editorial` or `unclear`, and the subclass (see [Classification](#classification)). |
| `evidence` | Why: a book and page, a print-check result, or the signals. |
| `aon_check` | AoN verdict (`yes`, `yes-other-page`, `fuzzy`, `partial`, `no`, `unchecked`), the note and control scores on the entry's page, the entry's page (`page=`), and the other page that holds most of the note (`match=`, with its score). |
| `prd_check` | PRD verdict (`yes`, `fuzzy`, `partial`, `no`, `unchecked`), the best single-document score, the score pooled over all PRD text, and the best document (`doc=`: a legacy PRD URL or a `ref/carried/` entry id). |
| `aon_evidence` | A hand AoN/PRD verdict, when one was made. |
| `on_aon_prd` | `yes`, `no`, `candidate`, or `unchecked` (see [AoN and PRD presence](#aon-and-prd-presence)). |
| `presence_basis` | Why: `aon-page`, `aon-other-page`, `prd-document`, `aon-no+prd-no`, `fuzzy`, `partial`, `aon-no+prd-unchecked`, `aon-unresolved+prd-no`, `aon-unresolved+prd-unchecked`, `hand`, `hand: paraphrase`, or `conflict: …` (a hand verdict made on other check results; none on this pin). |
| `carried_by` | For `yes`: the AoN page or PRD document that carries the note. For a candidate: the one that holds most of it, when there is one. |
| `action` | Owner rule: `keep` (yes), `remove` (no), `review` (candidate), `check` (unchecked). |
| `text_sha` | The first 16 hex digits of the SHA-256 of the note's full text. A rerun reuses a row's reviews only when this matches. |
| `text` | The first 160 characters of the note, to identify it. |

## Where the notes come from

Every one of the 1,284 blocks comes from the Pathfinder Data 1e dataset
(`jasontankapps/pathfinder-data-1-e` at commit `08ea71f2…`). None comes from
PSRD-Data.

Pathfinder Data stores each record's text as a Markdown array in
`description`. It has no separate field for notes. Notes come in three source
forms, which its `JSON.md` documents:

- Markdown footnotes: a `[^1]` reference and a `[^1]: …` definition line.
- `::mfn[X~text]` stat-block notes, such as `B~Bonus feat.`.
- `@FN[X]` pointers with `@FN[text]{from=X}` bodies inside a `:::fakeFootnotes`
  container.

The importer, at the git tag `psrd-last-buildable`
(`crates/content-compiler/src/pf_markdown.rs`), turned the first two forms into
the level-six `{#footnote-…}` headings that the corpus keeps today. It turned
`@FN` bodies into inline paragraphs that start with the marker
(`^1^ The original text …`). It had no flag for editorial text: a printed table
note and an editor's remark went through the same code. The coverage rows for
`::fakeFootnotes` in `exclusions.toml` say "no footnote text is lost", which is
true and does not tell the two kinds apart.

The Summon Monster notes show the path. The base record `summon_monster` in
`json/spells16.json` holds both definitions; its `[^1]` and `[^2]` references
sit in the Summon Monster I paragraph about templates. The records
`summon_monster_2` to `summon_monster_9` are `copyof` records. The importer's
`append_referenced_footnotes` (`canonical.rs`) copied the definitions into each
level's section, but the paragraph with the references stayed in Summon
Monster I. That is why spells II to IX show the notes with no marker. The
issue's guess that the markers sat in the summon tables is not right.

The editor who wrote the notes is the Pathfinder Data author, not AoN. The
notes speak of "this app", "*Pf Data 1e* has no connection to the authors",
and once advertise another app by the same developer
(`ref/rule/tavern-toolbox`). AoN has its own editorial notes, marked "Nethys
Note". The two sometimes cover the same point in different words: AoN's
[Summon Monster II](https://aonprd.com/SpellDisplay.aspx?ItemName=Summon%20Monster%202)
page has a Nethys Note that cites *Monster Summoner's Handbook* p. 18 on
entropic and resolute templates, and the Pathfinder Data footnote 1 cites the
same rule but calls AoN "Paizo's official open-content website". The words do
not match. Some deity notes state the same rule as an AoN bracket in other
words, because both condense the same printed sentence: the deity "unique
spell rules" (for example AoN's
[Shelyn](https://aonprd.com/DeityDisplay.aspx?ItemName=Shelyn),
[Urgathoa](https://aonprd.com/DeityDisplay.aspx?ItemName=Urgathoa),
[Abadar](https://aonprd.com/DeityDisplay.aspx?ItemName=Abadar) and
[Besmara](https://aonprd.com/DeityDisplay.aspx?ItemName=Besmara) pages). A
paraphrase is not the note's text, so the audit marks those notes as
candidates for the owner, not as `yes`. The
[Robe of Bones](https://aonprd.com/MagicWondrousDisplay.aspx?FinalName=Robe%20of%20Bones)
and [Permanency](https://aonprd.com/SpellDisplay.aspx?ItemName=Permanency)
pages carry none of the Pathfinder Data notes on those entries.

## Classification

Each block is `printed`, `editorial` or `unclear`, with a subclass:

| Class | Subclass | Blocks | Entries | What it is |
|---|---|---:|---:|---|
| printed | `print-match` | 527 | 361 | The automatic print check found the note in a source book (score ≥ 0.6). Includes close paraphrase. |
| printed | `paraphrase` | 87 | 74 | A hand check found the rule printed in other words (for example the deity variant spells, *Inner Sea Gods* p. 19 and p. 176). |
| printed | `legend` | 74 | 73 | The note restates a printed marker key: "Bonus feat." for a superscript B (*Bestiary* p. 6), "Domain spell". |
| editorial | `source-citation` | 438 | 70 | A page citation the dataset added, such as "Aquatic Adventures pg. 62". 322 of them are on 23 *Figurine of Wondrous Power* entries, 14 each. |
| editorial | `commentary` | 88 | 71 | The editor's remark: rules reading, GM advice, a critique of the source, a cross-reference to a later book. |
| editorial | `pointer` | 43 | 12 | A pointer the dataset wrote in place of a printed table or page reference, such as "Refer to Type D, potions and oils" (*Ultimate Equipment* pp. 365–368 print "Table 7–14, page 370"). |
| editorial | `compiled` | 6 | 4 | A dataset addition that joins material from later books, such as classes added to the *Core Rulebook* age table (p. 169 prints only the core classes). |
| unclear | | 21 | 14 | No check settles it: the book is not in the PDF set, or its text extraction is too poor. |

Totals: **688 printed, 575 editorial, 21 unclear**. The commentary set is 88
blocks in 71 entries, including all nine *Summon Monster* spells (18 blocks).
Entries with footnote blocks, by category, match the issue: monster 278, rule
91, magic-wondrous 73, faith 68, npc 62, spell 28, eq-spellbook 17, eq-weapon
12, feat 10, template 4.

### Method

1. **Inventory.** `editorial-footnotes-nested.py` parses each entry's front
   matter and body, finds each `######` heading with a `footnote-` anchor, and
   takes the one paragraph after it as the note, as
   [docs/corpus-format.md](../corpus-format.md) defines a footnote. A later
   paragraph is body text, and the outside scan reads it. On this pin no
   footnote heading is followed by a second paragraph before the next
   heading.
2. **Text signals.** Phrases such as "Consult your GM", "rules-as-written",
   "the original source", "this app", "no such feat exists", "no errata", a
   website or Paizo in the third person, and a book title newer than the
   entry's source mark a note as commentary. A note that is only
   "Title pg. N" is a source citation.
3. **Automatic print check.** `footnote-print-check-nested.py` extracts the
   text of each source book in the owner's PDF set and measures the share of
   the note's four-word shingles in it. It measures the entry's own body text
   the same way, as a control: a control of 0.5 or more proves that the right
   book was searched. Verdicts: printed (note ≥ 0.6), absent (note ≤ 0.2 with
   control ≥ 0.5), partial, inconclusive (control < 0.5), no PDF. Results:
   536 printed, 301 absent, 346 inconclusive (322 of them figurine citations,
   whose item tables lower the control), 86 no PDF, 15 partial.
4. **Hand review.** Every block that had no editorial signal but was absent,
   partial, inconclusive or had no PDF got a hand verdict: 76 distinct note
   texts, 175 blocks. The review read the page text around the passage and,
   for deity notes, the AoN page. Each verdict cites a book and page.

Precedence: hand review, then the automatic print check, then the text
signals.

### Error rate

- **Automatic print matches.** A stratified sample of 33 print matches (5 per
  major category) was read by eye in the page text. All 33 are printed
  content. Two are close paraphrases of a printed sentence (*Inner Sea Gods*
  p. 19 and p. 35). The 95 % upper bound on the error rate is about 9 % (three
  in 33).
- **Commentary.** All 88 blocks were read. None of them matched print: 69
  were absent with a good control; 15 have no PDF; 4 were inconclusive or
  partial and are commentary by their wording ("No such feat exists"). One
  block that had only a weak signal ("appears to be", *Ydersius*) moved to
  unclear.
- **Signals alone are not enough.** The word "suggestion" first flagged six
  printed *crystal ball* notes; the print check corrected that. The issue's
  phrase search (63 files) also hits printed text: the *Advanced Player's
  Guide* race sections say "Consult your GM before selecting any of these new
  options".
- **Legends** are classified by rule, not checked at each place. The keys are
  printed in each book's introduction.

## AoN and PRD presence

Owner rule test: does the note's text appear on AoN or in the Paizo PRD?
`footnote-aon-check-nested.py` measures it for each block and each outside
paragraph, and `editorial-footnotes-nested.py` combines the two checks.

**One page at a time, the whole note.** Every score is the share of the
note's four-word shingles found on ONE AoN page or ONE PRD document, and the
TSV names that page or document. A score pooled over many pages is reported
(`pooled=`) but never makes a `yes`: it does not show that any page carries
the note. A page *carries* the note only when the whole note appears on it as
written: after lower-casing and normalising punctuation and spacing, the
note's words form one unbroken run of the page's words. A shingle share does
not prove that: a page that omits the note's opening words still holds 90 %
of its shingles. A page with 60 % or more of the note but not the whole note
is a fuzzy match; quotations, paraphrases and shortened notes land there.
Every `yes` row decided by a check has a score of 1.00 on the page or document
it names. Notes under four words are matched whole, in their printed key form
("D Domain spell"), because "bonus feat" also occurs in ordinary text.

Earlier versions of this audit were looser. The first accepted 60 % pooled
over all pages, which counted, for example, a Wild Stalker note as present
because it quotes a printed sentence. The second accepted 90 % on one page.
The whole-note test moved six blocks from `yes` to candidate: three deity
notes whose AoN bracket omits the note's opening words (Shelyn block 2:
"This spell may be prepared …" against the bracket "may be prepared by any
priest …"; Besmara block 1; Ydersius block 3), and three PRD table notes
whose page text either is broken up by table cells or lacks words of the
note (Breaking Things block 2, Miscellaneous Actions block 5, Terrain
Improvements block 1, which cites a "Table 3-3" that the page does not
print). Two outside paragraphs of book prose (Harrower line 13, How to Play
line 22) moved the same way; their action stays "not a note".

**AoN.** For each of the 754 entries (the 664 with footnote blocks and those
with an outside paragraph) the script read one AoN page and scored the
entry's body text on it as a control (0.3 or more proves it is the entry's
page). The page is the entry's `aon` path (251 entries), else a URL derived
from the kind and title, else an AoN search link whose name equals the title.
Rule pages come from the AoN rule navigation in `aon-rule-sections.json`. A
derived URL is evidence for this audit only; the corpus stores no guessed
path. A note that is not on its entry's page is then matched against each of
the 794 AoN display pages that the run read. The live run made 1,154
requests two seconds apart, and AoN never blocked it. The committed TSVs come
from a rerun on that page cache with `--offline` (no requests). A usable page
was found for 528 of the 664 entries with footnote blocks.

**PRD.** Each `ref/carried/` entry of the corpus (1,984, the PSRD-Data parse
of the PRD) and each crawled page of the legacy PRD at
https://legacy.aonprd.com/ is one PRD document. The crawl starts at the
legacy front page and follows relative links inside ten book folders
(`coreRulebook`, `gameMasteryGuide`, `ultimateEquipment`, `ultimateCampaign`,
`ultimateCombat`, `ultimateMagic`, `advancedPlayersGuide`,
`occultAdventures`, `unchained`, `bestiary`); it reached 351 pages. (The
earlier version said 507: that counted cache files left from trial crawls,
which the old script also added.) Notes under four words are not checked
against the PRD.

**Limits of a negative.** An AoN "no" means: not on the entry's page, and no
more than 60 % of the note on any of the 794 pages this run read. AoN has
many more pages. A PRD "no" means: no single document holds more than 20 % of
the note and the pooled share is under 60 %, over the carried entries and the
351 crawled pages only. The crawl skips spell and monster folders (their
pages carry no rule-table notes), links that leave a book folder, and every
PRD book outside the ten folders. A note in a skipped folder, or on an AoN
page the run did not read, shows as absent. This is why a combined `no`
needs both checks, and why a row where only one check ran is a candidate.

**Verdicts.**

| `on_aon_prd` | When |
|---|---|
| `yes` | The entry's AoN page carries the whole note, another single AoN page carries it, or a single PRD document carries it. `carried_by` names the page or document. One row is `yes` by hand (Urgathoa "Arcane version.", see below). |
| `no` | The entry's AoN page (control ≥ 0.3) has 0.2 or less of the note, no AoN page read has 0.6 or more, AND the PRD check ran and no document has more than 0.2 (pooled under 0.6). Two rows are `no` by hand: the Groetus note, and the Wild Stalker outside paragraph at line 90, whose raw checks are `partial` because it quotes a printed sentence (see below). |
| `candidate` | Anything between: a fuzzy match (0.6 or more on one page, not the whole note), a partial match, a paraphrase, an AoN "no" with the PRD unchecked, no AoN page with a PRD "no", or a hand verdict that conflicts with a newer check. |
| `unchecked` | No AoN page and a note under four words. |

**Hand verdicts.** Six rows got a hand verdict, recorded in `aon_evidence`
with the page. A hand verdict holds only for the text and the checks it was
made on: see [Reproduce](#reproduce).

- Urgathoa's note "Arcane version." is printed word for word on the AoN
  [Urgathoa](https://aonprd.com/DeityDisplay.aspx?ItemName=Urgathoa) page as
  "[arcane version]" at the same spell lines: `yes`.
- Three deity notes say the same rule as an AoN bracket in other words:
  Urgathoa's "Only necromancers gain this benefit." ("[necromancer only,
  arcane version]"), Geryon's "This spell affects only reptiles." ("[affects
  only reptiles]") and Gozreh's aquatic-variant note. A paraphrase is not the
  note's text, so these are `candidate` (`hand: paraphrase`) for the owner.
- The Groetus note is not on its AoN page and in no PRD document: `no`.
- The Wild Stalker outside paragraph at body line 90 quotes a printed
  sentence and then corrects the progression. The AoN
  [Wild Stalker](https://aonprd.com/ArchetypeDisplay.aspx?FixedName=Ranger%20Wild%20Stalker)
  page and the legacy PRD
  [ranger archetypes](https://legacy.aonprd.com/ultimateCombat/classArchetypes/ranger.html)
  page carry only the quoted sentence (0.24 of the paragraph). The editor's
  remark is on neither: `no`.

The earlier version gave 34 "Bonus feat." notes a "no" by pattern from other
monster pages. This version drops that extrapolation: those notes have no
usable AoN page, so they are `unchecked`.

**Results.**

| `on_aon_prd` | Blocks | Entries | How |
|---|---:|---:|---|
| yes | 716 | 335 | 534 on the entry's AoN page, 41 on another AoN page, 140 in a PRD document, 1 by hand |
| no | 138 | 105 | 137 by both checks, 1 by hand |
| candidate | 391 | 302 | 172 AoN no with PRD unchecked, 131 no AoN page with PRD no, 43 partial, 42 fuzzy, 3 paraphrases by hand |
| unchecked | 39 | 12 | No AoN page and a note under four words |

By subclass:

| Subclass | yes | no | candidate | unchecked |
|---|---:|---:|---:|---:|
| `print-match` | 310 | 1 | 181 | 35 |
| `paraphrase` | 74 | 1 | 12 | 0 |
| `legend` | 7 | 1 | 63 | 3 |
| `source-citation` | 323 | 55 | 60 | 0 |
| `commentary` | 0 | 73 | 15 | 0 |
| `pointer` | 0 | 3 | 39 | 1 |
| `compiled` | 0 | 1 | 5 | 0 |
| unclear | 2 | 3 | 16 | 0 |

**"Bonus feat." notes.** 205 blocks in 179 entries are a standalone "Bonus
feat." note: 143 classified `print-match` and 62 `legend`. (The print check
matches a short note whole, and the words "bonus feat" occur throughout any
book, so `print-match` is weak evidence for these; the key is printed once
per book, for example *Bestiary* p. 6.) AoN shows the superscript B at the
feat and never prints the words. Of the 205, 171 have a resolved AoN page
without the words and no PRD check (under four words): `candidate`. 34 have
no usable AoN page: `unchecked`. Two other blocks mention a bonus feat. The
Morrigna block (`ref/monster/psychopomp-morrigna`) holds two notes that the
importer did not split: "Bonus feat.|C~No spell equivalent was provided.".
The second half is the editor's remark, so the block is classified
`commentary`, and as a whole it is on no AoN page and in no PRD document:
`no`. The earlier version counted it with the B legends in the summary (206)
and with commentary in the subclass table; this version counts it once, with
commentary. Removing it must keep the printed `^B^` marker (ticket 1). The
`ref/rule/stat-block` note "Example of a bonus feat's footnote." is
commentary and a candidate.

**Other printed notes that are confirmed absent:** the Qinggong Monk note on
an empty ki pool, the Groetus note, and the Kortash Khain block (an unsplit
legend with `\|X~` residue). The 3 unclear "no" blocks are the Apsu note and
the two gidim "Dread spell" notes.

**The 39 unchecked blocks** are the 34 "Bonus feat." notes above and 5 short
notes on rule entries. **The 131 candidates without an AoN page** are mostly
rule entries (85; their AoN rule pages have numeric ids that the navigation
file does not list) and *Figurine of Wondrous Power* and greater talisman
entries (34). 14 of them are commentary by wording.

## Orphaned markers

140 blocks have no marker in the entry body:

| Cause | Blocks | What happened |
|---|---:|---|
| `marker-expanded-inline` | 79 | The importer's `spell_entry` rewrote the `#D`, `#M` and `#B` spell markers as the text " (domain spell)", " (mythic spell)", " (bonus spell)" and still emitted the `::mfn` note as a heading. The note now repeats the inline text with nothing pointing at it. |
| `shared-record-copy` | 43 | The note was copied from a shared record whose marker stayed in a sibling entry: *Summon Monster II–IX* (16, two per spell), the four *detect* spells that copy *detect evil* (8), *Contact Entity I, II, IV* (3), *minor creation* and Type G (2), and 14 monsters with one each. |
| `marker-dropped` | 18 | The source marker was lost. Most are monster stat-block notes (C, S, X, F, N). |

Under the owner rule the orphans split 83 `yes` (75 inline expansions, 6
shared-record copies, 2 dropped markers), 33 `no` (all 16 *Summon Monster
II–IX* copies among them) and 24 candidates.

The inline expansion also changes the meaning in one group. For the
archdevils, demon lords, empyreal lords, horsemen and qlippoth lords, the
printed asterisk means "can use the mythic version of this ability in his
realm" (*Bestiary 4* p. 48). The corpus shows "(mythic spell)" after each such
spell, which reads as an unconditional mythic spell.

The reverse case, a marker with no block, has 764 plain superscripts in 69
entries. 684 are in 35 PSRD carry-over entries (`ref/carried/`), where the book
defines the marker in prose (for example the spellbook S and OP marks) or the
note is a normal paragraph. 30 are the leading markers of the inline `@FN`
notes. 50 are in 33 Pathfinder Data entries. A linked marker
(`^[1](id#footnote-1)^`) always has its block: `corpus-check` fails on a link to
a missing anchor.

Three blocks keep unparsed `\|X~` source syntax (`old-mage-jatembe`,
`kortash-khain`, `psychopomp-morrigna`): the importer did not split a
multi-note `::mfn`.

## Editorial notes outside footnote blocks

The issue's phrase search found commentary in entries with no footnote block.
The outside scan lists every paragraph outside a block that has a strong
signal ("this app", "rules-as-written", a website, "the original text", "no
errata", "consult your GM") or starts with a note marker (the inline `@FN`
form). It found 108 paragraphs in 97 entries. A hand review found **76
editorial paragraphs across 68 entries** (6 of those entries also have
footnote blocks), 29 printed (book prose that says "consult your GM" or names
paizo.com), and 3 unclear. The editorial ones include 21 inline notes (19 of
them on archetypes, such as "This archetype was published before the void
element was created"), "*Note:* The original text also contains …" on
spellbooks, the "These specific breeds do not have published prices" lines
on mounts, and the app plug in `ref/rule/tavern-toolbox`.

The AoN and PRD checks score the full paragraph, read from the corpus at its
line; the TSV keeps only a 160-character excerpt. Under the owner rule the 76
editorial paragraphs split **61 no** (60 by both checks, plus the Wild
Stalker paragraph at line 90 by hand; 18 of the 61 are inline notes) and **15
candidates** (13 with no usable AoN page and a PRD "no", 2 partial; 3 of the
15 are inline notes). The 3 unclear paragraphs are candidates. None of the 79
is confirmed on AoN or in the PRD. The 29 printed paragraphs are book prose,
not notes (`action` "not a note").

**The Tavern Toolbox plug is a candidate, not a removal.** The paragraph
("The developer of this app also created Writing Quirks …") is in no PRD
document (best 0.00, pooled 0.00). Its AoN page is not resolved: AoN search
for "Tavern Toolbox" lists no rule page of that name, and the nearest result,
[Taverns](https://aonprd.com/Rules.aspx?ID=827), is not the entry's page
(control 0.00). A check of the AoN page for *Game Mastery Guide* p. 200
settles it (ticket 3).

## Licensing

Each entry carries the license basis `ogl-1.0a` plus `legal#community-use`.
`legal.toml` describes the dataset as "Open Game Content from the pinned
Pathfinder Data 1e corpus, used under the Open Game License 1.0a and Paizo's
Community Use Policy".

The dataset's own notices point two ways:

- Its `README.md` says the app uses Open Game Content "encoded in
  human-readable format in the json directory", and its `OPENGAMECONTENT.md`
  Section 15 list includes "Pf Data 1e Copyright 2025, Jason Tamez".
  `legal.toml` and `LICENSES/OGL-1.0a.md` in the corpus carry that line. On
  this reading the editor contributed the notes as Open Game Content of
  *Pf Data 1e*, and the corpus already gives the required notice.
- The same README says "The non-OGC content of *Pf Data 1e* is Copyright 2024
  by Jason Tamez" and licenses it under GPL-3.0-or-later. OGL 1.0a Section
  1(d) defines Open Game Content as game mechanics plus "additional content
  clearly identified as Open Game Content by the Contributor", and Section 8
  asks for that identification. A directory-wide statement is a weak
  identification for prose that advises the reader, cites forum posts, or
  advertises another app. On this reading the notes are GPL text, and the
  corpus `LICENSE.md` statement that none of the dataset's application content
  "was copied" is not accurate for them.

The Community Use Policy does not help on either reading: it covers Paizo's
material, not a third party's prose.

Risk by subclass:

- `source-citation` and `pointer`: page numbers and short functional phrases.
  They carry little or no protectable expression.
- `compiled`: lists of class names. Low.
- `commentary`: original prose by a third party. The basis is ambiguous.

AoN's own Nethys Notes are a separate question. AoN hosts Paizo's rules
under a license from Paizo, but its editors' notes are not Paizo's printed text
either. The
owner's rule keeps a note that AoN carries; the corpus would then hold
Pathfinder Data's copy of a note whose wording happens to match AoN's (most
matches are condensed printed sentences, so the risk is low). A note's text
from the PRD is Paizo's Open Game Content.

## Options

1. **Remove every editorial note; record each in `exclusions.toml`.** Printed
   text wins and the licensing question disappears. No schema change. Loses
   the useful AoN-carried notes and the deity rule summaries that AoN also
   shows. Superseded by the owner's rule.
2. **Keep the notes as marked commentary.** Needs a typed node (a note block
   with an origin), a `SCHEMA_VERSION` increase, a full CI run, a corpus format
   rule, and a licensing decision for third-party prose. It leaves the GPL
   question open.
3. **Keep printed footnotes and fix orphaned markers.** Needed under any
   option.
4. **Per-kind handling.** Convert source citations to structured `[[sources]]`
   and `[[expanded_in]]` rows, rewrite pointers as links, drop compiled
   additions, decide commentary on its own.
5. **Owner rule: keep what AoN or the PRD carries, remove the rest, show the
   kept ones de-emphasized.** One test that a reviewer can repeat. It keeps the
   printed notes that AoN or the PRD carries, removes the Pathfinder Data
   commentary that nobody else carries (which also removes the licensing
   question for it), and needs no schema change: the de-emphasis is a
   rendering rule for footnote headings, which the corpus already marks with
   the `footnote-` anchor ([#509](https://github.com/phynics/cast-iron/pull/509)).

## Owner decisions

Two groups need an owner decision before the corpus tickets touch them.

**(a) "Bonus feat." legends.** 205 standalone blocks in 179 entries: 171
candidates (the AoN page shows the superscript B but not the words, and the
PRD check cannot test a two-word note) and 34 unchecked (no AoN page). The
Morrigna block is separate: its editor's half makes it a confirmed `no`.

- Option 1: keep them as de-emphasized notes. They are the book's own key
  for the B superscript (*Bestiary* p. 6), even where the AoN page does not
  print the words at the stat block. The orchestrator recommends this
  option.
- Option 2: apply the rule strictly. AoN and the PRD do not print the words
  at the stat block, so the 205 notes go with `exclusions.toml` rows, and the
  printed `^B^` superscripts stay as plain markers. The B key could then be
  explained once, for example by a link to `ref/rule/stat-block`.

**(b) Page-citation notes.** 438 `source-citation` blocks in 70 entries: 323
are verified (`yes`: AoN prints the same page as the item's source line, and
`carried_by` names the page), 55 are confirmed `no`, and 60 are candidates
(34 with no AoN page, 22 partial, 4 fuzzy).

- Option 1: convert them to structured `[[sources]]` and `[[expanded_in]]`
  rows and remove the citation blocks. The information then appears in the
  form the app already shows. Only the 323 verified citations can become
  source rows directly; the 55 negatives and the 60 candidates need their
  page checked in the book first. The orchestrator recommends this option.
- Option 2: treat them like any other note under the rule. Keep the 323 as
  de-emphasized notes, remove the 55, and review the 60.

## Recommendation

Apply the owner rule as one corpus pass per follow-up ticket below:

1. **Remove the 138 confirmed "no" blocks** (105 entries) and, in a separate
   pass, **the 61 confirmed "no" editorial outside paragraphs**. Record each
   removal in `exclusions.toml`, one `[[excluded]]` row per source record
   (`path = "json/<file>.json#<record>"`) with a reason that names #109, the
   subclass, and the count, for example "2 footnotes not on AoN or the PRD
   (#109): commentary". Remove each removed note's marker reference with it
   (for example the two linked markers in Summon Monster I). For the Morrigna
   block, keep the printed `^B^` superscript as a plain marker
   (docs/corpus-format.md allows an unlinked superscript) and remove only the
   note.
2. **Keep the 716 "yes" blocks.** The app already shows them de-emphasized
   ([#509](https://github.com/phynics/cast-iron/pull/509)).
3. **Review the 391 candidate and 39 unchecked blocks and the 18 candidate
   outside paragraphs** before removing anything. The owner decisions above
   settle 205 of the blocks and the citation group.
4. **Fix the orphaned markers of the notes that stay** (83 blocks). The 75
   inline expansions: restore the `^D^`/`^M^` superscript in place of the
   "(domain spell)" or "(mythic spell)" text, as AoN and print show it. In
   the realm-only mythic group (archdevils and the other lords) this also
   corrects the meaning. Restore or drop the markers of the 6 shared-record
   copies and 2 dropped markers that stay.

## Follow-up tickets

Corpus tickets go to phynics/cast-iron-data; the lint ticket goes to
phynics/cast-iron. Each one rereads this audit's TSV columns, and each corpus
ticket ends with `corpus-check` passing and a pin bump PR.

1. **Remove footnotes that are confirmed absent from AoN and the PRD**
   (cast-iron-data). Scope: the 138 blocks with `on_aon_prd = no` (`action =
   remove`) in `editorial-footnotes.tsv`; no candidate or unchecked row.
   Acceptance: rerunning `scripts/editorial-footnotes-nested.py` on the new
   pin lists none of those blocks. Each removed block has an `exclusions.toml`
   row naming its record, #109, and the reason. No marker reference to a
   removed anchor remains (`corpus-check` enforces the linked ones; the plain
   `^n^` ones are checked by the script). Printed `^B^` superscripts stay.
2. **Remove editorial paragraphs outside footnote blocks that are confirmed
   absent from AoN and the PRD** (cast-iron-data). Scope: the 61 rows with
   `action = remove` in `editorial-notes-outside-footnotes.tsv`, 18 of them
   inline notes. No candidate row is in scope: the 3 inline notes and the
   Tavern Toolbox plug that are still candidates wait for ticket 3. Acceptance: the outside scan on the new pin lists none of
   them; each removal has its own `exclusions.toml` row.
3. **Review the candidates and unchecked rows** (cast-iron-data, research).
   Scope: the 391 candidate and 39 unchecked blocks, and the 18 candidate
   editorial and unclear outside paragraphs (Tavern Toolbox and 3 inline
   notes among them).
   Apply the owner decisions on "Bonus feat." legends and paraphrases first.
   Acceptance: each row has `on_aon_prd` yes or no with evidence (the AoN or
   legacy PRD URL that carries the note, or the resolved pages that do not),
   merged with `--aon-review`. Confirmed "no" blocks join ticket 1's scope;
   confirmed "no" outside paragraphs join ticket 2's scope.
4. **Restore orphaned markers of kept footnotes** (cast-iron-data). Scope:
   the 83 blocks with `orphan_cause` set and `on_aon_prd = yes`. Acceptance:
   the 75 `marker-expanded-inline` blocks have their superscript back and no
   "(domain spell)" or "(mythic spell)" text remains in those spell lists. The
   realm-only mythic abilities no longer read as plain mythic spells. The
   script reports no absent marker among kept blocks.
5. **Clean converter residue** (cast-iron-data). Scope: the 3 blocks with
   `raw_residue = yes` and the 7 entries that repeat an anchor. Acceptance: no
   `\|X~` text remains; every anchor in an entry is unique. Inbound links,
   if any, are updated in the same PR.
6. **Correct the corpus license note** (cast-iron-data). `LICENSE.md` says
   that none of the dataset's GPL application content was copied, which is
   not true while the editor's notes are in the corpus. Acceptance: after
   tickets 1 and 2, `LICENSE.md` names the dataset notes that stay (the ones
   AoN or the PRD carries) and their license basis.
7. **Option: convert source-citation footnotes to structured sources**
   (cast-iron-data, owner decision (b)). Scope: the 438 `source-citation`
   blocks. The 323 verified citations (`on_aon_prd = yes`, page in
   `carried_by`) convert directly. The 55 negative and 60 candidate
   citations are checked in the book before any becomes source metadata.
   Acceptance: each converted publication and page is a `[[sources]]` or
   `[[expanded_in]]` row on the entry; no citation block remains.
8. **Lint footnotes in `corpus-check`** (cast-iron). Acceptance:
   `corpus-check` warns on a `footnote-` heading whose marker does not occur
   in the entry body and on `\|X~` residue, with a test for each; the corpus
   format doc names the rule.

## Reproduce

From the repository root, with the pinned corpus in `.cache/sources/cast-iron-data`
(`cargo xtask fetch-sources`):

```sh
# Regenerate both TSV files from themselves (no network, no PDFs):
python3 scripts/editorial-footnotes-nested.py .cache/sources/cast-iron-data \
    --reuse docs/research/editorial-footnotes.tsv \
    --outside-review docs/research/editorial-notes-outside-footnotes.tsv \
    --tsv docs/research/editorial-footnotes.tsv \
    --outside-tsv docs/research/editorial-notes-outside-footnotes.tsv
```

The output is byte-identical on the same pin.

**Reuse at a new pin.** Every row carries `text_sha`, a hash of the note's or
paragraph's full text. The script reuses an earlier row's print check, hand
review and hand AoN/PRD verdict only when the current text has the same hash;
a row whose text changed starts unreviewed. A reused hand AoN/PRD verdict
also needs this run's AoN and PRD check verdicts to equal the ones in the
row's `aon_check` and `prd_check` (what the reviewer saw). When they differ,
the row becomes a `candidate` with the basis `conflict: …`, keeps the old
evidence, and needs a new hand verdict through `--aon-review`. The script
prints how many rows it did not reuse and how many conflict.

To rerun the checks (at a new pin, or to reproduce the method), keep both
cache directories out of the repository and run, in this order:

```sh
CORPUS=.cache/sources/cast-iron-data
# 1. Print check (needs pdftotext and the owner's PDFs):
python3 scripts/footnote-print-check-nested.py $CORPUS PDF_DIR PDF_CACHE --out print.tsv
# 2. Inventory and outside scan (writes the outside TSV that step 3 reads):
python3 scripts/editorial-footnotes-nested.py $CORPUS --auto print.tsv \
    --reuse docs/research/editorial-footnotes.tsv \
    --outside-review docs/research/editorial-notes-outside-footnotes.tsv \
    --tsv docs/research/editorial-footnotes.tsv \
    --outside-tsv docs/research/editorial-notes-outside-footnotes.tsv
# 3. AoN and PRD check (about one AoN request per entry, two seconds apart;
#    stops on HTTP 403 or 429; add --offline to use only AON_CACHE):
python3 scripts/footnote-aon-check-nested.py $CORPUS AON_CACHE --out aon.tsv \
    --legacy-books coreRulebook gameMasteryGuide ultimateEquipment \
        ultimateCampaign ultimateCombat ultimateMagic advancedPlayersGuide \
        occultAdventures unchained bestiary
# 4. Merge. A hand verdict in the TSVs stays only when its row's text hash and
#    its AoN/PRD check verdicts are unchanged; settle conflicts with --aon-review:
python3 scripts/editorial-footnotes-nested.py $CORPUS --aon aon.tsv \
    --reuse docs/research/editorial-footnotes.tsv \
    --outside-review docs/research/editorial-notes-outside-footnotes.tsv \
    --tsv docs/research/editorial-footnotes.tsv \
    --outside-tsv docs/research/editorial-notes-outside-footnotes.tsv
```

Steps 3 and 4, run with `--offline` on the page cache of the live AoN run,
reproduce the committed TSVs byte for byte.
