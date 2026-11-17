---
title: "Header update indicator"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: install-offline
---

# Header update indicator, 2026-11-05

- **An app update replaces the Install button in the one header slot.** `views::header_action::choose_action` is the pure rule, with a native test: when `shell_update_ready` is true the header shows Update, whether or not Install would also apply. Update is the action the reader can apply now.
- **The banner is gone.** The full-width `.banner` below the header pushed the reading content down when an update arrived. The header button keeps `--header-h` fixed, so `<main>` does not move. Settings keeps its `.banner-inline` row with "App update ready. Reload" for readers who look there.
- **The button announces once, the click reloads.** Update uses the shared `.header-action-btn` pill: a 14px refresh arc in `currentColor` and a 6px `var(--accent)` pip on the icon, with the visible label on screens 48rem and wider and the icon alone below it. Its accessible name is "App update ready. Reload to apply". A visually hidden `role="status"` element carries "App update ready." when the state becomes ready; the button itself is not a live region, so it does not re-announce. Clicking it calls `js::apply_shell_update()`.
