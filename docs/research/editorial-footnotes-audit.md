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
reader can tell it is not rules text; a separate design ticket in
phynics/cast-iron covers that. The `on_aon_prd` column applies this rule. The
printed/editorial classification stays as supporting evidence.

## Summary

- The corpus has **1,284 footnote blocks in 664 entries**. All of them came
  from the Pathfinder Data 1e dataset; none came from PSRD-Data.
- **On AoN or the PRD (owner rule): 763 yes, 345 no, 176 unchecked.** Under
  the rule, 763 blocks stay (shown de-emphasized), 345 blocks in 286 entries
  go, and 176 blocks in 91 entries need one more check.
- The 345 "no" blocks are 206 "Bonus feat." notes (AoN prints the superscript
  B, never the words), 72 other Pathfinder Data commentary notes (all nine
  *Summon Monster* spells among them), 55 page citations, 5 pointers, 1
  compiled addition, 3 printed notes that AoN does not carry, and 3 unclear.
- **No Pathfinder Data commentary note is on AoN or in the PRD.** The only
  editorial blocks that pass are page citations that AoN prints as an item's
  source line.
- As supporting evidence, each block is also classified against the owner's
  PDFs: **688 printed, 575 editorial, 21 unclear**. The editorial blocks are
  438 page citations, 88 commentary notes, 43 pointers and 6 compiled
  additions.
- **140 blocks are orphaned** (no marker in the body). 88 of them stay under
  the rule and need their marker restored; 46 go with their notes; 6 are
  unchecked. The main cause is the importer, which rewrote `#D`/`#M` spell
  markers as text and still emitted the note.
