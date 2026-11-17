---
title: "The route focus move yields to the reader and to a newer navigation"
date: 2026-11-17
status: current
superseded_by: []
issues: [419]
prs: []
area: ui
---

# The route focus move yields to the reader and to a newer navigation, 2026-11-17

Refs [#419](https://github.com/phynics/cast-iron/issues/419), part 1. Part 2, the VoiceOver check of the loading-to-loaded transition on an iPhone, stays open for the iPhone session (#423).

After a route change, `Reader` moves focus to `<main>` in a `requestAnimationFrame` so that a screen reader announces the new page. A frame has no fixed deadline. A fast keyboard user, switch control or a slow device could focus a control after the navigation began, and the frame then took focus (and the key just pressed) away. The same held for a newer navigation that superseded an older, still queued frame. The flake of #121 is this case. The section-jump frames (`Reader` for a stop on a page that is on screen, `EntryView` on a first load; #455) moved focus to a section heading with the same race.

- **A generation counter, bumped where the route signal changes.** `focus_guard::begin_navigation` runs in the `hashchange` handler and in `navigate` and `replace`, before the route signal is written. Bumping at the source and not in `Reader`'s effect means that `EntryView`'s frame, which is scheduled while the page renders and may be built before `Reader`'s effect runs, takes its ticket after the bump of its own navigation and not before it. A frame whose ticket is older than the counter does nothing at all, scroll included: the newer navigation's frame scrolls.
- **Focus yields to any element other than the one that had it when the frame was scheduled.** `FocusTicket::take` records the focused element at scheduling. When the frame fires, `FocusTicket::verdict` returns `Proceed` when the body has focus (nothing is focused) or the same element still has it, `ScrollOnly` when another element has it, and `Stale` when the counter moved. The same element counts as no move because a reader who clicked a link in the header (Chrome focuses it) must still get the move to `<main>`; a focused element that the new page removed leaves the body focused, which also proceeds. `ScrollOnly` still scrolls to the anchor, the offset or the top, since the page changed under the reader and that move is not what they are typing against; it skips only `focus_main` and the focus half of the section jump (`route::scroll_to_section_with(section, false)`).
- **The cost.** A page that focuses its own control after a navigation (the Search box, Settings' Updates link) already moved focus before the frame, so the frame leaves it alone, which is what those pages wanted (Search was excluded by `to_search` for that reason). The generic move to `<main>` no longer runs for such a page. If a screen reader needs the announcement there, the page owns it.
- **The decision is a pure function.** `focus_guard::decide` takes the two generations and the two focused elements and is covered by native tests; the document reads are the only browser code (rule 8). `tests/e2e/tests/route-focus.spec.ts` runs the race inside a `hashchange` listener that the page adds after the app's own, so the app's frame is already scheduled when the test focuses a control, and a spy on `HTMLElement.prototype.focus` counts the moves to `<main>` for a double navigation.
