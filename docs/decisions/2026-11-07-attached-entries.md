---
title: "Attached entries"
date: 2026-11-07
status: current
superseded_by: []
issues: [246, 249]
prs: []
area: content
---

# Attached entries, 2026-11-07

Refs [#246](https://github.com/phynics/cast-iron/issues/246), [#249](https://github.com/phynics/cast-iron/issues/249). Model from the attached-entries council; slice 1 ships schema 4 with no visible change.

- **Every add-on is its own entry, gated by its own book alone.** A gated body block was rejected because search and backlinks read the whole body and a section has no id; a reader rule-set switch was rejected because books are the only filter. Stamina follows Unchained, and a collapsed Stamina row is enough. The owner chose no switch.
- **Edges.** `variant` (spell to spell), `addon` (labels `mythic`, `stamina`, `changes`), and `revision` (labels none, `removed`, `merged`) join the existing `prerequisite`, `part`, and `archetype`. `relations.json` keeps one flat edge list and gains an optional fourth label element; an edge without a label keeps the three-element form. The schema bump gates old readers before any edge is parsed.
- **Class options are a kind, not an edge.** `EntryKind::ClassOption` carries `group` from `option-groups.toml` and `replaces`. An `option` edge was rejected because option groups are shared across classes; a kind brings the A to Z list, facets, and the search label. Kind is not a permanent contract: published ids are fixed, and an unknown kind resolves by id.
- **`trades` was dropped.** Class bodies hold no feature headings, so an anchored trade target cannot be validated. `ArchetypeFields { replaces, alters }` and `ClassOptionFields { replaces, alters }` hold the printed items in order and are drift-checked against the `**Replaces:**` and `**Alters:**` lines. `replaces_features` waits for a class-feature-headings pass.
- **Class options gain `alters` before the first schema-4 release.** Eight options print an `**Alters:**` line, so `ClassOptionFields` carries `alters` exactly as `ArchetypeFields` does. The change does not bump `SCHEMA_VERSION`: schema 4 is merged but unreleased, so no reader has seen the earlier shape. An entry may print several `**Replaces:**` or `**Alters:**` lines, one per traded power (oaths and medium spirits do); the typed list is their concatenation in body order, and a comma-separated line contributes one item per part. Single-line entries keep their earlier comparison.
- **A revision keeps the original id.** One revision in force shows the revised text under the original id. Two or more in force mean no substitution: the original shows with a `Revised by` list. Removals and merges are `revision` entries holding the printed sentence.
- **Ids.** The published carried mythic twins stay live and are the mythic entries. New add-ons nest as `ref/<base>/<suffix>`, where the suffix comes from the publication row in `publications.toml`. A `variant` keeps its existing flat id; variants are not suffix-nested.
- **Files.** The compiler derives `option-groups.json` (the declared groups and their `class_option` entries) and `replaces.json` (archetype and class-option id to printed `replaces` items), both sorted. `SCHEMA_VERSION` is 4; `INDEX_FORMAT_VERSION` is unchanged.
- **Owner decisions.** A collapsed Stamina row is enough, with no switch. Druid animal companions (213 entries) are class options. Mythic spells become add-on entries in slice 3.

| Removed | Where it is now |
|---|---|
| Pasted Mythic, Stamina and Elephant text in base bodies | Its own entry, stacked or collapsed on the base page |
| Duplicated cluster text in variant spells | The base, through `Variant of` and a `Variants` row |
| `Rule` label on class options | `{Group singular} · {book}` |

Refs #246, #249.
