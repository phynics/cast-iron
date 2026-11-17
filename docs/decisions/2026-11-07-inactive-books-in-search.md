---
title: "Inactive books in search"
date: 2026-11-07
status: partly-superseded
superseded_by: ["2026-11-17-book-membership-is-derived-from-source-ids"]
issues: [148, 149]
prs: [221]
area: search
---

# Inactive books in search, 2026-11-07

Refs [#148](https://github.com/phynics/cast-iron/issues/148), [#149](https://github.com/phynics/cast-iron/issues/149), [#221](https://github.com/phynics/cast-iron/pull/221).

- **The search row and the entry header use one preferred-source rule over different data.** Both call `preferred_source`: the entry header offers every `source_ids` entry because it has the full entry, while a search row offers only the canonical book placement because the catalog stub carries one source. A search row therefore cannot name a third active book that only the full entry knows. Making it do so would need the alternatives in the index or catalog, a format change.
