---
title: "Reviewed primary-source records during the migration"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Reviewed primary-source records during the migration

- Pin Pf Data 1e independently and convert its descriptions in native Rust to the existing schema-3 nodes. Source formats, directives, and renderer code do not reach the client. Compiler-only additions do not change the serialized schema or search format.
- Keep full-corpus experiments outside the production library. Unknown constructs, unresolved references, missing credits, and Product Identity screening remain explicit publication gates. The comic fixture requires primary evidence for its appendix's OGC designation and Section 15 notice. Paizo Blog rules have a separate Community Use permission route; that permission excludes comic excerpts.
- Allow whole records into production only through `sources/pathfinder-data-1-e/approved.json`. The initial selection restores the missing Core Rulebook page 206 opening passage. It has its own permanent native-protocol ID and retains all previous IDs. Existing parent and reviewed topic IDs provide navigation; unreviewed topics remain in coverage.
- The selected passage belongs beneath the existing Magic article. Its page attribution and the publisher evidence for the Magic chapter establish that placement. This does not revise printed chapter order or claim completeness for the full upstream corpus. PSRD-Data remains the fallback while the full migration is blocked.
