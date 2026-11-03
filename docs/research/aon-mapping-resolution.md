# AoN rule target resolution

Issue [#11](https://github.com/phynics/cast-iron/issues/11) tracks missing and ambiguous book content. Checked on 2026-11-03 against installed library `v171f0348e5717ee0`. AoN navigation identifies site sections; it does not establish printed book order.

## Current targets and limits

| Disposition | Rows |
|---|---:|
| Unique exact title target | 90 |
| Source-validated opening passage target | 59 |
| Source-validated chapter context | 2 |
| No verified target in an installed source | 27 |
| Source not installed | 105 |

The inventory has 283 topic/subsection rows under 23 source headings. All 49 formerly ambiguous rows now have target evidence: 47 match an opening passage and two match chapter context. Twelve formerly unmatched labels identify existing content, including Character Creation, Racial Favored Class Bonus, Effect Word Descriptions, Downtime Overview, and the misspelled Two Quirks title. The Optional label points to an unheaded paragraph within Words of Power.

A passage target requires an exact source match and at least 12, up to 24, opening words from the primary page. The installed target can be an article or the nearest typed heading containing that passage. A unique dedicated article with the same title takes precedence over an introduction containing a copy. Root and leading-heading links to the same paragraph are equivalent. Unresolved ties remain unresolved; no fuzzy name match or cross-book target is accepted.

The [AoN Alignment page](https://aonprd.com/Rules.aspx?ID=113) identifies the alignment rules rather than the glossary definition. The [Bestiary Introduction](https://aonprd.com/Rules.aspx?ID=2449) opening matches the nested reading entry rather than its parent container. [Two Quirks](https://aonprd.com/Rules.aspx?ID=1244) spells Establishment differently from the installed heading. The evidence records primary URLs, source/page labels, paragraph hashes, chosen IDs/anchors, and candidate targets without redistributing the fetched prose.

The two chapter cases are [Magic](https://aonprd.com/Rules.aspx?ID=68) and [Magic Items](https://aonprd.com/Rules.aspx?ID=365). Their canonical chapter containers share 41 and 52 descendant headings with the primary pages. These matches identify reading context, not complete text. Magic has no opening prose at its top-level AoN heading. Magic Items has opening prose that is absent from the installed snapshot. These limits remain in the evidence. AoN reports page 83 for the top-level Magic heading while its children begin at page 206; that metadata does not supersede the publisher contents evidence.

The 27 unmatched rows include 13 navigation headings without qualifying opening prose and 14 opening passages not matched in installed content. A `section_not_in_snapshot` status therefore means no verified target, not proof that every related rule is absent. No new book, rule text, or printed-order assertion was imported by this mapping work.

## Sources for the remaining prose

The passage audit also checked pinned Pf Data 1e descriptions. Thirteen of the 15 unmatched opening passages, including the Magic Items chapter introduction, have a source-tagged candidate record. No candidate opening passage was found for the Bestiary or GameMastery Guide book introductions. Each candidate includes its exact JSON pointer, file/description hashes, source tags, and unsupported-directive inventory. These are candidates for a native adapter, not newly installed content.

The [missing-book source audit](missing-books-source-audit.md) and its [machine-readable inventory](missing-books-source-inventory.json) separately inspect all 14 unavailable source headings. Pf Data 1e contains source-tagged records for each; PCGen has matching manifests for nine. Counts describe stored occurrences and referenced files, not complete books or automatic publication clearance. Start a bounded native import with the complete Magic and Spellcasting record at `json/rules_basics04.json#/magic`, which matches [Magic Basics](https://aonprd.com/Rules.aspx?ID=202), before expanding to more complex families. This bounded import is tracked in [#19](https://github.com/phynics/cast-iron/issues/19).

## Reproduction

Use the installed corpus paths below. Fetch the source directory page into `/tmp/cast-iron-aon-rules-source.html`. The article fetch is bounded to unresolved rows in installed sources, sequential, and cached outside the repository. No raw HTML or prose cache is committed.

```sh
# Recreate the title-only baseline, independently of selected evidence.
python3 scripts/extract-aon-rule-sections.py /tmp/cast-iron-aon-rules-source.html /tmp/aon-inventory.json --catalog generated/content/v171f0348e5717ee0/catalog.json --books generated/content/v171f0348e5717ee0/books.json --entries generated/content/v171f0348e5717ee0/entries --aliases sources/psrd-data/aon-rule-aliases.json --mapping-output /tmp/aon-baseline.json

# Fetch only uncached reference pages; omit --fetch to replay the saved inputs offline.
python3 scripts/audit-book-content.py --mapping /tmp/aon-baseline.json --entries generated/content/v171f0348e5717ee0/entries --pages-dir /tmp/cast-iron-aon-rule-pages --aliases sources/psrd-data/aon-rule-aliases.json --pf-data-dir /tmp/pf1-data-audit --fetch --output /tmp/aon-evidence.json

# Apply the reviewed evidence and fail on stale corpus/book hashes or invalid targets.
python3 scripts/extract-aon-rule-sections.py /tmp/cast-iron-aon-rules-source.html /tmp/aon-inventory.json --catalog generated/content/v171f0348e5717ee0/catalog.json --books generated/content/v171f0348e5717ee0/books.json --entries generated/content/v171f0348e5717ee0/entries --aliases sources/psrd-data/aon-rule-aliases.json --evidence sources/psrd-data/aon-rule-evidence.json --mapping-output /tmp/aon-mapping.json

PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/scripts -v
```

Review changed audit evidence before replacing the committed evidence file. Regeneration rejects missing entries, wrong sources, missing/duplicate anchors, invalid chapter paths, changed labels, unused evidence, and changed entry/book-index hashes. Replaying a saved cache is deterministic; live page changes require a new evidence review. The compiler does not consume this research mapping, so target coverage must remain separate from actual installed corpus coverage.
