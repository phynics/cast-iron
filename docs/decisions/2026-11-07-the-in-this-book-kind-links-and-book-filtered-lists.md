---
title: "The `In this book` kind links and book-filtered lists"
date: 2026-11-07
status: current
superseded_by: []
issues: [158]
prs: []
area: ui
---

# The `In this book` kind links and book-filtered lists, 2026-11-07

Closes [#158](https://github.com/phynics/cast-iron/issues/158). Delivery step E of the Books council (`BOOKS-COUNCIL.md`, owner decisions of 2026-11-07).

- **The book page lists its kinds between the `Active` row and `Contents`.** `In this book` is an `h2` and a `.jump-list` of `.jump` links, each `{Kind} {count}`, in Home category order (`views::categories::arrange`, then the remaining kinds). Only kinds with at least one entry are listed, and Rules is left out because it has no list page. The count is the catalog's canonical-source count, the same set the Books row shows, so it includes a disabled book's entries. Each link is `#/browse/kind/<kind>?source=<book id>`.
- **The links are new navigation rectangles, not a colour strip.** At a container width below 340 px the `.jump` rectangles become plain 44 px rows: no border, fill or radius, a hairline under each, the label left and a tabular count right. A CSS container query on the section drives it, so the form follows the column and not the viewport; at 390 px the rectangles remain.
- **Contents now leaves out the linked kinds.** `LEAVE_OUT_LINKED_KINDS` in `views/book.rs` is `true`, so spells, feats, items, classes, prestige classes and archetypes are reached through the links. A book that held only those kinds says so and points at `In this book`.
- **`Route::BrowseKind` gains `source`.** It parses and writes `?source=<id>` beside `l=` and `facet=`, and `Route::with_source` keeps the other two when the book filter is removed. The source list reads the whole catalog (`Library::stub_is_in_source`), not the active snapshot, so a disabled book still lists what it holds. With a facet, the ids come from the whole index, for the same reason; without a book the list keeps the active-book snapshot it had. An unknown id is the ordinary list with `Unknown book "<id>". Showing all <kind plural>.` and is never NotFound, so a bookmark survives a rename.
- **The book filter is a Filter-role pill with a cross.** It sits above the count line, carries the book's name, and its accessible name is `Remove filter: {Book}`. Removing it keeps the letter and the facets and moves focus to the first filter control, or to the H1 when the page has none (a grouped class list, for example). That H1 carries `tabindex="-1"` and the fallback waits two animation frames, so it wins over the shell's `main` focus. A book filter has no filter menu; it arrives only from a book page.
- **The count line counts the book's set and never repeats the book.** `626 spells` for the book alone; `48 of 626 spells · Show all` once a facet narrows that set, with `Show all` widening to the book's whole kind. `0 spells` plus `No spells in Core Rulebook.` and `Browse all spells` when the book has none of the kind.
- **A book-filtered class list is a plain A to Z list.** Classes are grouped by `class_type` from the active index; a book may be disabled, and its group data would then vanish. The book filter therefore takes the plain list, with the same pill, count and empty state as every other kind. This is the conservative choice until the group data can read the whole catalog.
- **Scoped Quick search carries the book.** `Scope` gains `source` and `source_title`; a book-filtered list scopes the search to its kind, facets and book, and the pill reads `Feats in Core Rulebook` with the field `Search feats in Core Rulebook`. The title is stored with a recent search, so the pill still names the book after a reload; two scopes are the same when their kind, facets and source id match. A results page with a single kind and a single publication shows the scope too, and removing it clears all three.
- **No serialized app format changed.** The recent-search record gains optional fields with serde defaults; old entries read as before. Content and the search index are untouched, so `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
