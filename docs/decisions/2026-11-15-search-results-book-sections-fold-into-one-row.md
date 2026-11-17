---
title: "Search results: book sections fold into one row"
date: 2026-11-15
status: current
superseded_by: []
issues: [167, 387]
prs: []
area: search
---

# Search results: book sections fold into one row, 2026-11-15

Refs [#387](https://github.com/phynics/cast-iron/issues/387), [#167](https://github.com/phynics/cast-iron/issues/167). Owner decision, option C of the mockup review. For a broad query the matching book sections came before every entry: at 390 px with `fire` the first section row started at 304 px but the first entry at 907 px. Rejected: A (sections first, as before) and B (sections after all entries).

- **One native `<details>` in the page flow**, not a sections-only scope. The row reads `Book sections · 37 matches` (`1 match` for one). It is the Navigation role: 44 px, `--radius`, a 1 px `--line` border, a trailing chevron. The accessible name is the visible text, so it carries the count. A scope would need a new `Route::Search` field, a second results mode and its own Back behaviour; the disclosure needs none of that and keeps the sections one activation away.
- **The open state is in the URL, and the owner overrode the first decision on 2026-11-15 after the visual review.** The first build kept it in a screen signal, so Back from a section returned with the row folded and a reader comparing sections paid two taps each. Now `Route::Search` has a `sections` flag (`&sections=1`; only `1` opens). A toggle calls `replace`, so it never adds a history entry and Back never undoes a toggle. The search screen reads the flag through its own memo, so a toggle does not re-run the search. `SearchState::route()` never sets the flag, so a new query or a filter change writes a route without it and the row folds again; this also removes a stale open row after a filter that leaves no sections. `Show more results` is not in the URL and keeps the row as it is.
- **Focus.** Native `<details>` keeps focus on the summary on open and close; Enter and Space toggle it. Nothing moves focus into the list, because the reader chose to open it and the next Tab stop is the first section.
- **The fold applies at every width.** The sections are secondary to entries on desktop too, and one component avoids a second layout. There is no breakpoint.
- **Only sections match.** The sections are listed open, under a `Book sections · 14 matches` heading (the folded row's wording, with a real separator), because nothing sits below them to protect and a fold would hide the only results. The count line still reads `No entry results`.
- **An exact-title section match gets no special treatment.** Entries come first by design, and the count is visible; a rule that promotes one section would need a ranking comparison between sections and entries that the search engine does not make.
- **The list still shows at most eight sections.** When there are more, a note says `Top 8 of 37 book sections. Refine the search to see the rest.` Before, the heading showed 37 over eight rows.
- **Thresholds.** At 390 px the first entry result starts at most 380 px down with the hidden-matches hint (`fire`) and at most 330 px without it (`fireball`). `mobile.spec.ts` measures the first entry hit, not the fold row. The #385 thresholds for the first result row (280 and 330 px) now apply to the fold row.
- **An open list ends inside its own group.** The `<details>` carries the 1 px `--line` border and the card radius, so the summary and the sections share one container, and the entry results start below it (the section rows lose their last hairline). No `Entries` label exists elsewhere on the results page, so none was added.
