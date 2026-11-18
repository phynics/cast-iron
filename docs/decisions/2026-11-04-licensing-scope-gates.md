---
title: "Licensing scope gates"
date: 2026-11-04
status: partly-superseded
superseded_by: ["2026-11-18-paizo-blog-source-bases-under-cup"]
issues: [5, 34, 36]
prs: []
area: licensing
---

# Licensing scope gates

- PF2 material in the pinned Pf Data registry is out of scope. Two PF2 publications are registered (`Monster Core`, `Bestiary 3 (2nd Edition)`); eight records are tagged with them. Two more records carry PF2 text without a PF2 tag: `json/main.json#ogc` and `json/equipment_weapons2.json#gnome_flick_mace`. `sources/pathfinder-data-1-e/out-of-scope.json` excludes all ten before selection. ORC text never enters the OGL legal page. The filter is clean for known contamination; untagged PF2 prose still needs per-record review before a bulk cutover.
- Comics are out of scope permanently. Records whose only publications are *Pathfinder Comics* or *Pathfinder Spiral of Bones* are excluded, including Apollyon Clasp and *Spiral of Bones #5*; no further evidence will be sought. A record also published in an in-scope PF1e book stays.
- Paizo Blog publications are ignored for now and tracked in issue #36 for a possible return. Records whose only publication is a blog are excluded; records also published in an in-scope PF1e book stay.
- Every record in `sources/pathfinder-data-1-e/approved.json` must now name a required `rights` basis. `ogl` imports under the canonical `ogl-1.0a` reference. `cup` imports under both `ogl-1.0a` and the legal-page Community Use notice path, `legal#community-use`. `orc` is refused because the legal page has no ORC notice path, so ORC text cannot be presented under the OGL. `excluded` is never imported; the record appears in coverage with the reviewed reason. Any other value fails selection. The initial Magic and Spellcasting record is recorded as `ogl`. Schema 3 and search format 1 are unchanged.
- PSRD-Data has no dataset-level license. That risk is accepted for the existing 16-book baseline, overridden for now, and tracked with the other unresolved rights in issue #34.
- The pin hashes `ORCLICENSE.md` and `GNULICENSE.md` even though their content is not imported, so a changed or missing out-of-scope notice fails the source audit.
