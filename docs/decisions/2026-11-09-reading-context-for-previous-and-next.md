---
title: "Reading context for previous and next"
date: 2026-11-09
status: current
superseded_by: []
issues: [161]
prs: []
area: ui
---

# Reading context for previous and next, 2026-11-09

Refs [#161](https://github.com/phynics/cast-iron/issues/161). Owner decisions recorded in the
issue on 2026-11-09.

- **The context is a URL parameter.** An entry opened from a book page or a chapter page carries
  `?book=<source>` on its entry route, next to the existing `?a=<anchor>`. It survives reload
  and shows the book the reader came from, not hidden state.
- **Without a context the first active book in book order wins.** That book is the candidate
  whether or not it places the entry; the footer appears only if its Contents lists the entry.
  A search hit or a direct address from another book therefore shows no reading links, while a
  search hit that the first active book does place follows that book.
- **Previous and next follow Contents, not the raw tree.** `views::book::reading_order` flattens
  only the entries Contents shows and `reading_neighbors` takes the neighbours from that list.
  A hidden kind is skipped. At a chapter end the walk continues into the next chapter's first
  entry and the footer names that chapter (`Next in {Book} · {Chapter}: {title}`); the reverse
  crossing names the previous chapter. The first and last visible entries of a book have one
  side only.
- **The book and chapter title stay the reader's.** Titles use `section_label`, so a display
  rename never changes an id or a URL.
