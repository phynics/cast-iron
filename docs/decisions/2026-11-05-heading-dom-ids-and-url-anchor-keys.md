---
title: "Heading DOM ids and URL anchor keys"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Heading DOM ids and URL anchor keys, 2026-11-05

This replaces the earlier rule that heading ids were the compiler's anchors, unprefixed. That rule let a content anchor such as `main` or `references-heading` collide with an app element, and let two headings with the same text share one id.

- **A heading's KEY is its URL anchor.** It is the compiler anchor when that is non-empty, otherwise `sec-` plus the slug of the heading's plain text. If neither produces text, the heading has no KEY and no id. Existing `?a=` URLs therefore keep working.
- **Duplicate KEYs in one entry are suffixed deterministically.** The first heading in document order keeps the base KEY. Later headings with that KEY get `KEY-2`, `KEY-3`, and so on. A suffix that matches any literal KEY elsewhere in the same body is skipped, so `sec-x`, `sec-x`, and a real anchor `sec-x-2` become `sec-x`, `sec-x-3`, `sec-x-2`.
- **The DOM id is `h-` plus the final KEY.** The prefix keeps content ids away from app ids such as `main` and `references-heading`.
- **`scroll_to_anchor` tries `h-<anchor>` first, then the raw id.** The fallback keeps routes that target app elements working, including `entry-related` and the browse letter groups.
- **App anchors are reserved.** `rich::RESERVED_ANCHOR_KEYS` lists the route anchors that target app elements on an entry page (`entry-related`). `heading_keys` treats them as already taken, so a content heading whose KEY would equal one gets the next free suffix (`entry-related-2`) and `?a=entry-related` can only reach the Related panel. The 2026-11-05 scan of the 29,515-entry canonical library found no such heading, so no existing URL key changed. A new route anchor for an app element must be added to that list.
- **The renderer and article contents share one assignment.** `rich::heading_keys` walks the same document order as the renderer, and `ArticleContents` links to the final KEY. Headings with a KEY are no longer dropped from the contents just because another heading shares their text.
