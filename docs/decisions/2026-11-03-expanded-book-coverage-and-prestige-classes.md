---
title: "Expanded book coverage and prestige classes"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: content
---

# Expanded book coverage and prestige classes

- The expansion uses every available book in the pinned snapshot, rather than fetching a newer or different dataset. Selection is explicit and generated from a reviewed 16-book inventory. Unsupported standalone types remain listed in coverage; many are represented inside the selected parent rules trees. This is broader book coverage, not a claim of every published PF1e book or complete standalone category coverage.
- Prestige classes have a distinct schema kind while retaining `ClassFields` and existing IDs. The content schema is version 2; the new client deliberately reads versions 1 and 2 so an old verified library stays usable during an update. The search format stays unchanged. Technology Guide stores Technomancer as a section, so the reviewed source configuration explicitly classifies that URL as a prestige class.
- Placeholder `PFSRD` source labels inherit the reviewed enclosing publication. Explicit non-placeholder source labels remain intact. The catalog and legal page therefore identify the actual books rather than a generic source bucket.
- Named stories and setting material are omitted before deduplication and rendering. Their locations and reasons are part of coverage. Absent optional equipment fields are treated as absent, rather than rejecting the whole record.
- Abrakarn Viper has both creature statistics and construction-item metadata at one upstream URL. The creature remains canonical and retains the supplemental item fields and both provenance paths. Its public identity is unchanged. Conflicting nested copies remain visible in the validation notes.
- The full-corpus text-preservation test now follows URL-addressed records inside trees and exempts only reported policy removals or typed copies rendered separately. Its independent text extraction decodes HTML entities at text-node boundaries and ignores discretionary soft hyphens, matching what users read.
