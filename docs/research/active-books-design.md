# Active-book discovery boundary

Chosen 2026-11-03 after two independent design passes. This is an implementation contract, not a shipped-behavior report. Requirements are UI-03 in `docs/ui-tasks.md`.

## Caller usage

```rust
let discovery = library.discovery();
let counts = discovery.kind_counts();
let hits = discovery.search(&query).await?;
let lists = discovery.spell_lists().await?;
let relations = discovery.relations().await?;
let entry = library.entry(&id).await?; // canonical reading remains available
```

Books uses canonical publication rows plus active-state controls. Home stars and direct entry routes use canonical reads, with a disabled-book notice where applicable. Search, category browsing, relation discovery, spell lists, and feat chains use the scoped view.

## Ownership and invariants

- `store.rs` persists a set of disabled canonical book IDs in metadata. Missing preferences mean all active. Unknown IDs survive replacement updates, and new books default active. Zero active installed books is valid.
- `Library::load` resolves preferences and book placement before returning a ready library. A read failure is reported instead of exposing an unfiltered library temporarily.
- A lightweight discovery scope hides eligibility policy and projects counts, graph indexes, and search candidates. It does not duplicate the canonical catalog or rebuild the search index.
- Book placement takes precedence over the attribution source. Older libraries without `books.json` use a documented catalog-source fallback. Missing placement in a newer index is handled explicitly and tested.
- Search and facet eligibility applies before aggregation or result limits. Both empty-query and ranked search follow the same rule.
- Preference writes serialize. Successful persistence precedes publishing the new reactive state. Failed writes retain the previous state and show recovery text.
- Discovery screens recompute when selection changes. Async results are guarded against stale selection as well as stale queries.
- Disabled graph nodes are not shown or counted. Pruning must not invent prerequisite edges between remaining nodes.
- Disabling does not delete content, stars, or reading history. Explicit reading links remain usable.

## Synthesis decision

Both passes compared view-level predicates with a shared scoped library projection. Choose the projection because filtering order and aggregation are easy to get wrong at individual call sites. Keep the policy small and specific to discovery. Retain canonical APIs for reading, publication controls, stars, and legacy-schema recovery.

One pass proposed storing active IDs. Prefer the disabled-ID representation because an absent new book is naturally active after an update, and unknown disabled IDs can survive without reconciliation. This representation also distinguishes zero active installed books without a separate sentinel.

## Verification sequence

1. Test preference defaults, persistence, placement, older-schema fallback, zero active, and update behavior.
2. Test eligibility before search limits and scoped facets, graph structures, and counts.
3. Wire controls and all discovery callers. Test direct reading and stars separately from discovery.
4. Build `dist/` and exercise toggles, offline restart, updates, empty state, and migration from collections. Run `cargo xtask check` and affected browser acceptance tests.
