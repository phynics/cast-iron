# AoN Rules mapping resolution

Checked against the saved Rules.aspx inventory and installed generated library `v171f0348e5717ee0` on 2026-11-03. This is site-navigation mapping evidence, not printed chapter order.

The extractor now accepts reviewed exact source-title aliases and indexes headings in typed `rule` entry bodies in addition to `books.json` nodes. Heading traversal follows nested typed structures, including list/table/quote containers and inline formatting. Heading targets retain entry IDs and anchors; duplicate target headings remain ambiguous, while duplicate evidence for the same target is deduplicated. It retains source boundaries: a section is only matched inside the resolved book. No AoN article content was fetched or imported.

## Current dispositions

The inventory has 283 topic/subsection rows. The mapping now reports 90 unique exact heading/node matches, 49 ambiguous targets, 39 `section_not_in_snapshot` rows in installed books, and 105 `source_not_installed` rows under unavailable books. These are row dispositions, not a claim that the AoN directory is complete. The source heading inventory resolves 9 titles to installed books (including the reviewed GameMastery spelling alias); 14 headings have no installed source.

The 14 missing source headings are: Pathfinder #28: The Infernal Syndrome; Heroes of the Wild; Pathfinder Unchained; Occult Adventures; Ultimate Intrigue; Inner Sea Intrigue; Horror Adventures; Adventurer's Guide; Book of the Damned; Ultimate Wilderness; Pathfinder #128: Songbird, Scion, Saboteur; Planar Adventures; Heroes of Golarion; Pathfinder #144: Midwives to Death.

`docs/research/licensing-audit.md` identifies PCGen's Pathfinder `.lst` data as a possible research lead, but its data redistribution terms are unverified; it does not establish a usable candidate for these specific books. The audit did not identify another licensed, redistributable dataset supplying these missing titles. AoN/legacy pages can verify navigation labels and URLs, not grant redistribution rights. The source content therefore remains absent pending a separate evidence and licensing review.

The installed books' 39 remaining section misses cannot be resolved from exact typed headings or current tree nodes. No cross-book guesses or unsupported semantic aliases were added. The 49 duplicate heading cases remain ambiguous without a unique URL-to-entry or body-heading anchor correspondence. Existing mapping rows preserve candidate entry IDs where duplicates exist.

Regenerate with:

```sh
python3 scripts/extract-aon-rule-sections.py /tmp/cast-iron-aon-rules-source.html docs/research/aon-rule-sections.json --catalog generated/content/v171f0348e5717ee0/catalog.json --books generated/content/v171f0348e5717ee0/books.json --entries generated/content/v171f0348e5717ee0/entries --aliases sources/psrd-data/aon-rule-aliases.json --mapping-output sources/psrd-data/aon-rule-mapping.json
```


Extractor fixture tests: `python3 -m unittest discover -s tests/scripts -v`.
