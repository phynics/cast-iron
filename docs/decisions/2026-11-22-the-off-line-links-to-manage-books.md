---
title: "The off line links to Manage books"
date: 2026-11-22
status: current
superseded_by: []
issues: [585, 586, 594]
prs: []
area: ui
---

# The off line links to Manage books, 2026-11-22

Amends [2026-11-08 Manage books, a flat contents list, and no book-page toggle](2026-11-08-manage-books-a-flat-contents-list-and-no-book-page-toggle.md).
Decided by the owner in the UI/UX survey [#594](https://github.com/phynics/cast-iron/issues/594) and
built in [#586](https://github.com/phynics/cast-iron/issues/586).

- **The book page still has no toggle.** Books switch on or off only on Manage books.
- **The off line now ends with a link.** On the book page and on the chapter page of a turned-off
  book, `This book is turned off. Its entries don't appear in search or lists.` is followed by a
  `.count-action` link `Manage books` to `#/books/manage`.
- **It is a plain link.** It does not scroll to the book's row, focus it, or add a route
  parameter. Manage books opens as it does from Books: focus on its `h1`.

The reading footer now follows the same reading of the address as the entry page: it appears only
when the route carries `?book=`, and its eyebrow adds `· draft order` only while the book has open
order questions. A chapter page gains an `In this chapter` list of `{Kind} {count}` links
([#585](https://github.com/phynics/cast-iron/issues/585)); it keeps the address rule of
`2026-11-09-chapter-pages-and-their-address` and the Contents rules of
`2026-11-09-hub-contents-collapse`.
