---
title: "Recently viewed in Quick search"
date: 2026-11-09
status: current
superseded_by: []
issues: [356]
prs: []
area: search
---

# Recently viewed in Quick search, 2026-11-09

Closes [#356](https://github.com/phynics/cast-iron/issues/356). Owner decisions recorded in the issue on 2026-11-09. Builds on "Quick search suggestions and recent searches" and "Continue reading per book".

- **Recently viewed moves off Home.** Home keeps Starred and the category directory, and the teaching line until the first star. Quick search's empty field shows Recent searches first and then a Recently viewed group, so one place holds the reader's search history and reading history.
- **The Continue row leads the group.** `Continue {Book}` is the first row of Recently viewed and no longer appears on Home. Its link keeps `?book=<source>`, so opening it stays inside that book and moves the point forward. The book page keeps its own `Continue reading: {entry title}` link.
- **Five rows, then Show more.** The Continue row counts toward the five-row cap. `Show more (N)` expands the rest with `aria-expanded`, and `Show fewer` folds it back; the polite status line names the total when expanded.
- **Each group clears itself.** Recent searches keeps its `Clear`. Recently viewed gets its own `Clear`, which empties the visit list and removes `continue_points`, so it also forgets where the reader stopped. Each recently viewed row also has a remove button, and Delete on the highlighted row removes it.
- **Visibility matches the library.** A stored visit resolves through the alias map and opens under the canonical id. A retired entry, an unknown id, or an entry whose book is inactive is hidden. Continue points are read the same way, and the most recently read *active* book supplies the row.
- **One listbox per group, one keyboard order.** The field keeps focus; `aria-controls` names every group's listbox and `aria-activedescendant` names the focused option. Down and Up cross the groups in display order, and Enter opens the focused row. The row rules, order, cap, continue row and empty states are native tests in `views/quick_rows.rs`; the store and the layer add no new JavaScript.
