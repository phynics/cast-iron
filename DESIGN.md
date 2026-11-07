# Design

Cast Iron keeps one warm, paper-like identity: the palette, type and tokens in `web/styles.css` (`:root`). This file records the rules that later UI work must follow. Product context lives in `PRODUCT.md`.

## Chip roles

A small control can mean five different things. Each role has one form, so a reader can tell what a thing does from its shape. Do not invent a sixth form, and do not borrow one role's form for another.

| Role | Use it for | Form | Class | Never |
|---|---|---|---|---|
| Metadata | What the entry is: kind, book | One muted text line. Links are underlined text, separated by `·`. More than one extra book opens from a `+N more` disclosure. | `.entry-meta`, `.meta-link`, `.meta-more` | Pills, borders, a row of boxes |
| Filter | A choice that narrows a list: search kinds, books and facets; browse facets; the scope of a search. Use it for a group whose options and "All" fit one row at 320 px. | The only pill: `border-radius: 999px`. A selected filter is filled with `--accent` and shows a check mark. The count stays. The scope pill shows a cross instead of the check, because pressing it turns the filter off. | `.chip` (checkbox in `search.rs`), `.lens-chip` (link), `.scope-choice`, `.scope-pill` | A pill for anything else |
| Filter menu | A group too large for one row (feat types, schools, creature types) | A 44 px button with the card radius, a 1 px `--line` border, the text `{Group}: {value}` and a chevron-down icon. A narrowed menu has an `--accent` border and weight 600, with no fill. It opens an in-flow panel of 44 px link rows (label, tabular count, check on the current row, `aria-current`), "All {plural}" first, and a find field above 10 options. | `.filter-btn`, `.filter-panel`, `.choice-list`, `.choice-row` | A pill; a modal, a backdrop or a focus trap; ARIA `menu` |
| Navigation | A jump to another place: recently viewed, shortcuts, spell-list links, search suggestions (Quick search rows: plain link rows with a muted `Kind · Book` line) | A rectangle with the card radius (`--radius`, 6 px) or a plain link row. It may show `aria-current` wayfinding (the current spell level). | `.jump` in `.jump-list`, `.rel-chips li`, `.rel-list` | A pill; a check mark or other filter state |
| Navigation: related page | A link from a list to a related page (Feat chains, Spell lists), in the filter row | Accent text, weight 600, a chevron-right icon, a 44 px hit area, no border. | `.page-link` | A bordered rectangle beside a filter menu: two same-looking shapes with different jobs |
| Label | A short fact about the item next to it: "or", "repeats", "not in this library" | Small muted text with no border, no background and no pointer cursor. Use `.tag-warn` for a warning (error color, heavier weight). | `.tag`, `.tag-warn` | A border, a link, anything that looks clickable |

Rules that apply to every role:

- Interactive parts keep a 44 px target (`--touch`). A text line that stays compact reaches 44 px through a larger hit area (`.meta-link::after`), not through padding.
- Keyboard focus uses the global `:focus-visible` ring (3 px `--focus`). Do not remove it.
- Selected state never relies on color alone: filters add a check mark. Navigation may mark the current page with `aria-current`; it never shows filter state.
- Use `aria-pressed` for a toggle that keeps its place (the star), and `aria-current` for a link to the current page.
- A selected value appears once, in its control. Filter state is route state. Icons are inline SVG with `aria-hidden="true"`, never text glyphs.

## List pages

A list page is a kind's A to Z list (`#/browse/kind/<kind>`). Feat chains, the spell hub and the spell lists follow the same head. Home, entries, Search, Settings, Legal, the Books directory and the book reader are not list pages.

From the top, at 390 px:

