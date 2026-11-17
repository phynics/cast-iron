---
title: "Reverse body-link index"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: content
---

# Reverse body-link index, 2026-11-05

- **The index is optional and additive, so the schema stays 3.** `pipeline::package` writes `backlinks.json` plus `backlinks/<NN>.json` from the same entries that the shards hold. Both are listed in the manifest, hashed, and covered by `content_version`. A library built before the index has no descriptor and the client treats the index as empty; an older client ignores the extra manifest files. The descriptor carries `BACKLINK_FORMAT` 1, so adding or removing it does not change `SCHEMA_VERSION` or `INDEX_FORMAT_VERSION`. `CoverageReport::backlinks` is serde-defaulted for older `coverage.json` files.
- **Self-links and typed-relation links are dropped.** A link from an entry to itself is not a reference the reader needs while standing on that page. A link whose source already states a typed relation to the same target (`part`, `archetype`, or `prerequisite`) would duplicate the relation panels, so it is left out. Both counts appear in `coverage.json` and `coverage.md`.
- **One row per source and target; an anchor survives only when it is unique.** The compiler keeps the heading anchor when one source reaches the target through exactly one anchor. Several anchors, or a mix of an anchor and the entry top, keep no anchor because no single heading is the reference. The client repeats the collapse for any older compiler output.
- **Sixteen FNV-1a shards, not a literal id prefix.** The canonical index is 5,172,403 bytes for 134,391 rows over 16,199 targets, 2.29% of the 226,239,516-byte library. That is below the 10% sharding threshold, but reading and parsing the whole file on every entry view is wasteful on mobile. Canonical ids all start with `ref/`, so a literal prefix would not split them; `backlink_shard` hashes the target id with FNV-1a modulo 16 instead. Shards are 344–408 KB, the client keeps four parsed shards in an LRU, and one entry view reads only its bucket. The shard count is in the descriptor, so a later change does not need a client release.
- **The panel groups by publication and caps the preview.** "Referenced by" is in the same sticky side column as References. Rows group by publication title, groups with three or fewer rows open by default, and a group shows its first eight rows with a "Show all N" disclosure. Rows are ordinary links and native `<details>` disclosures, so keyboard and screen-reader use need no script. A missing index, a missing row, or a read error hides the panel and leaves the entry readable.
