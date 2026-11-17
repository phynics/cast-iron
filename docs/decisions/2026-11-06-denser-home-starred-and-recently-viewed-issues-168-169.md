---
title: "denser Home, Starred and Recently viewed (issues #168, #169)"
date: 2026-11-06
status: current
superseded_by: []
issues: [144, 168, 169, 175, 182]
prs: []
area: ui
---

# 2026-11-06: denser Home, Starred and Recently viewed (issues #168, #169)

- **Owner decision: an empty Starred takes no space.** With no stars, Home has no Starred heading. One muted line (a small star icon plus "on an entry to keep it here", with "the star" in screen-reader text) teaches the feature until the first star. It sits under Recently viewed, or alone when nothing was viewed yet. It shows only after the stars have loaded, so it does not flash. With stars the order is Starred, Recently viewed, Categories; without, Recently viewed, Categories.
- **Starred is a list of up to 5 rows.** Each row is the name plus a muted kind (Metadata role) and the existing 44 by 44 unstar button. With more than 5, "See all (N)" expands the list in place and "Show fewer" folds it back (`aria-expanded`, `aria-controls`). There is no separate page: `#/bookmarks` still opens Home. A starred entry the library no longer holds keeps its row, reads "Not in this library", and keeps its unstar button.
- **Recently viewed is the same list, as plain link rows.** Navigation role, not chips: no border, no radius, a hairline `--line` under each row, at least 44 px tall. The title truncates with an ellipsis and never wraps; the muted kind follows it (at most 45% of the row, also truncated). Five rows show; "Show more (N)" expands to the stored history (12, `MAX_RECENT`) with `aria-expanded` and a polite status line; "Clear" in the heading row empties the history at once, with no confirmation, stores the empty list (so it survives a reload), and moves focus to the page title because the section disappears.
- **Equal titles show their book.** When two rows of a list have the same title (case-insensitive), both also show the book (`Creature · Book title`). The test uses the whole stored history, not the visible five, so a row does not change when "Show more" opens.
- **One row component serves both lists** (`entry_row` in `home.rs`; styles `.home-list`, `.home-row`). The spell rows from #144 stay as they are: they have two lines and an overlay link, and the Compact/Wide toggle of #182 applies to them, not to these one-line rows.
- **No two-column grid on wide screens.** The titles of rules entries are long, a single column keeps the kind next to its title and reads in recency order, and the list is at most five rows by default.
- **The star in the hint is an inline SVG** like the other icons. Issue #175 replaces the remaining text glyphs; this change adds none.
- **PRODUCT.md is unchanged.** Home still holds starred entries with direct open and unstar actions and recent reading. Only density and the empty state change.
