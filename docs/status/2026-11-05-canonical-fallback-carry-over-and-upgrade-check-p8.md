---
title: "Canonical fallback carry-over and upgrade check (P8)"
date: 2026-11-05
kind: record
---

# Canonical fallback carry-over and upgrade check (P8), 2026-11-05

The canonical build now carries a legacy entry when its planned canonical target
was not emitted, instead of retiring the alias row. On the pinned corpus 29 rows
take that fallback (elementals, ettercap, night hag, several traits and
archetypes, Magic, hazards, mythic feats/spells, special materials, spellbooks),
so `aliases.json` has 9,968 rows and zero retirements. The fallback count is one
coverage row, `alias-fallback-carry-over`; the carry-over report moves from
2,216 to 2,245 entries and 434 to 454 possible-duplicate review leads.

| Measure | Value |
|---|---:|
| Content version | `v3c4d6a1ad9e6838b` |
| Schema / search format | 3 / 1 |
| Entries | 29,417 |
| Total library bytes | 206,052,099 |
| `search.bin` | 64,746,739 |
| Shards / largest shard | 250 / `entries/creature-018.json`, 524,284 bytes |
| Books / publications (with entries) | 436 / 456 (394) |
| Relation edges | 4,170 (`part` 342, `archetype` 1,200, `prerequisite` 2,628) |
| Book placements / unpaged | 23,546 / 5,871 |
| Alias rows / retired | 9,968 / 0 |
| PSRD carry-over entries | 2,245 (29 from the fallback) |

Entries by kind: 8,011 items, 7,583 rules, 3,444 feats, 3,370 creatures,
3,099 spells, 1,639 traits, 1,240 archetypes, 342 afflictions, 342 racial
traits, 111 prestige classes, 103 races, 58 traps, 49 classes, and 26 skills.

`python3 scripts/check-library-upgrade.py generated/content generated/canonical`
now exits 0. It resolves all 9,968 PSRD ids in one step: 5,710 direct, 1,898
shared, 115 derived, and 2,245 carried; no id is missing, unresolvable, or
retired, no title differs under the alias fit gate, no class category is wrong,
and no relation, list, or book target dangles. The 11,937 repeated book
placements are the canonical multi-publication rule and are reported as a count.
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/scripts -v`
covers the alias and legacy paths.

This is a local build, not a deployment. PSRD was still the production default at
that time, and P8 acceptance (install size, quota, latency, offline navigation,
bookmark migration, browsers) had not been run for that library.
