---
title: "Carry-over of unmatched legacy ids"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Carry-over of unmatched legacy ids, 2026-11-05

- **Every legacy id without a Pf Data counterpart is carried, not dropped.** The maximum-OGC rule from the Pf Data cutover policy applies to the remaining `no_source` rows: they are re-emitted from the PSRD snapshot at `ref/carried/<legacy-id>` instead of being retired. Body, fields, source ids, license refs, and PSRD provenance stay unchanged. The carried entry is a first-class canonical record, not a compatibility shim. Rights, out-of-scope, and unrepresentable retirements stay retired.
- **A possible duplicate is a review lead, never an automatic match.** If a native or derived canonical entry has the same kind and the same normalized title (the alias module's comparison form) in any book, the carried entry is still produced and the pair is listed under `carry_over.possible_duplicates` with the candidate ids. Title equality alone does not prove the same article, so it cannot change the map.
- **The map is the explicit carried set.** Each remainder row is now `[map."<legacy-id>"]` with `target = "ref/carried/<legacy-id>"`. Carry-over reads those rows, and `cargo xtask data-alias` adds exactly the produced entries to its canonical catalog. A carried target with no produced entry fails the target gate; a collision is a build error.
- **References are rewritten, and an unresolvable one is visible.** A target that maps to a canonical id becomes that id, a target that is carried becomes its carried id, and any other legacy target is flattened to plain content and counted. Parent, relation, and feat-prerequisite references follow the same rule; an unresolved parent is dropped and counted. The real corpus currently rewrites 7,238 references and flattens none, because the map is total.
- **This step is isolated from production.** The carry-over module feeds the alias gate and `reports/migration-coverage.json` only. `cargo xtask content` still produces byte-identical PSRD output, so the cutover is not live.
