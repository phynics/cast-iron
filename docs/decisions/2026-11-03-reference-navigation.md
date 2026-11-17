---
title: "Reference navigation"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Reference navigation

- Category browsing and book browsing are separate routes. Categories offer alphabetical lists, facets, spell lists, and feat chains. Book pages render only the curated contents tree, preserving its order and showing source notes and review warnings. They do not substitute category lists when chapter data is missing.
- Relation and spell-list indexes load from the installed cache on demand, once per library. Missing schema-3 files in an older library disable the corresponding panels and offer an update route. Ordinary use has no network fallback.
- Racial traits and archetypes link to their parents, and parent entries show the inverse lists. Feat panels retain prerequisite wording alongside named feat links. An `or` label remains visible; these extracted links are not an eligibility calculator.
- `books.json` carries optional source notes and review questions with serde defaults. These additions preserve compatibility with existing schema-3 readers, so the schema remains 3. Entry pages link back to book position and offer previous/next navigation in the curated order.
