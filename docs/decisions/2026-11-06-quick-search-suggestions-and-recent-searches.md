---
title: "Quick search suggestions and recent searches"
date: 2026-11-06
status: current
superseded_by: []
issues: [164, 165]
prs: []
area: search
---

# Quick search suggestions and recent searches, 2026-11-06

Refs [#165](https://github.com/phynics/cast-iron/issues/165), [#164](https://github.com/phynics/cast-iron/issues/164). Builds on "Search is a header icon that opens Quick search".

- **Suggestions come from the engine's ranking.** While the field has text, the layer asks the existing search index (through the active-book discovery snapshot) for the best 64 hits, keeps the best hit of each kind and title, and shows the first 8, each with its name and a muted `Kind · Book` line. A last row, "See all results for …", opens the results page. There is no second index. The order is the engine's, with the tie-break below, so the suggestions are the first rows of the results page for the same text except where the engine tied. A reprint of a spell in other books is one suggestion; the entry page lists the other books.
- **Suggestions are the engine's order with a shortest-completion tie-break for near ties.** The engine scores `fireb` as Firebrand (archetype) 522.5, Fireball 518.7, Firebrand (spell) 518.2, Firebrand (feat) 517.5 and Fireball Trap 515.7, so the results page lists Firebrand first. The scores are not equal, only within one percent of each other, so an exact-equality tie-break never fires. For suggestions only, hits within 1% of the best score of a run whose title starts with the typed text are ordered shortest title first, then the engine's order; other hits and other runs keep the engine's order, and a real score gap is never crossed (`firebal` scores Fireball 519.0 against Controlled Fireball 216.3). The results page and `crates/search` are unchanged. So `fireb` suggests Fireball first and `firebrand` suggests Firebrand first, and the first suggestion can differ from the first result where the engine nearly tied. `shortest_completion_first` in `quick_rows.rs` has the native tests; the 1% band is the `NEAR_TIE` constant.
- **The lookup waits for a pause.** It starts 100 ms after the last keystroke, and a newer keystroke cancels it through the same sequence counter that drops stale answers. When the suggestions change, the highlight is cleared, so `aria-activedescendant` never names a missing row and Enter never opens a row the reader did not see.
- **Announcements.** The count of suggestions is announced only while the list is expanded. The header icon carries `aria-haspopup="dialog"` except on the results page, where it only focuses that page's field.
- **Known limits.** If the address changes while the layer is open (a typed address or an external link), the layer closes and its same-URL history entry stays, so one Back press appears to do nothing. Opening the keyboard on a phone as the layer opens relies on focusing inside the tap; that is verified in Chromium at a phone size, not yet on an iOS Safari device.
- **Combobox, not a menu.** The field has `role="combobox"` with `aria-controls`, `aria-expanded` and `aria-activedescendant`; the rows are `role="option"` in a listbox, 44 px high. Focus stays in the field. Down and Up move the highlight (Up from the first row returns to the field), Enter opens the highlighted row or, with none, searches the text. The highlight has an accent rule at the row start as well as a tint.
- **Escape closes the list, then the layer,** but only while the reader has typed text. The recent and starter searches are the content of the empty layer, not a popup, so Escape there closes the layer at once. Down or typing brings a closed list back.
- **Recent searches** are the last 10 submitted queries: Enter in Quick search, "See all results", a recent or starter search, or Enter in the results page's field. Opening an entry from a suggestion does not count. They are normalized (trimmed, single spaces), unique without regard to case, newest first, and stored in the same `meta` store as the recently viewed entries (key `recent_searches`), so they stay on the device, work offline, and nothing is sent anywhere. Each has its own remove button (a normal button after the option, reachable by Tab; Delete on a highlighted recent search does the same), and "Clear" removes all. With none, the starter searches show instead.
- **Rules are native tests.** `recent_searches.rs` tests order, dedupe, cap, case and empty input. `quick_rows.rs` tests the row rules, the reprint collapse, the cap and the highlight movement.
