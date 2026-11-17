---
title: "Reference simplification implementation"
date: 2026-11-03
status: partly-superseded
superseded_by: ["2026-11-17-book-membership-is-derived-from-source-ids"]
issues: [157]
prs: []
area: ui
---

# Reference simplification implementation

- Active-book preferences store disabled canonical book IDs under the existing metadata key `disabled_books`. Absence and an empty set mean all books active. Unknown disabled IDs survive updates, and newly added books default active (new installs change to Paizo Rulebooks when #157 lands). Disabling does not remove cache files or stars.
- `Library` keeps canonical reads. `Discovery` snapshots eligible IDs using book placement, with an explicit catalog-source fallback for older libraries or missing placement. Ordinary discovery uses this scope before counting, limiting, or projecting relations and spell lists. Search publication filters still describe attribution sources, which can differ from book placement.
- Library loading reads preferences and placement before rendering discovery. A corrupt preference value offers Retry or Reset active books, preserving downloaded content and stars. Toggle writes publish a revision only after persistence. That revision rebuilds discovery screens, while Books remains mounted to preserve its controls and open sections.
- Collection UI and manager routes are retired. Existing stored records remain readable, and each unique starred EntryId appears on Home. Missing entries keep their saved title and an Unstar action. Legacy saved-entry URLs recover to Home, and the old Rules-category URL recovers to Books.
- Article contents and heading-aware search use existing typed headings and entry anchors. Matching rule results read installed shards, and exact named rule sections rank ahead of longer title prefixes. Search format 1 and content schema 3 are unchanged. Article contents uses stylesheet classes because inline indentation violated the strict CSP in the first browser run. Clicking an already-current section explicitly scrolls to it.
- Primary rule text precedes relationship discovery. Compact jumps retain research access. Prestige requirements resolve unambiguous feat and skill names only inside their stated sections, with punctuation normalized. Duplicate names remain text instead of choosing a source silently.
- AoN's Rules directory is a navigation baseline and a reproducible mapping inventory. Printed-order evidence and unresolved-section reporting remain separate. No new rule content or source was imported.
