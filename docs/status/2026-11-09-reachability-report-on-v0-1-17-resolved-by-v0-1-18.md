---
title: "Reachability report on v0.1.17, resolved by v0.1.18"
date: 2026-11-09
kind: record
---

# Reachability report on v0.1.17, resolved by v0.1.18, 2026-11-09

`cargo xtask content` writes `generated/reports/reachability.json` for every entry that the
catch-all chapter `New entries (page order)` is the only route to. On corpus `v0.1.17`, 21
distinct entries of 31,807 were unreachable, all rules. The per-book rows added to 23 catch-all
placements because `ref/rule/traits-and-other-details` sits in three catch-alls: Bestiary 4 13,
Advanced Race Guide 2, Bestiary 2 2, Mythic Adventures 2, and one each in the Advanced Player's
Guide, Core Rulebook, Ultimate Campaign, and Ultimate Combat. Every entry is in the search index
under its exact title (31,807 of 31,807), and the report is byte-identical between two compiles.

Corpus `v0.1.18` places or links those rules (data #63), so the report now shows 0 unreachable
entries of 31,807, and [#363](https://github.com/phynics/cast-iron/issues/363) turns the report
into a compile error. [#360](https://github.com/phynics/cast-iron/issues/360) added the report.
