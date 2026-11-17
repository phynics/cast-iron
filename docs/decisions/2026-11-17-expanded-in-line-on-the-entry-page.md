---
title: "The Expanded in credit line on the entry page"
date: 2026-11-17
status: current
superseded_by: []
issues: [457, 460]
prs: []
area: ui
---

# The `Expanded in` credit line on the entry page, 2026-11-17

Closes [#460](https://github.com/phynics/cast-iron/issues/460). Part of [#457](https://github.com/phynics/cast-iron/issues/457). It shows the data from [expanded_in credits a book outside membership](2026-11-17-expanded-in-credits-a-book-outside-membership.md).

- **Where.** One muted line, `Expanded in: Monster Summoner's Handbook p. 18`, directly under the entry's `Sources:` line and above the Archives of Nethys link. It prints only when the entry has `expanded_in` rows. Several credits share the line, joined by `; `, in corpus order.
- **Links.** The book links to its book page, like the `Sources:` line. A `via` entry links the page number, and only while that entry is visible under the reader's active books. A hidden or unknown `via` entry leaves the number as plain text, so the line never links to an entry the reader cannot open.
- **Inactive book.** A credit from a turned-off book stays on the page, dimmed and followed by the `Inactive` Label. Dimming alone is not a cue, and the opacity (`.85`) keeps the text above 4.5:1 in both themes. The book link stays, because the book page opens for an inactive book.
- **Never membership.** The line is a display of credit. It does not change the metadata line, counts, filters, search or page order, as the field's own decision says.
- **Revisions.** `effective_entry` used to copy the base's credits. A revision replaces citations because a citation belongs to one printing's text. A credit says that a book expands the rule, so it does not follow the text. The page shows the base's credits, then any credit that the revision adds, each once. A revision with none keeps every base credit. This is `revisions::effective_expanded_in`.
- **Testing.** No pinned corpus has `expanded_in` yet (#459 adds it). The browser spec builds an overlay of the served library that adds credits to chosen entries and rewrites only their shards and the manifest hashes (`buildLibraryWithExpandedIn`). Native tests cover the row logic and the revision rule.
