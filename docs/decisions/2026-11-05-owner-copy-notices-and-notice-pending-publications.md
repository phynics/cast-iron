---
title: "Owner-copy notices and notice-pending publications"
date: 2026-11-05
status: current
superseded_by: []
issues: [34]
prs: []
area: licensing
---

# Owner-copy notices and notice-pending publications, 2026-11-05

- **The owner supplied three Section 15 lines from the owner's copies of the books.** The reviewed list at `sources/pathfinder-data-1-e/section15/owner-copies.json` keeps those lines verbatim, and its provenance file records the source ("the owner's copies of the books"), the 2026-11-05 supply date, the SHA-256, and the three-row count. `pf_legal` verifies the hash and row count before matching and reports `owner-copy` as the match method. The provenance cites the book only. The notices reach `legal.json` `section_15` and `SourceInfo.copyright` unchanged.
- **Eight publications ship as assumed OGL while notice coverage stays open.** Bestiary 6, Goblins of Golarion, Horror Realms, Faiths of Purity, Inner Sea Taverns, Serpent's Skull Player's Guide, Kingmaker Player's Guide, and Magic of Thassilon have no Section 15 notice in any pinned source. The owner reviewed that list at `sources/pathfinder-data-1-e/section15/notice-pending.json` and chose on 2026-11-05 to ship their records as assumed Open Game Content, tracked in #34. The build treats a notice-pending publication as shippable: its records keep `ogl-1.0a` and `legal#community-use`, and `SourceInfo.copyright` carries "`<Title>`: Section 15 notice pending; Open Game Content used under the OGL." The legal page adds a **Section 15 notice pending** list, and the migration reports carry the rows under `pending_publications` and `notice-pending`.
- **Matched and pending are mutually exclusive.** `pf_legal::build` fails when a title appears in both the matched and pending sets, or when a notice-pending name has no in-scope records. `blocking_publications` and `gate` ignore pending rows, so only the 11 Dynamite comic labels still block.
- **The legal page field is additive.** `LegalNotices.notice_pending` is optional with a serde default and is omitted when empty. Old readers ignore it, and new readers treat an older `legal.json` as an empty list. `SCHEMA_VERSION` stays 3 and `INDEX_FORMAT_VERSION` stays 1.
- **Results on the pinned corpus.** 453 publications: 434 matched, 8 pending, and 11 unmatched comic labels. The canonical library grows from 29,515 to 31,106 entries. The legal screen rejects 21 records, every one naming only a comic. Two canonical compilations are byte-identical at content version `vf32215920f3724aa`.
