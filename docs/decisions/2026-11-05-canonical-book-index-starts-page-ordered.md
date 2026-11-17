---
title: "Canonical book index starts page-ordered"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Canonical book index starts page-ordered, 2026-11-05

- **Page attribution is structured evidence, not body text.** The canonical converter now reads each screened record's `‹SOURCE Publication/page›` markers and `source="Publication/page"` directive attributes into `Converted.pages`, keyed by canonical entry id. A record may name several publications; a range or list keeps its first page, and a repeated publication keeps its earliest page. A derived child inherits its host's attribution unless its own extracted lines carry a SOURCE marker. This is additive: no body text, `SCHEMA_VERSION`, or search format changes.
- **One book per publication, ordered by page.** `books::build_canonical` creates a `BookTree` for every publication that has entries. Sibling order is page, then kind slug, then normalized title, then id. A `NestedLink` child stays under its host, so it follows the host even when its own page number is lower. Entries on the same page are not in printed order. An entry in several publications appears in each book, which the client already handles by taking the first place in book order.
- **The canonical index has no curated chapters yet.** `book-order.toml` matches PSRD `pfsrd://` urls, not Pf Data records. Only the Core Rulebook has a verified printed contents page sequence, and mapping one dataset's chapter titles onto another dataset's records would add a second, unreviewed identity step. The canonical index therefore uses one page-ordered list plus an explicit Unpaged section per book. The curated PSRD chapters remain a later refinement, to be mapped only where page evidence supports it.
- **Unpaged and unattributed entries stay visible.** An entry whose source line names a publication but gives no page goes under that book's `Unpaged` chapter and into `BookOrderCoverage.unplaced` with the reason. An entry that names no publication at all is reported there too and cannot enter a book. Nothing is silently dropped (AGENTS.md rule 6). `BookOrderCoverage.placed` counts entries with at least one page, so placed plus unplaced still accounts for every entry.
- **Full-corpus counts are reporting evidence.** `data-report` and `data-import` write `books.json` and `book-coverage.json` under the ignored migration output and record the counts in the migration report. The full pinned corpus yields 454 books, 27,853 entries, 24,129 placed by page, 3,724 unpaged, 4,872 in more than one book, and 35,700 placements. The representative run yields 49 books and 42 entries. Production PSRD output is unchanged (`vdb43853600b8ed8b`, 9,968 entries).
