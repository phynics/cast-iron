---
title: "Canonical library compile (P7)"
date: 2026-11-05
kind: record
---

# Canonical library compile (P7), 2026-11-05

`cargo xtask content --canonical` now builds the complete canonical library into
`generated/canonical/` through the same `pipeline::package` tail as the PSRD
build. The measured local build on 2026-11-05, against PSRD pin
`76ea4784604145ba3dea40eb56f83bb904759467` and Pf Data pin
`08ea71f24375908d0190c822fd1fd080797667a9`, is:

| Measure | Value |
|---|---:|
| Content version | `vf49db728890fffb9` |
| Schema / search format | 3 / 1 |
| Entries | 28,777 |
| Total library bytes | 197,486,346 |
| `search.bin` | 62,295,124 |
| Shards | 235 |
| Largest shard | `entries/item-016.json`, 524,282 bytes |
| Books | 436 |
| Publications (with entries) | 456 (394) |
| Relation edges | 2,142 (`part` 342, `archetype` 332, `prerequisite` 1,468) |
| Book placements / unpaged | 22,997 / 5,780 |
| Alias rows / retired | 9,794 / 174 |
| Section 15 notices | 660 |
| Dataset pins | `psrd-data`, `pathfinder-data-1-e` |

Entries by kind: 8,006 items, 7,370 rules, 3,368 feats, 3,360 creatures,
3,099 spells, 1,631 traits, 942 archetypes, 342 afflictions, 342 racial traits,
111 prestige classes, 103 races, 58 traps, 26 skills, and 19 classes. The
library carries 2,216 PSRD carry-over entries.

The legal screen rejects 1,292 records whose publications have no Section 15
notice in either the PSRD or Pf Data list; 5,075 references to those records are
flattened or dropped and counted under `legal-reference-settlement`. The alias
file retires 174 rows whose planned canonical target the converter did not emit
and none for rights. The PI screen records 1,301 omissions across the in-scope
inventory. Coverage rows also carry 17,863 converter/out-of-scope/PI/carry-over
rows and 3,400 rejected records.

Two consecutive builds produced byte-identical trees, including `manifest.json`,
`aliases.json`, and every shard. `python3 scripts/check-publication.py
generated/canonical` passes with 0 issues. `cargo xtask content` (default) still
produces `vdb43853600b8ed8b` with 9,968 entries, byte-identical to
`origin/main`. `cargo xtask check` passes.

`python3 scripts/check-library-upgrade.py generated/content generated/canonical`
exits nonzero because the script is not alias-aware and expects one global book
placement: it reports all 9,968 PSRD ids as missing (the canonical library uses
`ref/` ids and ships the map in `aliases.json`), and 17,042 `books.json`
bookkeeping entries as "placed twice" or "neither placed nor reported". The
latter is the script's single global `placed` set: a canonical entry may appear
in several books, and an entry without a page is both listed under Unpaged and
recorded in `book_order.unplaced`. Schema validation reports no dangling link,
parent, relation, or book target. The script was not weakened.

This is a local build, not a deployment. PSRD was still the production default at
that time; the later default switch is recorded at the top of this page. P8
acceptance on the later facet-carrying build (`v523b94bd49b15699`) is recorded
under [Canonical library acceptance (P8)](#canonical-library-acceptance-p8-2026-11-05).
