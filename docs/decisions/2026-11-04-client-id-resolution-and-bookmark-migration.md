---
title: "Client id resolution and bookmark migration"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: content
---

# Client id resolution and bookmark migration, 2026-11-04

- **Resolution is one step and catalog-first.** `Library::resolve` checks the active catalog, then one `aliases.json` entry, then the retirement list. It never follows a chain. A library that still publishes a legacy id therefore keeps opening it even if an alias map is present, which makes the alias-capable shell safe before the canonical library ships. `retired` ids report their reason; the stored row keeps its snapshot and is never redirected to a guessed article.
- **The migration marker is the alias-file hash.** `meta["id_migration"].aliases_sha256` records the manifest hash of `aliases.json`, not the content version. The map is append-only, so a new alias can make a previously retired row resolvable; a content-only change must not force a rewrite.
- **The rewrite is one transaction and is idempotent.** Bookmark rows that resolve to one target merge: `added_at` is the minimum, `collection_ids` is the sorted union, `kind` is the target catalog kind when present, otherwise the earliest row's kind with a lexicographic tie-break, and `title_snapshot` is the earliest row's title with a lexicographic tie-break. An existing canonical row at the target joins the group. Recent ids use the same resolution and dedupe, newest first. Retired and unknown rows are untouched.
- **A refused transaction is not fatal.** The library still loads; reads keep resolving old ids, and the next load retries. This differs from `disabled_books`, whose read failure blocks because there is no correct fallback.
- **Verification is fixture-based until the cutover ships.** The real browser test builds a small legacy library and a small canonical library with an alias map, stars a many:1 pair and a retired id, installs the update, and checks merge, marker, second-load idempotence, old-URL replacement, and offline use. The production library still publishes PSRD ids, so no release note may claim the cutover is live.
