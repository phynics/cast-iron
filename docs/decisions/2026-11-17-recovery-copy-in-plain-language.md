---
title: "Recovery copy in plain language"
date: 2026-11-17
status: current
superseded_by: []
issues: [443, 445]
prs: [465]
area: ui
---

# Recovery copy in plain language, 2026-11-17

Closes [#445](https://github.com/phynics/cast-iron/issues/445). Part of [#443](https://github.com/phynics/cast-iron/issues/443).

- **Every recovery message says what happened, what was kept, and one next step, in a player's words.** The book-choice recovery screen no longer prints the store's error (`local storage error: TypeError: Reflect.get called on non-object`) in the alert. It reads `Cast Iron couldn't read which books you have turned on. Your downloaded library and your stars are safe.` The engine text stays under a closed `Details for a bug report` disclosure, so a bug report can still carry it.
- **The screen cannot tell a damaged value from a busy browser store, so it claims neither.** Both come back as the same error. The copy says "couldn't read" and offers `Try again` before `Reset book choices`. Reset writes the empty disabled set and then loads the library, and that load keeps the third-party opt-in rule (#189): a third-party book the device has not remembered yet is turned off again, and a remembered one turns on. The next-step sentence says exactly that: `every book turns on, except any third-party books you haven't seen before, which stay off`. Making reset forget the remembered set, so that all third-party books stay off, would be a behaviour change and was not done. A failed reset says nothing was changed.
- **An unknown book is `Book unavailable`, like `Entry unavailable`.** `#/browse/book/<unknown>`, a chapter address of an unknown book, and an old `#/browse/source/<unknown>` all show it, with links to Books and Home and no id. Before, the page used the slug as its H1 and said the library had no chapter order for it, which made a missing book look like a real one with incomplete contents. A book the library holds but whose contents are unavailable keeps its own title and `This library has no chapter order...` lines.
- **A deleted one-off tree (a blog post or comic issue) still opens Books, now with a line.** The #420 decision stands: such an address opens Books. It now parses to `Route::BookRemoved(id)`, the page is Books, and a `.notice` above the count says `That book is no longer in the library. Its entries are still in the books they came from.` (the page still shows Books, so it is a notice, not an alert) The route round-trips to the address the reader used, so Back and reload behave as before. A deleted listing tree that maps to a printed book still opens that book without a line, because the reader lands on the book they meant.
- **A list or search link with an unknown `source=` is no longer silent.** The kind list keeps showing the whole list, with `That book isn't in your library, so this list shows all spells.` as a `.notice` (the list is still usable, so it is not an alert). Search with an unknown book matches nothing and used to say only `No results`; it now adds the `.inactive-books-hint` line `A book in this link isn't in your library, so it can't match anything.` with a `Remove it` button that drops the unknown ids and keeps the known ones. This is the one new action; without it the reader had no control that removed the invisible filter.
- **A stored choice that names books the library lacks stays valid and silent.** It is not an error: the ids keep their state (see `next_disabled`), so a book that returns keeps the reader's choice. A spec seeds one and asserts that the app opens and shows no id.
- **Wording is fixed in constants** (`RECOVERY_*`, `BOOK_UNAVAILABLE_*`, `BOOK_REMOVED_LINE`, `MISSING_BOOK_LINE`, `unknown_source_notice`) and asserted by `recovery-copy.spec.ts` on Chromium and the phone project.
