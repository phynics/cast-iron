# Cross-checking the corpus

How to compare the corpus with outside datasets, what to do with a difference, and when a
difference goes to print or to the owner. The approved datasets and the first results are in
[research/cross-check-sources.md](research/cross-check-sources.md). The rule behind this page is the
decision [outside datasets are witnesses](decisions/2026-11-24-outside-datasets-are-witnesses.md).

Three terms:

- **Witness:** an approved outside dataset. It can be wrong.
- **Candidate:** a difference between the corpus and a witness. It is not yet an error.
- **Print check:** reading the printed book. Only a print check confirms an error.

## When to run a comparison

| Trigger | What to compare |
|---|---|
| A corpus tag changes the fields of a kind in bulk (a script rewrote them) | That kind, before the tag is pinned |
| A corpus pin in this repository | The kinds the tag changed, after the pin |
| Before a release promotion | Every compared kind, once, against the release candidate |
| A reader or reviewer reports a wrong value | That entry, plus the other entries of the same book and kind |
| A new witness is proposed | The three admission checks in the decision, then a full first run |

Do not run a comparison for a change that touches no compared field: body prose, book order,
citations, the client.

## The steps

Work through the steps in order. Stop at the first step that settles the candidate.

1. **Compare.** Match entries by title (normalised title, then the set of words, then the title
   without brackets) and compare the typed fields of each matched pair. Until
   [#668](https://github.com/phynics/cast-iron/issues/668) delivers the script, this is a hand run.
2. **Sort each candidate into one class.**

   | Class | Example | Next step |
   |---|---|---|
   | Format only | `10 min./level` against `10 minutes/level`; `30 feet` against `30 ft.` | Drop it. Add the pattern to the comparison's normalisation |
   | Witness defect | A title with a missing space; a truncated cell | Drop it |
   | Coverage | The witness lacks a book or a class column | Drop it. Record the gap once for the witness |
   | Corpus typo | `conecentration`; `1 round/leve` | Step 4 |
   | Value conflict | Level 2 against level 3; 360 days against 60 days | Step 3 |
   | Title conflict | Two spellings of a title | Step 3 |
   | Missing entry | A name with no corpus entry of that kind | Step 3 |

3. **Weigh the witnesses.** Count only independent witnesses (the source page says which ones share
   an origin).

   | Evidence against the corpus | Priority |
   |---|---|
   | Two independent witnesses agree with each other | High: check print first |
   | One witness, and the corpus value also breaks a pattern (a "greater" spell below its base spell, a value equal to a neighbouring entry's) | High |
   | One witness | Normal |
   | Witnesses disagree with each other and with the corpus | Normal; print is the only way to settle it |

4. **Check print.** Read the entry in the printed book: the book and page of its `[[sources]]` row.
   When the entry has several printings, read each; a later printing can change a value, and then
   both values are right for their book.
5. **Record the result**, whichever way it went:
   - **Print agrees with the witness:** fix the corpus in a pull request on the corpus repository.
     Cite the book title and the page in the pull request. Change `title` or the field, never the
     `id` or the file path.
   - **Print agrees with the corpus:** comment on the ticket, and add the row to the
     known-differences list so the next run does not report it.
   - **Print agrees with neither:** fix the corpus to print.
6. **Close the loop.** After the corpus tag is pinned here, run the comparison for that kind again.
   The fixed rows must be gone and no new row may appear for the entries the fix touched.

A fix changes both copies of a value. Most typed fields also appear in the body stat block, and
`corpus-check` fails when the two disagree.

## Tickets

- Candidates go to the corpus tracker, grouped by kind and class, not one ticket per row. A group
  of more than about 30 rows gets a review file (a TSV under `scripts/` in the corpus repository,
  named for the ticket) with the columns `id`, `field`, `corpus`, `witness`, `witnesses`, `print`,
  `book`, `page`, `action`.
- The ticket names the witness and its snapshot (commit or download date). It does not quote
  witness text beyond the value compared.
- Public text cites the book title and page. It does not cite a website, a file path or a file
  name as the authority for a fix.

## When to escalate to the owner

Stop and ask before acting when:

- **The fix is not a value correction.** Adding an entry, adding a publication, changing which
  books an entry belongs to, or merging two entries.
- **Printings disagree.** Two printings of one book, or a book and a later book that reprints the
  entry, give different values, and no `revision` relation models it yet.
- **Print looks wrong.** The printed value is an evident misprint, or official errata changed it.
  The corpus follows print unless the owner decides otherwise for that entry.
- **A name is involved.** A witness uses another name for the same text. Check first that the
  witness is not replacing a Product Identity name; never rename toward a generic name.
- **No print copy is available** for the book.
- **A pattern appears.** The same error shows in many entries of one book or one import batch. That
  is an import defect: it needs a script and a review file, not row-by-row fixes.
- **A witness turns out unreliable or derived.** More than half of its candidates for a kind fail
  the print check, or it proves to copy another witness.

Everything else, a reviewer of the corpus pull request can approve.

## What never happens

- A witness value is never copied into the corpus without a print check.
- No witness text enters the corpus, the app or a ticket.
- An `id` never changes because its title was misspelled.
- A comparison never runs in the build, in CI or at runtime. It reads local snapshots only.
