---
title: "Phase 5 review follow-up"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Phase 5 review follow-up, 2026-11-04

- **The kind tie-break is the serialized slug, not declaration order.** The merge rule already said "lexicographically smallest kind"; the implementation used `EntryKind`'s derived `Ord`, which follows declaration order (`Spell` before `Feat`). It now compares `kind.slug()`, so `feat` wins over `spell`. The unit test uses that discriminating pair and reverses read order.
- **A star is the set of stored rows that resolve to an id.** Pressed means at least one stored row resolves to the canonical id, not that the exact key exists. Star adds a row only when no row resolves there, and unstar or Remove deletes every resolving row in one transaction. The entry star, the entry bookmark control, Home, and the bookmarks view share `Library::resolves_to`. The bookmarks and Home lists collapse the set to one displayed row per resolved target with the union of collection membership, and an aliased row shows the target's current title. A refused migration therefore cannot make a saved item look absent, resurrect on unstar, or duplicate on star.
- **A refused conversion is disclosed, not just logged.** When IndexedDB refuses the id rewrite, `Library` records the refusal and Settings shows the non-blocking "Saved items will be converted when storage is available." notice. Every read still resolves old ids and the next load retries.
- **A damaged migration marker reads as absent.** An unparseable `meta["id_migration"]` used to fail the migration on every load. It now decodes tolerantly, like the recents read; the idempotent rewrite runs once more and repairs the marker.
- **Catalog membership trusts the sorted catalog.** `catalog_has` no longer falls back to a linear scan after a binary-search miss. A debug assertion states the sorted-unique-id invariant, and a miss is a miss.
- **A retired route shows its reason.** The entry route previously ignored `Resolution::Retired` and fell through to a generic fetch. It now renders the "not in this library" state with the reason category from design 6.2 and never redirects to a guessed article.
- **The alias map is validated for one step.** `AliasIndex::validate` now rejects a target that appears as an old id in either table and an old id that appears in both `entries` and `retired`, matching the rule that its comment already claimed. The client relies on that check before resolving.
- **The cutover fixture must publish a contract-valid catalog.** The fixture's new library listed its three entries in sourcebook order (`ref/rule/direct`, `ref/rule/shared`, `ref/race/trait`), not sorted by id. The client binary-searches the catalog, so both many:1 targets were reported absent: `plan_migration` gates every alias group on `known(target)`, so the shared and derived rows were never grouped, never merged, and never deleted. The 1:1 row migrated only because its id happened to sort into a reachable slot, which made the bug look like a many:1 defect. The fixture now sorts its catalog entries, as the compiler does.
- **An unsorted catalog is corrupt, not a lookup miss.** `Catalog::validate` checks that entries are strictly ascending by id, and `Library::load` rejects a violation as `LibraryError::Corrupt`. A miss stays a miss (no O(n) scan), but a catalog that breaks the invariant now fails loudly instead of silently mis-resolving every id after the first out-of-order row.
- **Search p95 is recorded, not a correctness gate.** A shared CI runner produced a 102 ms p95 for WebKit against a 4.8 ms desktop figure, so a hard 100 ms assertion failed on load, not on engine speed. The test now annotates the measured p95 and max every run and asserts only a 250 ms smoke bound, which still catches an index that failed to load or an engine regression.
