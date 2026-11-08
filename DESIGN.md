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
| Navigation: related page | A link from a list to a related page (Feat chains, Spell lists, `Active books ›` on Books), in the filter row | Accent text, weight 600, a chevron-right icon, a 44 px hit area, no border. | `.page-link` | A bordered rectangle beside a filter menu: two same-looking shapes with different jobs |
| Label | A short fact about the item next to it: "or", "repeats", "not in this library", `Inactive` on a book row | Small muted text with no border, no background and no pointer cursor. Use `.tag-warn` for a warning (error color, heavier weight). | `.tag`, `.tag-warn` | A border, a link, anything that looks clickable |

Rules that apply to every role:

- Interactive parts keep a 44 px target (`--touch`). A text line that stays compact reaches 44 px through a larger hit area (`.meta-link::after`), not through padding.
- Keyboard focus uses the global `:focus-visible` ring (3 px `--focus`). Do not remove it.
- Selected state never relies on color alone: filters add a check mark. Navigation may mark the current page with `aria-current`; it never shows filter state.
- Use `aria-pressed` for a toggle that keeps its place (the star), and `aria-current` for a link to the current page.
- A selected value appears once, in its control. Filter state is route state. Icons are inline SVG with `aria-hidden="true"`, never text glyphs.

## Glyphs

Text is never an icon. A chevron, arrow, cross, check or star is an inline SVG (`currentColor`, `aria-hidden="true"`), and the control keeps the accessible name. Every `<summary>` that opens a disclosure draws the same chevron and hides the platform's own marker. All icons share the 24 px grid and a 1.8 px round stroke. Two separators stay text because they separate rather than act: the muted `·` in a metadata or count line, and the `›` between breadcrumbs (and in the book trail).

## List pages

A list page is a kind's A to Z list (`#/browse/kind/<kind>`). Feat chains, the spell hub and the spell lists follow the same head. Home, entries, Search, Settings, Legal, the Books directory and the book reader are not list pages.

From the top, at 390 px:

1. The masthead (sticky): the brand and the header search icon.
2. A breadcrumb, only when a real parent page exists (see Breadcrumbs).
3. The `h1` (the kind's plural label) and, under it, the count line: `3583 feats`. They stack and never share a row. The count line names the kind, not "entries". It is a live region.
4. Notices.
5. The filter row (not sticky): filter menus on the left, a `.page-link` on the right, and an open menu's panel under the row. Feats, spells, traits, items and archetypes have it. Classes have no filter: they sit under group headings (see below). Races and creatures keep their lens pills until their pull requests of issue #193.
6. The letter bar (sticky, 48 px, `--letter-h`), only above 200 entries. It is the only sticky element below the masthead.
7. The letter heading with its count, then the rows. A row is a title, plus a muted book when a title is printed in several, or the revision's book when the row shows a revision.

No lede, no subhead and no explanatory prose sit above the rows. The blurb that describes a category lives on its Home card. Provenance prose goes below the list.

## Grouped lists

A short kind whose few groups are better seen than chosen is one list under headings, not a filter. Classes are listed under Core, Base, Hybrid, Occult, Unchained, Alternate and NPC, in the order of the books (`views/categories.rs`). A heading is an `h2.letter-head` with the count of its classes; there is no filter row and no letter bar. A group no active book holds shows no heading, and a class without a group is listed last under `Other`, so none is lost. Use a filter menu instead when the groups run to dozens.

## Filter menu

A group with more options than fit one row at 320 px is one filter menu (`views/filter_menu.rs`), not a wall of pills.

- **Button.** `{Group}: {value}`, such as `Type: All types`, 44 px high, `--radius`, a 1 px `--line` border, a chevron-down SVG at the right edge. The visible text is the accessible name. Narrowed: an `--accent` border and weight 600, no fill. Long values ellipsise. While the choices load it reads `Type: Loading…` and is disabled. If they cannot load, the button is hidden and a hint says search still works.
- **Panel.** Non-modal and in the page flow under the filter row: `position: static`, at most `50dvh` with its own scroll, no backdrop, no focus trap, no `<dialog>`, no sheet. A fixed panel with a find field would sit under the iOS keyboard, so the panel scrolls with the page. From 48 rem the rows flow into columns.
- **Contents.** A find field (`Find a type`, shown above 10 options) with a status line (`46 types`, or `No types match "xyz".` and Clear). Then a list of 44 px link rows: `All types` first, then A to Z, each with a tabular count at the right and a check on the current one. Options are links to the filtered route with `aria-current="true"`, not radios and not an ARIA `menu`, because the route is the state.
- **Behaviour.** Opening focuses the find field (the current row when there is no field). Escape closes the panel and returns focus to the button. Choosing a type navigates (a history entry) to the same list with `?facet=field:value`, resets the letter, and focuses the button on the new page. Choosing the current type only closes the panel. Back and Forward walk the filter states.
- **The list reacts in place.** The count line reads `1,324 of 3,583 feats · Show all`; the value is not repeated outside the button. A value absent from the active books keeps its URL, the button shows its default text, and the list says `No Combat feats in your active books.` with `Show all feats` and `Active books` (a link to `#/books/active`). Quick search takes the facet as its scope (`Search Combat feats`).

`.page-link` is accent text with weight 600 and a chevron-right SVG, a 44 px hit area and no border. It sits at the right end of the filter row and leads to a related page (`Feat chains`). `.count-action` is the same accent text, underlined, for an action inside a count line or an empty state (`Show all`).

## Chain marker

A feat row carries a quiet marker when its feat leads to more than 5 other feats down its chain (the transitive "leads to" relation, each feat counted once). The marker is a small muted inline SVG (a root joined to two feats, `aria-hidden`, with a hover title) after the title, inside the row's link. A visually hidden suffix gives the row its accessible name: `Power Attack, leads to 92 feats`. The threshold is `CHAIN_MARK_MIN` in `views/browse.rs`; with it, 70 of 3,583 feats (2.0 %) are marked. The numbers come from the relations graph, read once when the page opens (`Relations::long_chains`); the rows do not wait for them. Add-ons are not chain followers and never count toward the marker or its number. No other row tag exists.

## Breadcrumbs

A breadcrumb lists only the pages between a tab destination and the current page. It never names Home, Books or Settings, and a page directly under a destination has none. When no items remain, the `<nav>` is not rendered.

| Page | Breadcrumb |
|---|---|
| A kind's list page | none |
| Feat chains | `Feats` |
| Spell hub | `Spells` |
| Spell list | `Spells › Spell lists` |
| Active books | none |
| Book page | none |

Entry pages still open with `Home` or `Books`. They change to this rule in a later pull request of issue #193; do not copy their crumbs.

## Scoped search

On a list page the header search icon opens Quick search already limited to that list. The field reads `Search feats` (or `Search Combat feats` once a facet narrows the list), and a pill under the field names the scope: `Feats` with a cross. The pill is the Filter role, its whole 44 px area removes the scope, and its accessible name is `Remove scope: Feats`. Removing it searches everything and keeps the typed text. Suggestions and the results page that Enter opens use the scope.

The results page shows the same pill above its category row, with the same name and the same one-tap removal, and its count line names the scope: `12 results in Combat feats · Show all feats` (the control widens to the whole kind and keeps the text), or `18 results in Feats`. A scope is derived from the page that is open at that moment: on any other page the icon opens an unscoped Quick search.

A recent search stores the scope it ran in (the kind and any facet), not only its text. The row shows the text and a muted scope label (`grapple` and `Feats`); tapping it opens the results in that scope. The same words in another scope are another recent search. A search over everything stores no scope, and the starter searches take the scope of the page.

Focusing a field never navigates. The scope control is never a second field in the header.

## Toggles

**Star.** The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred row on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

**Toggle row.** Use it for a stored on/off choice that is saved right away: the rows on Active books and the book page's `Active` row. The whole row is a `<label>` of at least 44 px (56 px with a meta line) that holds a native checkbox (no `role=switch`). It has no chevron and does not navigate. A row has one accessory: a chevron (it opens something) or a checkbox (it toggles something), never both. The checkbox never disables itself while it saves, because that drops keyboard focus; writes queue instead. Class: `.toggle-row`.

## Books

Books is where a reader reads; Active books is where a reader changes which books count.

- **Books head.** The H1, one live count line (`aria-live=polite`) that names the matching preset or `Custom` (`28 of 439 books active · Paizo Rulebooks`; `No books active` when none), and a `.page-link` `Active books ›` in the filter-row slot. There is no lede, no hint, no sort control, no group Activate or Deactivate button and no `Only Rulebooks`.
- **One order.** Tier order (`Rulebooks`, `Player Companions`, `Campaign Setting`, `Adventures`, `Other`), and inside a tier the oldest book first, undated books last, ties by title. An old `?sort=` address opens `#/books` and the address bar says so.
- **Groups** (Books and Active books) are an `h2` in `--font-display`, `--step-1`, weight 700, `--space-5` above, that holds one disclosure `button[aria-expanded][aria-controls]`: a 16 px chevron SVG at the left and a muted, weight 400, tabular count at the right (`31 books` when every book in the group is active, otherwise `28 of 31 active`, including `0 of 151 active`). No fill and no box. Rulebooks starts open and the other groups closed; the open set is kept in memory for the session, so Back to Books finds the same groups open before the scroll position returns. Groups differ from rows by type (serif 700 against sans 600), the space above, the chevron and the right-aligned count; rows have 1 px `--line` rules.
- **A book row on Books is a link only** (`a.book-directory-row`): the title (sans, weight 600), a muted meta line, and a trailing chevron (inline SVG, `aria-hidden`), at least 56 px. The meta line is a month and year from a fixed English table and then the grouped count (`Aug 2009 · 2,994 entries`; the count alone when the date is unknown). A book that is not active ends the line with the `Label` role text `Inactive`. The row carries no checkbox. Do not hide content behind an inline disclosure in a row: the book page holds the content.
- **Book page** (`#/browse/book/<id>`), in this order: the H1 (no breadcrumb), the same meta line as the row, the `Active` toggle row between two hairlines (a whole-row `<label>` with a native checkbox and a state line), `In this book`, `Contents`, and the colophon. `In this book` is an `h2` and a `.jump-list` of `.jump` links reading `{Kind} {count}`, in Home category order, for the kinds the book holds (Rules has no list page). Each opens `#/browse/kind/<kind>?source=<id>`. Below a 340 px container width the rectangles become plain 44 px rows with a hairline, label left and a tabular count right (a container query; at 390 px they stay rectangles). `Contents` carries at most one muted regular-weight line (`Chapter order not yet checked against the printed book`) while the book's order is a draft, then the chapters. It leaves out spells, feats, items and class options (classes, prestige classes, archetypes), which `In this book` reaches, and a chapter that held only those kinds is not listed. The chapter the corpus calls `New entries (page order)` reads `More from this book`, and a book whose only listed chapter is that one shows its rows at the top. No row carries a flag or a note about its position. The colophon is a `<footer>` at the bottom: each distinct copyright notice once, then `Legal and attribution`. A book page highlights the Books tab; the old `#/browse/source/<id>` address is replaced by it.
- **Active books** is the route `#/books/active`, not local state: every change is saved at once, so a reload or Back stays on the page and Settings can link to it. The Books tab stays highlighted. The head is the H1 `Active books` with `Done` (accent text, weight 600, no chevron, 44 px) at its right, then a count line without the preset name (`412 of 439 books active`). Entering moves focus to the H1. `Done` goes Back when the Books link opened the page, otherwise it replaces the entry with Books; it returns focus to the `Active books ›` link.
- **Presets** (`h2` `Presets`): 44 px button rows with the name on the left, a muted tabular count (`28 books`) and a check mark with `aria-pressed="true"` on the one that matches the active set. No fill, no bold, hairlines between rows. A preset that repeats another preset's set, or has no books, is not shown, so `Everything incl. 3rd party` stays hidden until the corpus marks third-party books (#189). Applying one writes the choice once, shows a muted status line (`Applied Paizo Everything.`, a live region) and an accent `Undo` button. Undo restores the previous set and lasts until the next change or until the reader leaves the page. A preset's change is worked out inside the write, after any earlier write, so two quick taps end on exactly the second preset.
- **Below the presets**, Active books lists the same groups with a checkbox per book (toggle rows). There is no group control of any kind.

## Attached entries

An add-on (variant, mythic version, Stamina rules, a third-party change or revision) is its own entry from its own book. It appears only while that book is active. An inactive base never hides it. Search, lists, facets, backlinks, stars and Recent already work per entry, so an add-on needs no second filter model. There is no rule-set switch and no gated body block.

**The add-on's own page.** The metadata line ends with one link fragment: `Variant of {base}`, `Mythic version of {base}`, `Stamina rules for {base}`, `Changes {book} makes to {base}`, or `Revision of {base}`. The last appears only on the revision entry's own page. There is no `Base …` stat row; a value appears once. Search rows read `Mythic · {book}`, `Stamina · {book}`, or `Changes · {book}`; a class option reads `{Group singular} · {book}`. No breadcrumb.

**The base page,** after the body, in this order:

- each mythic version: an `h2` `Mythic`, the add-on's title as an `h3` link, a muted `Sources` line, and its text, expanded;
- third-party changes: an `h2` with the book title, in the same form;
- Stamina: one collapsed `details.rel-disclosure` `Stamina · {book}` with a 44 px summary;
- on a spell, `Variants`: a `.rel-list` of rows, each the title with a muted line `Bard 6, Cleric 5, Druid 6 · Core Rulebook`.

No box, no tint, and no left rule; the stat block keeps the only one.

**Class page.** `Class options` is a `.jump-list` of `.jump` rectangles `{Group} {count}`, each opening `#/browse/kind/class_option?facet=group:{slug}`. The list page has the H1 `Class options`, a count line, and a group filter menu; a class `.scope-pill` appears only when a group serves several classes. Archetype rows show a muted `Replaces {items}` from `replaces`. Option rows show only `{book}`, because the group is the selected filter.

**A revision in force.** A `.notice` reads `Revised by {book}. Changed: {fields}.`, or `Removed by {book}.` / `Merged by {book}.` followed by the printed sentence. Each changed field row carries the Label `changed` (plain muted `.tag`, never accent or bold), then a collapsed `details.rel-disclosure` `Original text · {book}` with a 44 px summary. A list row adds the revision's book as its muted line. Removed feats leave the list, and one prose line under it states the count. When two or more revisions are in force the page shows the original plus `Revised by` and a `.rel-list`; there is no substitution and no last-writer-wins.

**Lists and chains.** A mythic feat is its own row sorting right after its base (`Dodge`, `Dodge, Mythic`); there is no `+ Mythic` row tag. The `Leads to` panel and the chain marker ignore add-ons. Under a revision, the chain shows the revision's prerequisites with a revised notice. The chain-marker figures are measured again when the corpus gains add-ons.

**Never:** pills (the `.scope-pill` stays the only one here), cards, coloured borders, row tags, inline diff colours, or an activation control on an entry.

**Owner decisions.** A collapsed Stamina row is enough, with no switch. Druid animal companions are class options. Mythic spells become add-on entries in slice 3.

## Tokens

Colors: `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--line`, `--accent`, `--accent-ink`, `--focus`, `--error`. Type: `--font-text`, `--font-display`, `--step--1` to `--step-3`. Space: `--space-1` to `--space-6`. Shape: `--radius` (6 px) for rectangles; 999 px only for filters. Touch size: `--touch`.

The stat block keeps its left accent rule; it is part of the identity.