- 76 editorial paragraphs sit **outside** footnote blocks, in another 68
  entries. 60 of them are not on AoN or the PRD.
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
| [`scripts/footnote-aon-check-nested.py`](../../scripts/footnote-aon-check-nested.py) | Check of each block against the entry's AoN page and against the PRD text in the corpus. |

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
| `aon_check` | AoN verdict, note and control scores, and the AoN page used. |
| `prd_check` | PRD verdict and note score. |
| `aon_evidence` | A hand AoN/PRD verdict, when one was made. |
| `on_aon_prd` | `yes`, `no`, or `unchecked`. |
| `action` | Owner rule: `keep` (yes), `remove` (no), `check` (unchecked). |
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
not match (AoN check note score 0.17). Other notes match AoN brackets closely,
because both condense the same printed sentence: the deity "unique spell
rules" (for example AoN's
[Shelyn](https://aonprd.com/DeityDisplay.aspx?ItemName=Shelyn),
[Urgathoa](https://aonprd.com/DeityDisplay.aspx?ItemName=Urgathoa),
[Abadar](https://aonprd.com/DeityDisplay.aspx?ItemName=Abadar) and
[Besmara](https://aonprd.com/DeityDisplay.aspx?ItemName=Besmara) pages). The
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
   takes the paragraphs after it as the note.
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
`footnote-aon-check-nested.py` answers it for each block and each outside
paragraph.

**AoN.** For each of the 754 entries, the script fetched one AoN page and
scored it like the print check: the note's four-word shingles on the page,
with the entry's body text as the control (0.3 or more proves it is the
entry's page). The page is the entry's `aon` path (251 entries). Otherwise
the script derived one from the kind and title. Rule pages came from the AoN
rule navigation in `aon-rule-sections.json`, and AoN search was the fallback.
A derived URL is evidence for this audit only; the corpus stores no guessed
path. A note that is not on its entry's page was then matched against all
fetched AoN pages. That catches a table note of *detect evil* copied into
*detect chaos*. A note under four words is matched in its printed key form,
marker then note ("D Domain spell"), because "bonus feat" also occurs in
ordinary text ("gains Toughness as a bonus feat"). The run made 1,154 requests
two seconds apart. AoN never blocked it. It found a usable page for 528 of the
664 entries with footnote blocks.

**PRD.** The PRD check is local first: the corpus's `ref/carried/` entries are
the PSRD-Data parse of the PRD. It then crawled the legacy PRD at
https://legacy.aonprd.com/ for the ten PRD books behind most of the notes
(*Core Rulebook*, *Game Mastery Guide*, *Ultimate Equipment*, *Ultimate
Campaign*, *Ultimate Combat*, *Ultimate Magic*, *Advanced Player's Guide*,
*Occult Adventures*, *Pathfinder Unchained*, *Bestiary* rules), 507 pages
in all, skipping spell and monster folders. PRD matching skips notes
under four words.

**Hand verdicts.** 39 blocks and one outside paragraph got a hand verdict,
recorded in `aon_evidence`:

- Four deity notes are AoN bracket notes in other words. Examples: Geryon's
  "[affects only reptiles]", Gozreh's "[aquatic variant, may also communicate
  with underwater creatures in addition to land]", and Urgathoa's two notes.
  These count as yes.
- The Groetus note is not on its AoN page.
- 34 "Bonus feat." notes on entries without a resolvable page (the elemental
  families, for example) get "no" by pattern. None of the 172 resolved AoN
  monster pages with a B note prints the words.

Results:

| `on_aon_prd` | Blocks | Entries | How |
|---|---:|---:|---|
| yes | 763 | 347 | 540 on the entry's AoN page, 57 on another AoN page, 162 in the PRD, 4 by hand |
| no | 345 | 286 | 310 by the automatic check (page found, note absent, not in the PRD), 35 by hand |
| unchecked | 176 | 91 | No usable AoN page and no PRD match |

By subclass:

| Subclass | yes | no | unchecked |
|---|---:|---:|---:|
| `print-match` | 337 | 144 | 46 |
| `paraphrase` | 84 | 1 | 2 |
| `legend` | 7 | 63 | 4 |
| `source-citation` | 327 | 55 | 56 |
| `commentary` | 0 | 73 | 15 |
| `pointer` | 0 | 5 | 38 |
| `compiled` | 0 | 1 | 5 |
| unclear | 8 | 3 | 10 |

The 144 printed "no" blocks are almost all "Bonus feat." (205 such notes are
classified printed, as `print-match` or `legend`). The marker is printed and
AoN shows the superscript; neither prints the words at the stat block. The
other three printed notes that AoN does not carry are the Groetus note, one
Qinggong Monk note, and the Kortash Khain legend with its unparsed residue. The 327 citations that pass are AoN's "Source" lines, for
example the *Figurine of Wondrous Power* pages list the books for each
figurine.

The 176 unchecked blocks: 106 on rule entries (*Ultimate Equipment* 37,
*Game Mastery Guide* 35, *Core Rulebook* 13, *The Elephant in the Room* 5,
others), whose AoN rule pages have numeric ids that the navigation file does
not list; 56 on *Figurine of Wondrous Power* (citations that AoN prints in a
different format, partial match) and the greater talismans; 5 monsters (the
four idols, *Agnishvatta*); 9 others. Of these, 15 are commentary by wording.

## Orphaned markers

140 blocks have no marker in the entry body:

| Cause | Blocks | What happened |
|---|---:|---|
| `marker-expanded-inline` | 79 | The importer's `spell_entry` rewrote the `#D`, `#M` and `#B` spell markers as the text " (domain spell)", " (mythic spell)", " (bonus spell)" and still emitted the `::mfn` note as a heading. The note now repeats the inline text with nothing pointing at it. |
| `shared-record-copy` | 43 | The note was copied from a shared record whose marker stayed in a sibling entry: *Summon Monster II–IX* (18), *Contact Entity I, II, IV*, the *detect* spells, *minor creation*. |
| `marker-dropped` | 18 | The source marker was lost. Most are monster stat-block notes (C, S, X, F, N). |

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
form). It found 108 paragraphs in 97 entries. A hand review found 76 of them
editorial (68 entries), 29 printed (book prose that says "consult your GM" or
names paizo.com), and 3 unclear. The editorial ones include 22 inline `@FN`
notes on archetypes ("This archetype was published before the void element
was created"), "*Note:* The original text also contains …" on nine
spellbooks, the "These specific breeds do not have published prices" lines on
mounts, and the app plug in `ref/rule/tavern-toolbox`.

Under the owner rule, the 76 editorial outside paragraphs split 60 no and 16
unchecked; the 3 unclear ones are unchecked. None is on AoN or in the PRD.
The 29 printed paragraphs are book prose, not notes (`action` "not a note").

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
   printed notes and the AoN-carried summaries, removes the Pathfinder Data
   commentary that nobody else carries (which also removes the licensing
   question for it), and needs no schema change now: the de-emphasis is a
   rendering rule for footnote headings, which the corpus already marks with
   the `footnote-` anchor.

## Recommendation

Apply the owner rule as one corpus pass per follow-up ticket below:

1. **Remove the 345 "no" blocks and the 60 "no" outside paragraphs.** Record
   each removal in `exclusions.toml`, one `[[excluded]]` row per source record
   (`path = "json/<file>.json#<record>"`) with a reason that names #109, the
   subclass, and the count, for example "2 footnotes not on AoN or the PRD
   (#109): commentary". Remove each removed note's marker reference with it
   (for example the two linked markers in Summon Monster I). For "Bonus
   feat.", keep the printed `^B^` superscript as a plain marker
   (docs/corpus-format.md allows an unlinked superscript) and remove only the
   note. The de-emphasis design ticket can explain the B key once, for
   example with a link to the stat-block rules (`ref/rule/stat-block`).
2. **Keep the 763 "yes" blocks** and show them de-emphasized (the design
   ticket in phynics/cast-iron). No schema change: the reader already tells a
   footnote heading by its `footnote-` anchor.
3. **Settle the 176 unchecked blocks and 19 unchecked outside paragraphs**
   with a targeted AoN look (rule pages by id, the talisman and figurine
   pages, the idols) before removing anything. The 15 unchecked commentary
   blocks are commentary by wording and very likely "no".
4. **Fix the orphaned markers of the notes that stay** (88 blocks). The 75
   inline expansions: restore the `^D^`/`^M^` superscript in place of the
   "(domain spell)" or "(mythic spell)" text, as AoN and print show it. In
   the realm-only mythic group (archdevils and the other lords) this also
   corrects the meaning. Restore or drop the markers of the 10 shared-record
   copies and 3 dropped markers that stay.

Source citations raise one question for the owner: the rule keeps 327 of them
because AoN prints the same pages as source lines. They are metadata, not
notes. Converting them to `[[sources]]` and `[[expanded_in]]` rows (and
removing all 438 citation blocks) would keep the information in the form the
app already shows. The tickets list this as an option, not as a step of the
rule.

## Follow-up tickets

Corpus tickets go to phynics/cast-iron-data; the lint ticket goes to
phynics/cast-iron. Each one rereads this audit's TSV columns, and each corpus
ticket ends with `corpus-check` passing and a pin bump PR.

1. **Remove footnotes that are not on AoN or the PRD** (cast-iron-data).
   Scope: the 345 blocks with `action = remove` in `editorial-footnotes.tsv`.
   Acceptance: rerunning `scripts/editorial-footnotes-nested.py` on the new
   pin lists none of those blocks. Each removed block has an `exclusions.toml`
   row naming its record, #109, and the reason. No marker reference to a
   removed anchor remains (`corpus-check` enforces the linked ones; the plain
   `^n^` ones are checked by the script). Printed `^B^` superscripts stay.
2. **Remove editorial paragraphs outside footnote blocks that are not on AoN
   or the PRD** (cast-iron-data). Scope: the 60 rows with `action = remove` in
   `editorial-notes-outside-footnotes.tsv`, including the inline `@FN` notes
   and the app plug in `ref/rule/tavern-toolbox`. Acceptance: the outside scan
   on the new pin lists none of them; each has an `exclusions.toml` row.
3. **Check the 176 unchecked blocks and 19 unchecked outside paragraphs on
   AoN** (cast-iron-data, research). Acceptance: each row has `on_aon_prd`
   yes or no with evidence (an AoN or legacy PRD URL), merged with
   `--aon-review`; rows found "no" join ticket 1's scope.
4. **Restore orphaned markers of kept footnotes** (cast-iron-data). Scope:
   the 88 blocks with `orphan_cause` set and `on_aon_prd = yes`. Acceptance:
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
   (cast-iron-data, owner decision). Scope: the 438 `source-citation` blocks.
   Acceptance: each cited publication and page is a `[[sources]]` or
   `[[expanded_in]]` row on the entry; no citation block remains.
8. **Lint footnotes in `corpus-check`** (cast-iron). Acceptance:
   `corpus-check` warns on a `footnote-` heading whose marker does not occur
   in the entry body and on `\|X~` residue, with a test for each; the corpus
   format doc names the rule.

The de-emphasis of kept footnotes is the orchestrator's separate design ticket
in phynics/cast-iron.

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

The output is byte-identical on the same pin. At a new pin, rerun the checks
first and pass their output: `footnote-print-check-nested.py CORPUS PDF_DIR
CACHE_DIR --out print.tsv` (needs `pdftotext` and the owner's PDFs) as
`--auto`, and `footnote-aon-check-nested.py CORPUS CACHE_DIR --out aon.tsv` as
`--aon`. Keep both cache directories out of the repository. The AoN check
makes about one request per entry, two seconds apart, and stops on HTTP 403 or
429.
