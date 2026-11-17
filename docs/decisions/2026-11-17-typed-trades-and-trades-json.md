---
title: "Typed trades, corpus format 2 and trades.json (schema 6)"
date: 2026-11-17
status: current
superseded_by: []
issues: [251, 441, 450]
prs: []
area: content
---

# Typed trades, corpus format 2 and trades.json (schema 6), 2026-11-17

Refs [#450](https://github.com/phynics/cast-iron/issues/450), part of [#251](https://github.com/phynics/cast-iron/issues/251). The release checklist is [#441](https://github.com/phynics/cast-iron/issues/441).

- **Model.** `Trade { op: Replaces | Alters, printed, feature: Option<FeatureRef> }` and `FeatureRef { anchor, class: Option<EntryId>, levels, power }`. `printed` is always kept. `feature = None` is legal and means unresolved: a compatibility filter must never treat such a trade as compatible. `ArchetypeFields` and `ClassOptionFields` hold `trades` in place of `replaces`, `alters`, `replaces_features` and `alters_features`.
- **Corpus format 2 is flat.** One `trades = [ { replaces = "…", feature = "…" }, … ]` list, with optional `class`, `levels` and `power` on a trade that names a feature. The tool reads format 1 and 2 and `CORPUS_FORMAT` is 2. The pinned corpus stays format 1: #451 writes the migration and #452 applies it, so a data repository pull request between now and then still passes `fmt --check`.
- **Format 1 lowers lossily, but writes back unchanged.** An anchor list resolves its trades only when it has the same length as its printed list (position to position). Any other non-empty list cannot say which name each anchor belongs to (the old rule omitted names that no heading matched), so its trades stay unresolved. The reader keeps those anchors in a `#[serde(skip)]` field, `legacy_features`, that the format 1 writer puts back; the compiler drops it. That is what keeps `write(read(file)) == file`, which the corpus repository's `fmt --check` needs. The field goes away with format 1.
- **`class` only where the anchor is ambiguous.** The compiler computes the base classes of the entry (an archetype's `archetype` relation targets, or a class option's group classes) and the heading anchors of each. An anchor that one class holds keeps no `class`, written or not. An anchor that several hold needs a written `class`, or the compile fails and names the entry, the trade and the classes. An anchor that no base class holds, and a `class` that is not a base class, also fail. Format 1 has no way to write a class, so there an ambiguous anchor leaves the trade unresolved instead of failing.
- **`trades.json` replaces `replaces.json`.** One `{ id, trades }` row per entry that has trades, sorted by id. The entry shards hold none. It keeps `alters` and the anchors, which the old file lacked, so the compatibility filter of #170 can read it. An entry without trades has no row, as before.
- **Schema 6, and the older readers.** `SCHEMA_VERSION` rises from 5 to 6. `READABLE_SCHEMAS` is now 1 to 6, so an installed schema-4 or schema-5 library still validates and opens (rule 7). For such a library the client reads `replaces.json` and turns each item into an unresolved `Replaces` trade (`TradeIndex::from_replaces`): the Replaces line works, `alters` is not shown, and there is nothing for a filter to compare. A schema-5 shard names the old lists; the new reader ignores them.
- **Checks.** `corpus-check` fails on a `feature` that is not a base-class anchor and on a `class` that is not a base class or lacks the anchor. It warns when a printed name resolves confidently to another anchor than the trade's. It fails on a format 1 corpus whose trade needs `class`, `levels` or `power`. The drift check compares the printed `Replaces:` and `Alters:` lines with the trades of each key.
- **Format 5 checklist.** This is a breaking schema change (4 to 5 to 6 ship in the one format 5 promotion). The checklist in #441 gains: a schema-5 to 6 upgrade check (the old library keeps its Replaces lines through `replaces.json` until the new one is verified).
