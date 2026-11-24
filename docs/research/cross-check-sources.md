# Cross-check sources for the corpus

Checked 2026-11-24 against the pinned corpus v0.1.42 (31,735 entries, 443 publications). This page
records which outside datasets Cast Iron compares the corpus with, what the first comparison found,
and what comes next. The decision is
[2026-11-24-outside-datasets-are-witnesses.md](../decisions/2026-11-24-outside-datasets-are-witnesses.md).

A comparison is a way to find errors. It is not a source of text. Nothing from these datasets enters
the corpus or the app (AGENTS.md rule 12), and no code from them is copied. The earlier survey of
import candidates is [pf1e-dataset-candidates.md](pf1e-dataset-candidates.md).

## The sources

| Source | Snapshot | Content | Basis it states | Role |
|---|---|---|---|---|
| Community sheets, linked from [pathfindercommunity.net](https://home.pathfindercommunity.net/home/databases) | Downloaded 2026-11-24 | Google Sheets: 2,905 spells, 3,072 feats, 4,241 magic items, 6,748 NPCs, 233 afflictions, with one column per statistic | Community Use notice | Witness for spell, feat, magic item and affliction fields |
| [Nietsneflow/open-reference](https://github.com/Nietsneflow/open-reference) | `725d98f` | PSRD data of 2016 from a discontinued Android app: 19 hardcover books, about 31,000 records | OGL 1.0a with Section 15; code GPL-3.0 | Witness for the 19 shared books: titles, creatures, items |
| [baileymh/pf1e-content](https://github.com/baileymh/pf1e-content) | `baf5232` (last export 2023-03) | Foundry module: about 3,500 feats, 1,900 traits, 1,100 racial traits, 4,450 class abilities, 4,200 magic items. No spells | OGL Section 15 list and Community Use notice; code GPL-3.0 | Witness for trait and racial trait titles; second look at feats and items |
| [BurcinSayin/rpg_parser](https://github.com/BurcinSayin/rpg_parser) | `main`, 2026-10-17 | A scraper for spell pages. It ships sample files and no dataset | MIT code; data left to the user | Not a source |

SHA-256 prefixes of the sheet downloads: spells `78ae8d11c4f72c0e`, feats `7c42718dad7f0e81`, magic
items `a5d50f83126709bb`, NPCs `fb27f2d0caddaa05`, afflictions `ba9e9cd73c9d166d`. The sheets carry
no version, so a later download can differ.

Three limits on the witnesses:

- **The Foundry module is not independent of the sheets.** Its credits name the community feat and
  magic item sheets as its origin. For feats and magic items the two count as one witness.
- **The PSRD data is not independent of the PSRD carry-over** in the corpus. It is independent of the
  Pathfinder Data records, which are most of the corpus.
- **The NPC sheet mixes publishers.** It holds rows from third-party adventures. Use only rows whose
  source is a publication in the corpus.

## Product Identity names

The corpus ships Paizo names under the Community Use Policy. A dataset that cannot rely on that policy
may replace such names with generic ones, and a comparison against it would then report false
differences, or tempt an editor to "correct" a printed name.

None of the three datasets does this:

- Feat titles that contain a deity, nation or organisation name: 26 in the corpus, 22 in the Foundry
  module, 20 in the community sheet.
- For each such corpus title that a dataset lacks by name (feats, traits, items, archetypes), the
  nearest record by body text was either the same name with other punctuation, or no match at all. No
  record had the same text under a generic name.
- The PSRD data covers only the hardcover rulebooks, which print almost no setting names. Its titles
  show no rename in the 19 shared books.

d20PFSRD does replace Product Identity names. It is not a witness for names.

## First comparison

Titles were matched in three passes: the normalised title, the set of words (so `X, Greater` meets
`Greater X`), and the title without its brackets.

| Comparison | Source rows | Only in the source | Only in the corpus |
|---|---|---|---|
| Spells, community sheet | 2,905 | 14 | 133 |
| Feats, community sheet | 2,933 | 58 | 675 |
| Feats, Foundry module | 3,537 | 10 | 82 |
| Traits, Foundry module | 1,915 | 59 | 83 |
| Spells, PSRD (shared books) | 1,686 | 2 | 30 |
| Feats, PSRD (shared books) | 1,296 | 3 | 131 |
| Creatures, PSRD (shared books) | 1,886 | 42 | 63 |
| Racial traits, PSRD (shared books) | 252 | 0 | 1 |

Most "only in the corpus" rows are newer books, mythic forms, or a different grouping (the PSRD data
files racial feats under the race). Most "only in the source" rows are spelling differences. The
magic item and class ability title comparisons are too noisy to read without per-family rules
(`Cloak of Resistance +1` against one entry with a table), and are left for the tool.

Spell fields were compared on 2,889 matched spells. The sheet is wrong often: where a second reading
was available for a disagreement on casting time, duration, range or level, the corpus was right in
about three cases of four. A disagreement is therefore a prompt for a print check, never an edit.

Findings, each a ticket on the data tracker:

| Finding | Ticket |
|---|---|
| Major Image carries the level and duration of Minor Image; Secret Vault duration; five typos in field values; 20 value conflicts to resolve | [phynics/cast-iron-data#282](https://github.com/phynics/cast-iron-data/issues/282) |
| 24 title spellings to check (for example Abjuring Step against Adjuring Step in two witnesses) | [phynics/cast-iron-data#283](https://github.com/phynics/cast-iron-data/issues/283) |
| 251 class levels on 172 spells that the sheet has and the corpus lacks, mostly unchained summoner and the occult classes | [phynics/cast-iron-data#284](https://github.com/phynics/cast-iron-data/issues/284) |
| Names that match no corpus entry: 6 spells, 19 feats, about 40 faction traits | [phynics/cast-iron-data#285](https://github.com/phynics/cast-iron-data/issues/285) |

Not compared yet: feat prerequisites and types, magic item statistics, creature statistics,
affliction fields.

## Plan

The procedure for a run (when to compare, how to sort a difference, when print or the owner decides)
is [cross-checking.md](../cross-checking.md). A review of the corpus model that the first run
prompted is [corpus-model-review.md](corpus-model-review.md).

1. **Settle the findings.** The data thread works the four tickets above. Each fix follows a print
   check and cites the book and page. A row where print agrees with the corpus is recorded, so it
   does not return.
2. **Make the comparison repeatable** ([#668](https://github.com/phynics/cast-iron/issues/668),
   milestone `post-v1`). One research script reads the corpus directory and local snapshots of the
   sources, and writes TSV candidate lists under `generated/`. It makes no network request, edits
   nothing, and records the commit or hash of each snapshot.
3. **Extend the fields**, in the order that the same ticket lists: feats (types, prerequisites),
   magic items (price, cost, caster level, slot, aura, weight), creatures against the PSRD data (CR,
   type, size, alignment), afflictions, then trait and racial trait titles.
4. **Run it after a corpus pin that changes a compared kind**, and file only what is new. A
   known-differences file holds the rows that print settled in favour of the corpus.

Rules for every run:

- Two independent witnesses against the corpus make a strong candidate; one makes a prompt to look.
- Print decides. A witness never does.
- A title correction changes `title` only. The `id` and the file path stay (AGENTS.md rule 3).
- A name that is missing from the corpus becomes an entry only through the corpus repository, with
  its publication row and notice, or it gets a row in `exclusions.toml`.
