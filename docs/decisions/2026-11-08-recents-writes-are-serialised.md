---
title: "Recents writes are serialised"
date: 2026-11-08
status: current
superseded_by: []
issues: [273]
prs: [269]
area: install-offline
---

# Recents writes are serialised, 2026-11-08

Refs [#273](https://github.com/phynics/cast-iron/issues/273), [#269](https://github.com/phynics/cast-iron/pull/269).

- **The store write takes the signal, not a fresh read.** `recent::record` used to read the
  stored list, modify it, and write it back. Two visits in quick succession each read the same
  old list, and the last write dropped the other visit from storage; a clear could likewise be
  undone by a record that started before it. `record` and `clear` now update the shared signal
  first and write that list. The signal already merges visits in memory, so the last write is
  the newest state.
- **At most one write runs.** A small in-flight queue in `recent.rs` marks a running write stale
  when a visit or clear lands during it, and the writer then writes the signal again. A single
  visit still causes one write; a burst coalesces into the next one instead of racing. The queue
  is the durable-copy rule, not a second source of truth.
- **The startup read joins the queue.** `load_into` merges the stored history into the signal.
  It marks a running write stale so that writer takes the merged list, and when the write
  already finished it writes the merged list itself. Either way a write cannot leave the store
  without that history. A clear wins over a read that was already in flight: the clear bumps a
  generation, and `load_into` discards an answer whose read began before that generation, so a
  cleared history is never merged back. This replaces the old read-modify-write guarantee on a
  deep link or reload on an entry URL.
- **Every Home reload in the e2e suite waits for the index read.** `seedHome` now ends with
  `waitHomeIdle`, and every flow that navigates to Home and reloads at once waits before the
  reload: three in `search.spec.ts`, two in `update.spec.ts`, and one each in `shell-update`,
  `offline`, and `mythic`. Leaving Home while the search-index `Cache` read is in flight is the
  uncatchable WebKit `Context is stopped` above.
