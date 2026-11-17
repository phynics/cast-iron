---
title: "Continue reading per book"
date: 2026-11-09
status: current
superseded_by: []
issues: [161, 162, 346]
prs: []
area: ui
---

# Continue reading per book, 2026-11-09

Closes [#162](https://github.com/phynics/cast-iron/issues/162). Depends on the `?book=<source>` context from [#161](https://github.com/phynics/cast-iron/issues/161) (#346).

- **One point per book, in `meta` under `continue_points`.** Each record is `{ source, entry_id, read_at }`; the list is newest first, and re-reading a book replaces its row instead of adding another. The data is local only and no network request is made. `DB_VERSION` stays 1 because the key lives in the existing `meta` store; no serialized content or search format changed.
- **Only a visit that carries the book counts.** `views/entry::EntryView` records the point only when its route has `?book=<source>`, which is what a Contents row, a chapter page and a previous/next link produce. A search hit, a Home row without context or a direct entry URL records the recent visit but leaves every continue point alone. The write runs in one IndexedDB transaction, so two quick visits cannot drop each other's point.
- **Home shows the most recently read book only.** One `Continue {Book}` row sits first in `Recently viewed`, and the rest of that section is unchanged. Its link keeps `?book=`, so opening it stays inside the book and moves the point forward. The latest book is chosen by `read_at`, with the source name as a stable tie-break (`store::latest_continue_point`), not by the stored order.
- **The book page shows its own point near the top.** A saved place in that book renders `Continue reading: {entry title}` as a link under the meta line, carrying `?book=` again. Re-reading a different book leaves this one in place, because every book keeps its own point.
- **A stale id resolves or disappears.** On read, `store::resolve_continue_points` maps an old id through the alias map and drops a point whose entry is gone or retired; a point whose source the library no longer installs is hidden too. Resolution happens at display time, so a library cutover never shows a dead link. The next visit writes the canonical id.
- **Clear clears both.** Home's `Clear` empties `recent` and removes `continue_points`, so "Clear history" also forgets where the reader stopped.
