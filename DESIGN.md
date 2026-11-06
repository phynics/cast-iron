# Design

Cast Iron keeps one warm, paper-like identity: the palette, type and tokens in `web/styles.css` (`:root`). This file records the rules that later UI work must follow. Product context lives in `PRODUCT.md`.

## Chip roles

A small control can mean five different things. Each role has one form, so a reader can tell what a thing does from its shape. Do not invent a sixth form, and do not borrow one role's form for another. Rows marked "(arrives in PR 2)" are defined ahead of their code (issue #193); do not build another form for the same job in the meantime.

| Role | Use it for | Form | Class | Never |
|---|---|---|---|---|
| Metadata | What the entry is: kind, book | One muted text line. Links are underlined text, separated by `·`. More than one extra book opens from a `+N more` disclosure. | `.entry-meta`, `.meta-link`, `.meta-more` | Pills, borders, a row of boxes |
| Filter | A choice that narrows a list: search kinds, books and facets; browse facets; the scope of a search. Use it for a group whose options and "All" fit one row at 320 px. | The only pill: `border-radius: 999px`. A selected filter is filled with `--accent` and shows a check mark. The count stays. The scope pill shows a cross instead of the check, because pressing it turns the filter off. | `.chip` (checkbox in `search.rs`), `.lens-chip` (link), `.scope-choice`, `.scope-pill` | A pill for anything else |
| Filter menu (arrives in PR 2) | A group too large for one row (feat types, schools, creature types) | A 44 px button with the card radius, a 1 px `--line` border, the text `{Group}: {value}` and a chevron-down icon. A narrowed menu has an `--accent` border and weight 600, with no fill. It opens an in-flow panel of 44 px link rows (label, tabular count, check on the current row, `aria-current`), "All {plural}" first, and a find field above 10 options. | `.filter-btn`, `.filter-panel`, `.choice-list`, `.choice` | A pill; a modal, a backdrop or a focus trap; ARIA `menu` |
| Navigation | A jump to another place: recently viewed, shortcuts, spell-list links, search suggestions (Quick search rows: plain link rows with a muted `Kind · Book` line) | A rectangle with the card radius (`--radius`, 6 px) or a plain link row. It may show `aria-current` wayfinding (the current spell level). | `.jump` in `.jump-list`, `.rel-chips li`, `.rel-list` | A pill; a check mark or other filter state |
| Navigation: related page (arrives in PR 2) | A link from a list to a related page (Feat chains, Spell lists), in the filter row | Accent text, weight 600, a chevron-right icon, a 44 px hit area, no border. | `.page-link` | A bordered rectangle beside a filter menu: two same-looking shapes with different jobs |
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
5. The filter area: today the existing lens pills; from PR 2 one filter row with filter menus on the left and a `.page-link` on the right.
6. The letter bar (sticky, 48 px, `--letter-h`), only above 200 entries. It is the only sticky element below the masthead.
7. The letter heading with its count, then the rows. A row is a title, plus a muted book when a title is printed in several.

No lede, no subhead and no explanatory prose sit above the rows. The blurb that describes a category lives on its Home card. Provenance prose goes below the list.

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

The results page shows the same pill above its category row, with the same name and the same one-tap removal. A scope is derived from the page that is open at that moment and is never stored: on any other page the icon opens an unscoped Quick search.

Focusing a field never navigates. The scope control is never a second field in the header.

## Icon toggles

The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred row on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

## Tokens

Colors: `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--line`, `--accent`, `--accent-ink`, `--focus`, `--error`. Type: `--font-text`, `--font-display`, `--step--1` to `--step-3`. Space: `--space-1` to `--space-6`. Shape: `--radius` (6 px) for rectangles; 999 px only for filters. Touch size: `--touch`.

The stat block keeps its left accent rule; it is part of the identity.
