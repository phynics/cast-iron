---
title: "Search is a header icon that opens Quick search"
date: 2026-11-06
status: current
superseded_by: []
issues: [151, 164]
prs: []
area: search
---

# Search is a header icon that opens Quick search, 2026-11-06

Refs [#164](https://github.com/phynics/cast-iron/issues/164), [#151](https://github.com/phynics/cast-iron/issues/151). Owner decision from the 2026-11-06 browser review. It revisits the PRODUCT.md line "Search is available as a bar rather than a primary tab": search stays at the top and is not a tab, but it is an icon, not a field.

- **One field per screen.** The header holds a 44 by 44 icon button named "Search" and never a field, so it cannot show a stale query (#151). The results page keeps its own field, without a Search button, because results already update while the reader types. Quick search has its own field, so a screen shows at most one: the results page focuses its field when the icon is tapped, and `/` does the same.
- **Quick search is a native modal `<dialog>`.** It is a full-screen layer under 48 rem and a large sheet near the top above it. The browser gives the focus containment, the `dialog` role and the modal backdrop; the app adds `aria-modal`, the label "Quick search", a Tab wrap inside the layer, and focus to the field in the same event that opens it, because a phone shows the keyboard only for a focus inside the gesture. The expand animation runs only under `prefers-reduced-motion: no-preference`.
- **History.** Opening pushes one history entry with a marker state, on the same URL. Cancel, Escape and Back all end in `history.back()` and a `popstate` handler, so they leave the same history and return to the same page, scroll position and focus (the icon, or the control that was focused when `/` opened it). The scroll position is saved at open and restored at close, and focus is restored without scrolling, because Chromium scrolls a sticky header control's natural position into view. Submitting replaces that entry with the destination, so Back from the results goes to the page that was open before the layer, not to a closed layer. A route change that the layer did not make, such as a typed address, closes it without a history step.
- **The field is empty at each opening.** The route is the only holder of the query; the layer is a way to reach a route.
- **Icon on every width.** A field-styled button on wide screens was considered and left out: a second form of the same control would need its own tests and gives a reader nothing the icon and `/` do not.
- **Starter searches** ("fireball", "grapple", "cure light", "dragon") appear in the layer while the field is empty, and still on the empty results page.