1. The masthead (sticky): the brand and the header search icon.
2. A breadcrumb, only when a real parent page exists (see Breadcrumbs).
3. The `h1` (the kind's plural label) and, under it, the count line: `3583 feats`. They stack and never share a row. The count line names the kind, not "entries". It is a live region.
4. Notices.
5. The filter row (not sticky): filter menus on the left, a `.page-link` on the right, and an open menu's panel under the row. Feats, spells, traits, items and archetypes have it. Classes have no filter: they sit under group headings (see below). Races and creatures keep their lens pills until their pull requests of issue #193.
6. The letter bar (sticky, 48 px, `--letter-h`), only above 200 entries. It is the only sticky element below the masthead.
7. The letter heading with its count, then the rows. A row is a title, plus a muted book when a title is printed in several.

No lede, no subhead and no explanatory prose sit above the rows. The blurb that describes a category lives on its Home card. Provenance prose goes below the list.

## Grouped lists

A short kind whose few groups are better seen than chosen is one list under headings, not a filter. Classes are listed under Core, Base, Hybrid, Occult, Unchained, Alternate and NPC, in the order of the books (`views/categories.rs`). A heading is an `h2.letter-head` with the count of its classes; there is no filter row and no letter bar. A group no active book holds shows no heading, and a class without a group is listed last under `Other`, so none is lost. Use a filter menu instead when the groups run to dozens.

## Filter menu

A group with more options than fit one row at 320 px is one filter menu (`views/filter_menu.rs`), not a wall of pills.

- **Button.** `{Group}: {value}`, such as `Type: All types`, 44 px high, `--radius`, a 1 px `--line` border, a chevron-down SVG at the right edge. The visible text is the accessible name. Narrowed: an `--accent` border and weight 600, no fill. Long values ellipsise. While the choices load it reads `Type: Loading…` and is disabled. If they cannot load, the button is hidden and a hint says search still works.
- **Panel.** Non-modal and in the page flow under the filter row: `position: static`, at most `50dvh` with its own scroll, no backdrop, no focus trap, no `<dialog>`, no sheet. A fixed panel with a find field would sit under the iOS keyboard, so the panel scrolls with the page. From 48 rem the rows flow into columns.
- **Contents.** A find field (`Find a type`, shown above 10 options) with a status line (`46 types`, or `No types match "xyz".` and Clear). Then a list of 44 px link rows: `All types` first, then A to Z, each with a tabular count at the right and a check on the current one. Options are links to the filtered route with `aria-current="true"`, not radios and not an ARIA `menu`, because the route is the state.
- **Behaviour.** Opening focuses the find field (the current row when there is no field). Escape closes the panel and returns focus to the button. Choosing a type navigates (a history entry) to the same list with `?facet=field:value`, resets the letter, and focuses the button on the new page. Choosing the current type only closes the panel. Back and Forward walk the filter states.
- **The list reacts in place.** The count line reads `1,324 of 3,583 feats · Show all`; the value is not repeated outside the button. A value absent from the active books keeps its URL, the button shows its default text, and the list says `No Combat feats in your active books.` with `Show all feats` and `Choose books`. Quick search takes the facet as its scope (`Search Combat feats`).

`.page-link` is accent text with weight 600 and a chevron-right SVG, a 44 px hit area and no border. It sits at the right end of the filter row and leads to a related page (`Feat chains`). `.count-action` is the same accent text, underlined, for an action inside a count line or an empty state (`Show all`).

## Chain marker

A feat row carries a quiet marker when its feat leads to more than 5 other feats down its chain (the transitive "leads to" relation, each feat counted once). The marker is a small muted inline SVG (a root joined to two feats, `aria-hidden`, with a hover title) after the title, inside the row's link. A visually hidden suffix gives the row its accessible name: `Power Attack, leads to 93 feats`. The threshold is `CHAIN_MARK_MIN` in `views/browse.rs`; with it, 75 of 3,583 feats (2.1 %) are marked. The numbers come from the relations graph, read once when the page opens (`Relations::long_chains`); the rows do not wait for them. No other row tag exists.

## Breadcrumbs

A breadcrumb lists only the pages between a tab destination and the current page. It never names Home, Books or Settings, and a page directly under a destination has none. When no items remain, the `<nav>` is not rendered.

| Page | Breadcrumb |
|---|---|
| A kind's list page | none |
| Feat chains | `Feats` |
| Spell hub | `Spells` |
| Spell list | `Spells › Spell lists` |

Entry pages and a book's own pages still open with `Home` or `Books`. They change to this rule in a later pull request of issue #193; do not copy their crumbs.

## Scoped search

On a list page the header search icon opens Quick search already limited to that list. The field reads `Search feats` (or `Search Combat feats` once a facet narrows the list), and a pill under the field names the scope: `Feats` with a cross. The pill is the Filter role, its whole 44 px area removes the scope, and its accessible name is `Remove scope: Feats`. Removing it searches everything and keeps the typed text. Suggestions and the results page that Enter opens use the scope.

The results page shows the same pill above its category row, with the same name and the same one-tap removal, and its count line names the scope: `12 results in Combat feats · Show all feats` (the control widens to the whole kind and keeps the text), or `18 results in Feats`. A scope is derived from the page that is open at that moment: on any other page the icon opens an unscoped Quick search.

A recent search stores the scope it ran in (the kind and any facet), not only its text. The row shows the text and a muted scope label (`grapple` and `Feats`); tapping it opens the results in that scope. The same words in another scope are another recent search. A search over everything stores no scope, and the starter searches take the scope of the page.

Focusing a field never navigates. The scope control is never a second field in the header.

## Icon toggles

The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred row on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

## Rows with a separate control

A list row that opens a page and also carries its own control (the Books list: the book link and the Active switch) is one link plus a sibling control. The link fills the row, is at least 44 px tall and ends in a chevron, an inline `aria-hidden` SVG (`.book-chevron`). The control sits beside the link after a hairline, keeps its own 44 px target, and is never nested in the link. Do not hide content behind an inline disclosure in such a row: the page it opens holds the content.

## Tokens

Colors: `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--line`, `--accent`, `--accent-ink`, `--focus`, `--error`. Type: `--font-text`, `--font-display`, `--step--1` to `--step-3`. Space: `--space-1` to `--space-6`. Shape: `--radius` (6 px) for rectangles; 999 px only for filters. Touch size: `--touch`.

The stat block keeps its left accent rule; it is part of the identity.
