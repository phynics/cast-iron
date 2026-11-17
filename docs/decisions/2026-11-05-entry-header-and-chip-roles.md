---
title: "Entry header and chip roles"
date: 2026-11-05
status: current
superseded_by: []
issues: [99]
prs: []
area: ui
---

# Entry header and chip roles, 2026-11-05

Refs [#99](https://github.com/phynics/cast-iron/issues/99). One pill shape stood for eight meanings, and the entry header spent about 150 px on chips and a Star button at 390 px.

- **Four roles, each with one form** (`DESIGN.md`): metadata is a text line, filters are the only pills, navigation is a 6 px rectangle, labels carry no border. The old classes `pill`, `pill-list` and `chip-link` are gone; navigation uses `jump` and `jump-list`, metadata uses `meta-link`.
- **The entry header shows the first book in `source_ids`.** The metadata line shows the kind and that book; the other books open from `+N more`. All books stay reachable as links. The header does not reorder them: the compiler owns the primary order (#95).
- **The star is an icon-only toggle.** Its name still reads `Star {title}` or `Unstar {title}` (it already changed with state, and the e2e suite relies on it), now next to `aria-pressed`. Home's Unstar uses the same icon button.
- **"Related entries" jump link removed.** The relationship panels follow the body, and the Contents disclosure already covers navigation. The `entry-related` anchor stays for in-page links.
- **The spell school facet is the school word alone (#113).** The Pf Data school string keeps parenthetical subschools and a bracketed note (`conjuration (summoning) [see text]`), and the facet used to carry the parenthetical, so each subschool spell fell out of its school's count. The compiler now moves the words of every top-level parenthetical (split on commas, `or`, `and`) into `SpellFields.subschools` in Title Case, the form the directive subschools use, deduped without case against the existing list. The stored `school` keeps only the word and its note (`conjuration [see text]`), so the statistics panel shows the subschool once, on its Subschool row. Nested or unbalanced parentheses are left untouched. The facet value is the text before the first `[` or `(`, lowercased, in both the Pf Data and the PSRD paths. A parenthetical that only says `see text` is a pointer, not a subschool: it stays in the stored school (`multiple (see text)`, `conjuration (see text)`) and adds no Subschool row. `multiple (see text)` forms its own deliberate `multiple` facet bucket, so facet counts still add up to the spell total and the one spell with several schools stays findable. Subschool has no facet: the PSRD path emits none and the client's facet groups per kind are fixed. Changed facet routes: values such as `school:conjuration (summoning)` no longer exist; the router maps a `school` facet value in an old link to the school word, so those links land on `conjuration`, `enchantment` or `multiple`. `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay unchanged: the field shapes and the index format are the same, only values change. The client label keeps `or`, `and`, `of`, `the` lowercase except as the first word.
