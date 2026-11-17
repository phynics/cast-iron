---
title: "Book header counts"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Book header counts, 2026-11-05

- **The book page counts its contents.** `views::book::book_entry_count` counts distinct entry ids across the whole `books.json` contents tree, including every placement and regardless of active state or canonical source. The header says "N entries in this book" from that number and does not change when the book is disabled, because the contents stay visible.
- **The legacy source page counts what its groups list.** `views::util::active_count_line` says "N entries in your active books" from `Discovery::entries_of_source`, the same active-book-filtered set its kind groups render. A disabled book shows "0 entries in your active books" and no groups. The line keeps the existing muted paragraph in both views.
