---
title: "Pf Data canonical relations"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: content
---

# Pf Data canonical relations, 2026-11-05

- **The builder is source-independent.** `relations::apply_canonical(entries, nested)` reads only canonical `RuleEntry` values and the `NestedLink`s the converter synthesized. It shares the prerequisite reader and the index writer with the PSRD path; it does not read `build::Plan`, `TypedInput`, or a `pfsrd://` url. `data-report` and `data-import` now apply it before validation, search, and serialization, and the migration report carries the resulting `RelationCoverage`. The relation index is validated against the per-entry relations with `validate_derived`.
- **A parent comes from structure the converter already recorded.** A racial trait uses its `parent` race, or the race that nests it. An archetype whose id family is `ref/arc-<class>/...` names `ref/class/<class>` exactly; a prestige-class archetype uses the class linked by its own `This is an archetype of ...` statement. Nothing resolves a parent by title similarity. A parent that is absent from the entry set, or a trait with no race, becomes an `unplaced_parts` row with the exact missing id.
- **Encoded prerequisite links are read as their visible text.** Pf Data stores prerequisites as `‹feat/Iron Will›`, not as plain names. The reader replaces each encoded link with the text the renderer shows, then reuses `prereq::parse` and `FeatIndex`. The stored `FeatFields::prerequisites` text is unchanged. A name that resolves to a target already present is written once; a target that is both required and named again as an alternative keeps the unlabelled edge, because the schema allows one relation per kind and target.
- **Full-corpus counts (pinned `08ea71f`, 27,853 entries).** 1,853 edges: `part` 67, `archetype` 343, `prerequisite` 1,443. 639 archetypes have no parent. Of 8,297 prerequisite requirements, 1,542 read as feat names and 1,023 feats resolve at least one; 2,917 remain unread and are listed in coverage. The PSRD baseline is 1,531 edges: `part` 342, `archetype` 218, `prerequisite` 971, with no unplaced parts.
- **The differences are conversion scope, not relation logic.** Canonical conversion currently derives the 67 standard traits of the seven Core Rulebook races; it does not derive the Pf alternate traits that make up most of PSRD's 342 `part` edges. Pf Data has 982 archetype records, but 30 class records still fail conversion on unsupported directives and `companion`/`familiar` have no class entry, so 639 archetypes cannot be placed; PSRD held 221 typed archetypes and promoted 218. Pf Data also has 3,342 feats against PSRD's 1,099, which yields more prerequisite edges and more unread names, including base feats such as Power Attack, Cleave, and Dodge that the converter currently excludes. These gaps belong to the converter and its corpus coverage, not to the relation builder.
- **Production is unchanged.** The PSRD path still calls `relations::apply` with the same inputs and writes the same relation index (`part` 342, `archetype` 218, `prerequisite` 971). `cargo xtask content` still produces `content_version` `vdb43853600b8ed8b` with 9,968 entries, byte-identical to `origin/main`.
