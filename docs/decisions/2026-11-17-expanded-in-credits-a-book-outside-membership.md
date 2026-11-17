---
title: "expanded_in credits a book outside membership"
date: 2026-11-17
status: current
superseded_by: []
issues: [457, 458]
prs: []
area: corpus
---

# `expanded_in` credits a book outside membership, 2026-11-17

Closes [#458](https://github.com/phynics/cast-iron/issues/458). Part of [#457](https://github.com/phynics/cast-iron/issues/457). It builds on [book membership is derived from source_ids](2026-11-17-book-membership-is-derived-from-source-ids.md).

- **The problem.** `source_ids` holds the books that print an entry or a section of its body. Some books only expand an entry, and the expanding text has no entry of its own. The 22-row audit in #457 found six such rows (Summon Monster via the Monster Summoner's Handbook simple templates). Putting those books in `source_ids` makes the entry a member of a book that does not print it.
- **A new optional table, `[[expanded_in]]`.** Each row has `publication`, an optional `page` and an optional `via` entry id. The compiler stores it as `RuleEntry.expanded_in: Vec<Expansion>` with `serde(default)`, omitted when empty. The change is additive, so `SCHEMA_VERSION` stays as it is, and a corpus without the table compiles to the same bytes as before.
- **Never membership.** Membership, book counts, book filters, the search extra sources and the page-order derivation read `source_ids` only. None of them reads `expanded_in`. A test compiles one corpus with and without the table and compares the catalog, the counts and the book contents.
- **Checks.** `corpus-check` fails when the publication does not exist, when it is in the entry's `source_ids` (a book that prints the entry is a member, so the row is redundant and a leak), when `via` does not resolve, or when the `via` entry's `source_ids` do not include the publication. The last rule keeps `via` honest: it names the entry through which the book's text appears.
- **Rejected.** A `role` on `[[sources]]`: every reader of `sources` would have to filter it. A relation kind: a book is not an entry. When the expanding material has its own entry, the existing relation (`variant`, `addon`, a link) credits it and no `[[expanded_in]]` row is needed.
- **Not done here.** The "Expanded in" line on entry pages is #460, the data moves are #459 and #461.
