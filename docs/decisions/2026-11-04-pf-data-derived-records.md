---
title: "Pf Data derived records"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: [46]
area: corpus
---

# Pf Data derived records, 2026-11-04

- **Two families are mechanical and derive before review.** Standard racial traits of the seven Core Rulebook races are split out of their `::rtraits` directive; named inline `::aff` blocks in monster and rule records become standalone afflictions. IDs are `ref/racial-trait/{race-slug}/{trait-slug}` and `ref/affliction/{host-slug}/{affliction-slug}`, both from the existing `slug_segment` contract. This is the first step of the canonical-ID cutover (PR #46). It does not change `SCHEMA_VERSION` or `INDEX_FORMAT_VERSION`.
- **A collision is a rejection, not a suffix.** A later duplicate ID is reported with both identities and skipped, so derived output is always injective. Only the first claimant is emitted; there is no automatic numbering.
- **Coverage counts the corpus; the run counts entries.** `data-report` plans over the whole in-scope inventory (67 racial traits, 155 afflictions, 332 unnamed `::aff` blocks rejected) while `migration-*.json` reports the entries a sample or full run actually built. A host that still fails conversion does not suppress its self-contained derived record; that record is emitted without a `parent` link and keeps provenance to the upstream path and directive.
- **Derived bodies reuse the parent Markdown path.** `::rtraits` and `::aff` render through `pf_directives`; a converted parent replaces the directive with links to its children, so the text is not duplicated.
- **Attribution is inherited from the parent record.** Derived records take the parent's `source_ids` and license references, as the design requires. More precise per-trait book attribution belongs to the reviewed alias work in phase 4.
