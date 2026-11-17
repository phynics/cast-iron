---
title: "Collections code deleted"
date: 2026-11-16
status: current
superseded_by: []
issues: [400]
prs: []
area: ui
---

# Collections code deleted, 2026-11-16

Refs [#400](https://github.com/phynics/cast-iron/issues/400). This carries out the collections bullet of "Owner decisions from the architecture review, 2026-11-16".

- **Deleted.** The unmounted bookmarks and collections screen, the entry-page collection picker, the collection functions of `store.rs`, the unused star import (`merge_bookmarks`), the unused single-row bookmark functions, `random_uuid`, the `Route::Bookmarks` and `Route::Collection` variants, and the CSS that only they used. Stars on Home and on the entry page are unchanged.
- **Old addresses.** `#/bookmarks` and `#/collection/<id>` still parse to `Route::Home`, and the legacy redirect in `read_location_route` still rewrites them.
- **Stored data stays.** The IndexedDB `collections` store is still created (the version stays 1; no migration) and is never read or written. `Bookmark` no longer has `collection_ids`: an old row still reads (serde ignores the field), and the next write of that row (a kind refresh or an id migration) drops it. The star itself is kept.
