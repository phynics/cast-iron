---
title: "arm or wrist is an allowed item slot value, by name"
date: 2026-11-23
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# arm or wrist is an allowed item slot value, by name, 2026-11-23

Refs phynics/cast-iron-data#171. The owner ruled on 2026-11-23.

- **The value is accepted as printed.** Kerchief of Remembrance (Heroes of the High Court p. 13) prints the item slot `arm or wrist`. The pair rule of the closed vocabulary cannot produce it: `arm` is a tech slot, which never joins a pair, and `wrist` is not the body slot `wrists`. The value therefore sits in its own closed list, `PRINTED_SLOT_VALUES`, which starts with this one value and gets no other by pattern.
- **The list stays closed.** A printed value joins it only with a print citation and an owner decision. The pair rule does not change, and `arm or wrists`, `wrist`, and `arm or neck` stay rejected.
- This replaces the sentence of [the closed vocabularies decision](2026-11-22-affliction-types-and-item-slots-are-closed-vocabularies.md) that kept `arm or wrist` open.
