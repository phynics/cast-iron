---
title: "Third-party revisions replace their original in place"
date: 2026-11-09
status: current
superseded_by: []
issues: [192, 246]
prs: []
area: content
---

# Third-party revisions replace their original in place, 2026-11-09

Refs [#192](https://github.com/phynics/cast-iron/issues/192), [#246](https://github.com/phynics/cast-iron/issues/246). The app half of slice 7; corpus v0.1.14 holds 76 `revision` entries in The Elephant in the Room.

- **One active revision is in force.** The original id keeps its route, star, Recent entry and list row. Its page shows the revision's fields and body, with a `.notice` `Revised by {book}. Changed: {fields}.` and a collapsed `Original text · {book}` disclosure holding the original body. The effective entry keeps the original id and metadata, takes the revision's fields, body, links and prerequisite edges, and drops the revision edge so the page never names itself as a revision.
- **Removed and merged.** A `removed` revision shows `Removed by {book}.` followed by the printed sentence; a `merged` revision shows `Merged by {book} into {feat}.` with a typed link to the feat named in the revision body. Both keep the original text in the same disclosure and leave the feat list, but search still finds the original.
- **No revision row.** A revision hides from a feat list when its base is visible: the base row already carries the revised text. When the base's book is inactive the revision stays listed, because it is the only row. A book's own contents stay complete. Home category counts subtract the hidden rows, so a revision never inflates the Feat count.
- **Search reads the revision, not the pair.** A revision hit is rewritten to the original id with the revision's book and snippet, so a query returns one row. The original stays findable when the revision removed or merged it. Quick search applies the same rule.
- **Two or more revisions in force mean no substitution.** The original fields stay, and the page shows `Revised by` with one `.rel-list` row per revision. Never last-writer-wins.
- **A direct revision link opens the revision's own page.** Its metadata line ends with `Revision of {base}`, linking the base and marking it `Inactive` when the base's book is off. This follows DESIGN.md's add-on page rule and keeps the printed revision text available without a redirect.
- **Chains use the revision's prerequisites everywhere.** One effective graph, derived once per active-book set and cached, replaces every revised base's outgoing prerequisite edges with the revision's. A chain head that the revision removed leaves the graph; a merged base's dependents point at the feat that absorbed it. The chain list, list-page markers, entry panels and search all read this graph. Search facet and chip counts use the same effective rows: a base and its revision count once, and a removed or merged feat counts not at all while its book is active.

Refs #192, #246.
