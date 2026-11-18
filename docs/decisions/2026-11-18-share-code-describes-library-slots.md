---
title: "A share code describes the library's slots"
date: 2026-11-18
status: current
superseded_by: []
issues: [416]
prs: []
area: ui
---

# A share code describes the library's slots, 2026-11-18

"Book share codes, 2026-11-16" says the slot file is append-only. A pull request can append a line before the pinned corpus has the book (#489, #492). The code then describes the slots up to the highest `share_slot` in the library's catalog, not the length of `sources/published-books.txt`. The slots between the two are unknown to the library, and a receiver skips them as it skips any slot it lacks. `books-share.spec.ts` compares `described` with the highest catalog slot plus one.
