---
title: "Find a book is route state, narrows by name, and leaves bulk buttons whole"
date: 2026-11-23
status: current
superseded_by: []
issues: [447, 587, 594]
prs: []
area: ui
---

# Find a book is route state, narrows by name, and leaves bulk buttons whole, 2026-11-23

Refs [#587](https://github.com/phynics/cast-iron/issues/587), [#594](https://github.com/phynics/cast-iron/issues/594) and [#447](https://github.com/phynics/cast-iron/issues/447). The rules of the field are in `DESIGN.md` under Views › Books › Find a book. This entry records what a later reader cannot derive from the code.

## Decision

- **The label is visible.** The field reads `Find a book` above it, and the placeholder says it again. The Filter-menu find field has only a placeholder, but it sits inside a panel that has a heading; this field stands alone on the page.
- **The text is route state (`?q=`) on `Books` and on `BooksManage`.** A keystroke replaces the history entry on the next animation frame, and the frame reads the current route when it runs, as the spell-list name filter does after #620. A stale closure then cannot write into a page that the reader has left. Back from a book page restores the text because the address holds it. A blank text is no search and the address drops it.
- **The page is not rebuilt for a keystroke.** The router's page key ignores `q`, so the field keeps its focus and caret; the groups are drawn again from the text. The removed-book line and a share link's question belong to the same page, so typing beside them keeps the page and takes their address away.
- **A search never re-sorts.** It filters. Matches keep book order inside each group, as `#447` required.
- **Matching** ignores case, accents and punctuation, with `cast_iron_search::normalize`, on the full title and the short display title (`#1 Burnt Offerings`). No new dependency and no second folding rule.
- **A search opens the groups with matches and hides the rest.** A hidden group is not in the page, so it is not in the accessibility tree. What the reader opens or folds during a search is not remembered, so Clear brings back the groups the reader had open.
- **A series bulk button on Manage books acts on the whole series while a search shows only some of its books.** The button names the series (`Turn off all Rise of the Runelords books`), not the matches, and a search is a view, not a selection. A group control that silently shrank to the matches would make `Turn all off` mean different books at different times.
- **The count line still counts all books.** Ticking a checkbox in a filtered view changes it.
- **On Manage books the field sits above `Presets`, and a share link's question sits above the field.** The question is the reason a link opens the page, and it keeps the focus a link gives it. `ShareSet` therefore returns two parts, the error with the question and the disclosure with its status line, which the page places apart.

## Not decided here

A sort or an alphabetical jump for books. The find is the only new control.

No format changes: the text lives in the address, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
