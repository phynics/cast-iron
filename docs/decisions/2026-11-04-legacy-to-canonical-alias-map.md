---
title: "Legacy to canonical alias map"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Legacy to canonical alias map, 2026-11-04

- **The totality oracle is frozen.** `sources/canonical/legacy-ids.json` holds the 9,968 identifiers the last PSRD production build published, with kind, title, and source, sorted by id. It is exported once and never regenerated from a later build. `sources/canonical/id-map.toml` pins its `sha256` and count in `[meta]`, so an edit to the oracle is a visible, reviewed change rather than a silent rewrite of what bookmarks may hold.
- **The map has three explicit tables.** `[map]` resolves a legacy id through a native upstream identity (`record = "path#key"`), a derived child identity (`derived = "path#key|<kind>:<slug>"`), or an explicit `target = "ref/..."`. `[shared]` is the only way two legacy ids may name one canonical record, and each row needs a note. `[retired]` records `rights`, `out_of_scope`, `no_source`, or `unrepresentable`. Exactly one disposition per legacy id; unknown fields, duplicate dispositions, and canonical ids used as legacy keys fail parsing.
- **Generation verifies the gates, it does not repair them.** `cargo xtask data-alias` rebuilds the canonical catalog from the pinned Pf Data inventory (native `ref/<protocol>/<key>`, planned `ref/racial-trait/...` and `ref/affliction/...`, and extracted allowlisted `ref/rule/...`), then fails on an unaccounted legacy id, an identity the pin cannot produce, a target that does not exist, a chain, a merged target that is not declared `[shared]`, or a `[map]` row whose legacy title and source book do not fit the canonical record. Output is sorted by legacy id, has a fixed field order, and ends with a newline, so two runs are byte-identical.
- **A shared slug is not a match.** The semantic fit gate compares the legacy snapshot's `source` against the canonical record's full publication set (folding `PRPG Core Rulebook`, `Pathfinder RPG Bestiary`, and `GameMastery Guide` to their legacy book ids) and its `title` against the canonical title after case, punctuation, and spacing normalization. Ninety-six rows that only shared a slug moved to `[retired]` with the specific note `derived-rule slug coincidence; not a verified match`. One of them, `ultimate-equipment/rules/wondrous-items/chest`, passes the mechanical checks but was retired in review because the legacy page is the chest slot category and the target is the Chest (huge) item's section.
- **Current state is deliberately incomplete.** 94 legacy ids are mapped (1 direct, 93 derived), none are shared, and 9,874 are retired as `no_source`. A `no_source` row means "no canonical derivation or approved carry-over exists yet"; it is not a claim that the Pf corpus lacks a counterpart. The 8,088 direct-match candidates and the racial-trait, affliction, and rule gaps stay explicit rows until a reviewed audit accepts them. Phase 5 may begin on this format, but no release note may say the gap is closed while those rows remain.
- **The generated file joins the manifest in phase 5.** `cargo xtask data-alias` writes `aliases.json` into the current version directory, adds it to `manifest.json`, and recomputes `content_version` with the compiler's deterministic hash rule. An older library has no file and gets an empty map.
