---
title: "Feat fields describe the normal feat"
date: 2026-11-06
status: current
superseded_by: []
issues: [180]
prs: []
area: corpus
---

# Feat fields describe the normal feat, 2026-11-06

Refs [#180](https://github.com/phynics/cast-iron/issues/180). The checker is `crates/corpus/src/drift.rs`.

- **A base feat's `[fields]` are compared only with the body before its first source subsection.** A source subsection is a level-3 heading followed by its own `**Sources:**` line: a `### Mythic …` block, `### Stamina`, an Elephant-in-the-Room replacement, or any other book's version of the same feat. The fields describe the normal feat, so a field that matches only the subsection's `**Benefit:**` or `**Prerequisites:**` line is drift. A `###` heading that organizes the feat's own text and carries no `**Sources:**` line stays in the comparison, so its `**Benefit:**` line still counts as a variant statement of the normal feat.
- **The multiple-statement rule stays inside the normal part.** When the normal part states a label more than once (a variant statement), the field agrees with any of those statements. Subsections are never candidates.
- **Footnote markers do not decide a comparison.** The field writes `[^P]` or `[^X]`, the body carries `^1^` or the flattened `ranks.1`, and neither is rule text. The checker drops the markers and the footnote superscripts before it compares.
- **Measurement on corpus `v0.1.0`.** The previous checker reported 4 disagreements, all footnote-only (`conceal-scent`, `heightened-trap-sense`, `improved-day-job`, `religious-pilgrim`). The stricter checker reports 272 disagreements across 160 feats: 198 in base fields that hold a mythic subsection's wording, 74 that hold the Elephant-in-the-Room tail's wording, and none that hold plain Stamina wording; by field, 140 `benefit`, 120 `prerequisites`, 10 `special`, and 2 `normal`. A field is attributed to the source block that actually states the matched text, not to the nearest `###` heading: 44 of the Elephant-in-the-Room rows sit under a `### Mythic …` heading whose subsection ends with a bare `**Sources:** The Elephant in the Room…` line. The companion change in `phynics/cast-iron-data` (#3) corrects every one and brings the report to 0.
- **A field with no statement in the normal part is drift.** Some fields exist only because another block states them. `text_pair` returned nothing when the normal part had no `**Label:**` line, so nothing compared them and no warning appeared. When the label is stated elsewhere in the body (a source subsection), the field now counts as drift too, with a message that names the missing label. That is an error, like every other drift. A body that never uses the bold label — the carried heading-and-statline entries — stays outside this check, as it was before.
- **A statement inside a quote counts.** A sidebar or variant list may carry a `**Normal:**` or `**Special:**` line inside a blockquote; it is still a statement of the normal feat, so the checker reads quoted paragraphs too.
