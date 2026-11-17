---
title: "A book row opens the book page"
date: 2026-11-06
status: partly-superseded
superseded_by: ["2026-11-07-active-books-and-the-presets"]
issues: [107, 154]
prs: []
area: ui
---

# A book row opens the book page, 2026-11-06

Closes [#154](https://github.com/phynics/cast-iron/issues/154). Owner decision (option B1). It supersedes the inline contents tree of the #107 entry above.

- **One row, one link.** Each row on `#/books` is a single link (`a.book-directory-row`) to the book page, at least 44 px tall. It shows the title, the entry count, the category and date line, and a trailing chevron (an inline SVG, `aria-hidden`, so #175 has nothing to replace). The whole row is the target, as in an Apple HIG list row.
- **The switch stays a sibling.** The Active switch (`.book-activation`) is a separate control to the right of the link, separated by a hairline, 44 px tall. It never sits inside the link, so the two targets do not nest, and toggling it does not navigate.
- **No inline tree and no "Read [book]" link.** The `<details>` per row, `BookTree`, `DirectoryNode` and the `.book-tree*` CSS are deleted. The Books page no longer loads `books.json`, so it mounts without building the tree or waiting for that file. The book page (`views::book`) is the one place to read a book's contents; it already shows the disabled notice, the entry count, the order note and the lazy tree, so nothing moves there.
- **Tier groups, group controls and sort links are unchanged.** The e2e coverage of the lazy contents tree moved from the list to the book page.
- **Back restores the scroll position.** Leaving Books for a book page unmounts Books, and every route change used to scroll to the top, so Back landed at the top of a long list. The router now marks each history entry it reaches with a random id in `history.state` (`scrollId`) and keeps the last scroll offset of each id in memory (`route::remember_scroll`, `arrive_at_entry`). Back and Forward to an entry that this session left return to that offset; a new entry, a reload and an anchor link behave as before (top, or the anchor). `route::replace` and the legacy and boot URL rewrites keep the state they replace; Quick search leaves its own entry with `route::replace_as_new_page`, which starts the page clean. The state is one object, `{scrollId, quickSearch}`: the scroll memory owns `scrollId`, Quick search owns `quickSearch`, each reads and writes only its field, and the entry Quick search pushes copies the page's `scrollId`, so Back to it restores the page's offset. The memory keeps the last 100 entries. A chapter toggle writes its chapter into the URL only while the address bar still shows that book: the browser fires `toggle` in a task after the click, and a Back that lands first would otherwise have the late event rewrite the Books entry into the book page (seen in CI). A group that the reader opened by hand starts collapsed again when Books remounts, so the offset is exact only for groups that are open by default.
- **Measured mount time.** The Books mount-time numbers are in the pull request.
