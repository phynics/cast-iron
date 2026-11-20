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
| Navigation: related page | A link from a list to a related page (Feat chains, Spell lists, `Manage books ›` on Books), in the filter row | Accent text, weight 600, a chevron-right icon, a 44 px hit area, no border. | `.page-link` | A bordered rectangle beside a filter menu: two same-looking shapes with different jobs |
| Label | A short fact about the item next to it: "or", "repeats", "not in this library", `Inactive` on a book row | Small muted text with no border, no background and no pointer cursor. Use `.tag-warn` for a warning (error color, heavier weight). | `.tag`, `.tag-warn` | A border, a link, anything that looks clickable |

Rules that apply to every role:

- Interactive parts keep a 44 px target (`--touch`). A text line that stays compact reaches 44 px through a larger hit area (`.meta-link::after`), not through padding.
- Keyboard focus uses the global `:focus-visible` ring (3 px `--focus`). Do not remove it.
- Selected state never relies on color alone: filters add a check mark. Navigation may mark the current page with `aria-current`; it never shows filter state.
- Use `aria-pressed` for a toggle that keeps its place (the star), and `aria-current` for a link to the current page.
- A selected value appears once, in its control. Filter state is route state. Icons are inline SVG with `aria-hidden="true"`, never text glyphs.

## Glyphs

Text is never an icon. A chevron, arrow, cross, check or star is an inline SVG (`currentColor`, `aria-hidden="true"`), and the control keeps the accessible name. Every `<summary>` that opens a disclosure draws the same chevron and hides the platform's own marker. The exceptions are the two Collections pickers, which keep the platform marker, and the `+N more` link in a metadata line, which is text. All icons share the 24 px grid and a 1.8 px round stroke. Two separators stay text because they separate rather than act: the muted `·` in a metadata or count line, and the `›` between breadcrumbs (and in the book trail).

## List pages

A list page is a kind's A to Z list (`#/browse/kind/<kind>`). Feat chains, the spell hub and the spell lists follow the same head. The spell hub (`#/spells`) is the head and the class picker: the count line reads `31 spell lists`, and it has no filter row. A class list (`#/spells/<class>[/<level>]`) is the head, a filter row with `School` (multi-select, below) and the `Class page` `.page-link`, the name field, the level bar (`All levels`, then the levels with their counts: it is not sticky, because up to eleven 44 px targets wrap to two rows at 390 px), the density choice and the rows. Its count line reads `199 spells` (`26 cantrips` at level 0). Home, entries, Search, Settings, Legal, the Books directory and the book reader are not list pages.

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
- **The list reacts in place.** The count line reads `1,324 of 3,583 feats · Show all`; the value is not repeated outside the button. A value absent from the active books keeps its URL, the button shows its default text, and the list says `No Combat feats in your active books.` with `Show all feats` and `Manage books` (a link to `#/books/manage`). Quick search takes the facet as its scope (`Search Combat feats`).

**Multi-select.** A group whose values combine (the schools of a spell list) uses the same button and panel with checkbox rows. The button reads `School: All schools`, `School: Evocation` for one, `School: 2 chosen` for more, and is narrowed (an `--accent` border) from the first. A row is a `<label>` of 44 px with a native checkbox, the label and a tabular count; `All schools` stays first as a link with the same box drawn before it, ticked while nothing is chosen (the box is decoration; `aria-current` carries the state). Activating it while it is current only closes the panel, as the current row of a single-choice menu does; with schools chosen it clears them. A row's accessible name carries its noun (`Evocation, 22 spells`). The route is still the state: `#/spells/Wizard/3?school=evocation&school=conjuration`, the choices in the order made, a spell of any of them stays. A tick is one history entry, so Back and Forward walk the choices, and the page is not rebuilt for it: the panel stays open and the focus stays on the row. Opening focuses the first ticked row, or `All schools`. A level link keeps the chosen schools. The count line reads `45 of 199 spells · Show all`, and `Show all` clears the schools and the name filter. Use it only for a group where a spell can sensibly be in several values; a single-choice group stays a link list.

`.page-link` is accent text with weight 600 and a chevron-right SVG, a 44 px hit area and no border. It sits at the right end of the filter row and leads to a related page (`Feat chains`). `.count-action` is the same accent text, underlined, for an action inside a count line or an empty state (`Show all`).

## Footnotes

A footnote is never rules text, so it must not look like it (#507). On an entry page the notes leave the flow of the text and sit in one quiet group at the end of the body, `aside.notes` (a complementary landmark named by its `Notes` heading). The corpus keeps a note as a level-six heading with a `footnote-` anchor plus the one paragraph after it; `views/footnotes.rs` classifies that pair from the typed nodes, so the serialized body does not change. Only that paragraph belongs to the note: a monster's note sits inside its stat block, and the lines after it stay in the body.

- **Group.** A hairline `--line` rule above it, then an `h2.notes-title` in small uppercase muted type (`0.75rem`, weight 600). The group is always open: a disclosure would hide text that readers look for after they follow a marker. It is the last child of `.rich`, never in the contents list ("On this page" lists sections only), and it has no card, fill or accent border, so it never competes with a stat block or a quote.
- **Type.** Notes use `--step--1` in `--ink-muted` with a `1.5` line height, below the `1.0625rem` rules text. Sizes are rem, so they scale with Dynamic Type. The label (`1`, `B`) is `--ink-dim`, weight 600, tabular, in a fixed column; a screen reader hears `Note 1`. The note's DOM id is the id its heading would have had (`h-footnote-1`), so an old link to `#footnote-1` still lands on it.
- **Markers.** A marker (`^1^`, the `^B^` after a monster's bonus feat) in a paragraph becomes `sup.fn-ref`, a quiet annotation and not part of the word before it (#517): `max(0.7em, 0.6875rem)`, so smaller than the text with a floor for small stat lines; `--ink-muted`, the colour of the notes; weight 400 and no underline, so it never reads as a bold or accented part of a feat name. It is a link to its note. Hover and keyboard focus bring back `--accent` and an underline, and the focus ring stays. The accessible name says what the mark is: `Note 1`, and `Bonus feat, note B` for a `B` whose note reads `Bonus feat.` (the note decides, because other books use `B` for other things). The same treatment applies to every marker, number or letter, linked in the corpus or not, because a footnote mark is one kind of thing. The hit area is an invisible `::after`, so it takes no space: about 29 px tall and as wide as the comma after it. It grows up, down and to the right, never left onto the feat name, and on a touch screen (`pointer: coarse`) it is about 36 px tall. A linked marker follows its explicit anchor. A bare marker links to the first note with its label that follows it, or to the last one before it. A number straight after a digit (`10^2^`) is an exponent, not a marker. A marker in a table cell or a stat row is the same link (#534): the same size, colour, name and back link, and the note lists its back links in the order the page renders in. A marker that cites no note of the entry, such as an exponent, stays a plain superscript. A way back to a marker inside a closed disclosure (the spell columns of a class progression) opens the disclosure first.
- **Way back.** Each note ends with one icon-only link per marker that cites it (`Back to note 1 in the text`). The icon is small and the hit area reaches 44 px through `::after`, so a note stays compact. A note whose marker is missing from the text (a common case) shows its label and text and no way back.
- **Never.** Do not render a note as a heading, in rules-text size or colour, or inside a stat block. Do not put `h6` footnote headings in the contents. Do not size the group with `px`.

## Chain marker

A feat row carries a quiet marker when its feat leads to more than 5 other feats down its chain (the transitive "leads to" relation, each feat counted once). The marker is a small muted inline SVG (a root joined to two feats, `aria-hidden`, with a hover title) after the title, inside the row's link. A visually hidden suffix gives the row its accessible name: `Power Attack, leads to 92 feats`. The threshold is `CHAIN_MARK_MIN` in `views/browse.rs`; with it, 70 of 3,583 feats (2.0 %) are marked. The numbers come from the relations graph, read once when the page opens (`Relations::long_chains`); the rows do not wait for them. Add-ons are not chain followers and never count toward the marker or its number. No other row tag exists.

## Primary navigation

Four tabs in one order, **Home, Books, Table, Settings**, each an icon above or beside its label. Below 48rem they are the bottom bar: one equal column per tab, each at least 44 px wide and `--nav-h` tall (at 390 px, about 97 px wide). From 48rem they are the header's top nav, icon beside label, after the brand. The tab for the current section carries `aria-current="page"`, and a page inside a section keeps its tab: kind lists and spell lists belong to Home, a book and its chapters to Books, Legal to Settings. Table is the die icon (`views/icons.rs`, the set's 24 px grid and 1.8 px round stroke); its page is Quick reference, and nothing else belongs to it. The header keeps the `--wide` column edge from 48rem, so four tabs must fit beside the brand, the search icon, the install action and the Offline badge at 48rem, 1024 px and 1280 px without wrapping.

## Quick reference

`#/quick` (`views/quick.rs`) is the Table tab. It is an `h1` and four groups, each an `h2.sub-head` over a `.jump-list` of `.jump` chips: **Conditions** (A to Z), **Actions**, **Combat modifiers**, **Every session**. The groups are one Rust constant (`GROUPS`): a label, an entry id and, for a few chips, a `sec-` heading anchor inside the entry. The page copies no rules text. A chip opens the entry (and scrolls to the anchor when it has one), so the rule is read where it is maintained. A chip whose id is not in the library is not drawn, and a group with no chip left is not drawn. Chips follow the chip roles above: navigation, a rectangle with the card radius, never a pill, never a count. There is no chip row on Home; Starred is the personal list.

## Breadcrumbs

A breadcrumb lists only the pages between a tab destination and the current page. It never names Home, Books, Table or Settings, and a page directly under a destination has none. When no items remain, the `<nav>` is not rendered.

| Page | Breadcrumb |
|---|---|
| A kind's list page | none |
| Feat chains | `Feats` |
| Spell hub | `Spells` |
| Spell list | `Spells › Spell lists` |
| Manage books | none |
| Book page | none |

Entry pages still open with `Home` or `Books`. They change to this rule in a later pull request of issue #193; do not copy their crumbs.

## Scoped search

On a list page the header search icon opens Quick search already limited to that list. The field reads `Search feats` (or `Search Combat feats` once a facet narrows the list), and a pill under the field names the scope: `Feats` with a cross. The pill is the Filter role and wraps to as many lines as the scope needs rather than being cut (a scope of four schools takes three lines at 390 px), its whole area (at least 44 px) removes the scope, and its accessible name is `Remove scope: Feats`. Removing it searches everything and keeps the typed text. Suggestions and the results page that Enter opens use the scope.

The results page shows the same pill above its category row, with the same name and the same one-tap removal, and its count line names the scope: `12 results in Combat feats · Show all feats` (the control widens to the whole kind and keeps the text), or `18 results in Feats`. The spell hub scopes to `Search spells`. A class list scopes to its class and the chosen schools (`Search Evocation or Conjuration Wizard spells`; several values of one field read joined by "or", a class last). The level is not part of the scope (see `docs/decisions.md`). A scope is derived from the page that is open at that moment: on any other page the icon opens an unscoped Quick search.

**Results page controls.** Below the field is one scope row, `.search-controls`: the quick kinds (`All entries`, `Rules text`, `Spells`, `Feats`) are Filter pills in a row that scrolls sideways and never wraps, and the compact `Filters` button stays at its end. The button is the Filter-menu shape (44 px, card radius, 1 px `--line`; an `--accent` border while filters are on) and reads `Filters · N` when N filters are active. Below 56 rem it opens the filter panel in the page flow under the row (at most `50dvh` with its own scroll, no backdrop, no trap, no dialog); opening moves focus into the panel, and Escape or the button closes it and returns focus to the button. From 56 rem the panel is the sidebar and the button is hidden. The breakpoint stays at 56 rem, not the 48 rem of the filter menu's column flow, because a 16 rem sidebar next to the results needs that width; do not move it to 48 rem. There is no Search button, because results are live. The status area follows the row: a small count line and, when an inactive book holds more matches, the one-sentence hint with a `Turn on` action.

**Book sections on the results page.** Entries come first. The matching book sections fold into one Navigation row above the entries, `Book sections · 37 matches`, a native `<details>` in page flow: 44 px, `--radius`, a 1 px `--line` border, label left and a chevron at the end that turns when it opens (never a pill, never a filter state). Its accessible name carries the count. The `<details>` owns the 1 px border and the radius, so the open list sits in one container and the entries start below it. Opening lists at most eight sections (a note says `Top 8 of 37 book sections. Refine the search to see the rest.` when there are more) and keeps focus on the row; the row folds again with Enter or Space. The open state is `sections=1` in the search URL: a toggle replaces the history entry, Back from a section returns with the row open, and a new query or filter folds it. When a query matches only book sections, they are listed open under a `Book sections · 14 matches` heading, because nothing sits below them. The same fold applies at every width.

A recent search stores the scope it ran in (the kind and any facet), not only its text. The row shows the text and a muted scope label (`grapple` and `Feats`); tapping it opens the results in that scope. The same words in another scope are another recent search. A search over everything stores no scope, and the starter searches take the scope of the page.

Focusing a field never navigates. The scope control is never a second field in the header.

## Quick search

The header search icon opens Quick search. While the reader types, it lists suggestions; with an empty field it lists a Quick reference row, Recent searches (or the starter searches when none) and then Recently viewed. The empty-state groups are the content of the layer, not a popup, so they stay expanded and Escape closes the layer at once.

- **Rows.** Suggestions and recently viewed entries are plain navigation rows: the title, then a muted `Kind · Book` line, 44 px tall, with no border or pill. The Continue row reads `Continue {Book}` and puts the saved entry's title on the muted line.
- **Quick reference row.** While the field is empty and no scope is set, the first option is `Quick reference`, with the muted line `Conditions · Actions · Combat modifiers`. It is a plain row of the same form, and it opens `#/quick` (the Table tab) like any row opens its page, replacing the layer's history entry. It sits in a group of its own with a screen-reader heading and no visible one. It is a destination, not a search: it is never saved as a recent search and never counts as a search sample in diagnostics. Typing any text hides it; so does a scope, because a scoped layer was opened from a list to search that list. Removing the scope or clearing the text brings it back.
- **Recently viewed.** It shows the stored visits whose ids still resolve through the alias map and whose book is active; a retired entry or an inactive book is hidden. The Continue row is its first row when the reader has a saved place in an active book. Five rows show; `Show more (N)` expands the rest with `aria-expanded`, and `Show fewer` folds it back. Each group has its own `Clear`: Recent searches clears only searches; Recently viewed clears the visits and the continue points. Each recently viewed row also has a remove button, and Delete on the highlighted row does the same.
- **Order and keyboard.** The Quick reference row comes first when it shows, then Recent searches, then Recently viewed. Down and Up move through the groups in order, and Enter opens the focused row. The rows are options in a listbox per group; the field keeps focus and names every group's listbox through `aria-controls`, while `aria-activedescendant` names the focused option.
- **Home.** Home no longer shows Recently viewed. It holds Starred and the category directory, and the teaching line remains until the first star. Starred shows six entries, then `See all (N)`. Its rows are cells of the same grid as the category cards (`.cats`): one column below 36rem, two from 36rem, three from 64rem, with the same column gap, so each cell has the edges of the card column below it. Each cell keeps its own hairline row: title, muted kind, star. The rows read left to right and then down (the DOM order).

## Toggles

**Star.** The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred cell on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

**Toggle row.** Use it for a stored on/off choice that is saved right away: the rows on Manage books. The whole row is a `<label>` of at least 44 px (56 px with a meta line) that holds a native checkbox (no `role=switch`). It has no chevron and does not navigate. A row has one accessory: a chevron (it opens something) or a checkbox (it toggles something), never both. The checkbox never disables itself while it saves, because that drops keyboard focus; writes queue instead. Class: `.toggle-row`.

## Books

Books is where a reader reads; Manage books is where a reader changes which books count.

- **Books head.** The H1, one live count line (`aria-live=polite`) that names the matching preset or `Custom` (`28 of 439 books active · Paizo Rulebooks`; `No books active` when none), and a `.page-link` `Manage books ›` in the filter-row slot. There is no lede, no hint, no sort control, no group Activate or Deactivate button and no `Only Rulebooks`.
- **One order.** Tier order (`Rulebooks`, `Player Companions`, `Campaign Setting`, `Adventures`, `Other`), and inside a tier the oldest book first, undated books last, ties by title. An old `?sort=` address opens `#/books` and the address bar says so.
- **Groups** (Books and Manage books) are an `h2` in `--font-display`, `--step-1`, weight 700, `--space-5` above, that holds one disclosure `button[aria-expanded][aria-controls]`: a 16 px chevron SVG at the left and a muted, weight 400, tabular count at the right (`31 books` when every book in the group is active, otherwise `28 of 31 active`, including `0 of 151 active`). No fill and no box. Rulebooks starts open and the other groups closed; the open set is kept in memory for the session, so Back to Books finds the same groups open before the scroll position returns. Groups differ from rows by type (serif 700 against sans 600), the space above, the chevron and the right-aligned count; rows have 1 px `--line` rules.
- **Adventures** nests the same disclosure one level deeper (#159): **Adventure Paths** (each path is `Rise of the Runelords (6)`, its volumes ordered by issue number and titled `#1 Burnt Offerings`), **Modules** (one list ordered by publication date, without the `Pathfinder Module:` prefix), and **Pathfinder Society** (each `Season 3` holds its scenarios by number). Publications with no series are a plain **Other** list at the end and are never dropped. Every series block starts open inside the tier, so opening Adventures shows the same rows it did before the grouping; a reader folds a path or a season to shorten the list. A series heading is an `h3` (or `h4` for a path or season) in `--font-display`, `--step-0`, weight 700, with the same chevron and right-aligned tabular count. No box, fill or border; nesting is shown by the space above. On Manage books each path, `Modules` and each season also shows `N of M active` and a 44 px `Turn all on` / `Turn all off` text button; the tier itself still has no bulk control.
- **A book row on Books is a link only** (`a.book-directory-row`): the title (sans, weight 600), a muted meta line, and a trailing chevron (inline SVG, `aria-hidden`), at least 56 px. The meta line is a month and year from a fixed English table and then the grouped count (`Aug 2009 · 2,994 entries`; the count alone when the date is unknown). A book that is not active ends the line with the `Label` role text `Inactive`. The row carries no checkbox. Do not hide content behind an inline disclosure in a row: the book page holds the content.
- **Book page** (`#/browse/book/<id>`), in this order: the H1 (no breadcrumb), the same meta line as the row, a `Continue reading: {entry title}` link when this book has a saved place, a plain muted line while the book is turned off, `In this book`, `Contents`, and the colophon. The off line reads `This book is turned off. Its entries don't appear in search or lists.`; an active book shows no line and no control. Books are switched on or off only on Manage books. `In this book` is an `h2` and a `.jump-list` of `.jump` links reading `{Kind} {count}`, in Home category order, for the kinds the book holds (Rules has no list page). Each opens `#/browse/kind/<kind>?source=<id>`. Below a 340 px container width the rectangles become plain 44 px rows with a hairline, label left and a tabular count right (a container query; at 390 px they stay rectangles). `Contents` carries at most one muted regular-weight line (`Chapter order not yet checked against the printed book`) while the book's order is a draft, then one expanded, nested list of chapters and sections. There are no disclosure buttons: a branch row shows its title (a link when the chapter is itself an entry) and a muted tabular count of the entries below it. A top-level chapter row joins the title and count with a dotted leader, as on a printed contents page; the leader is decorative (`aria-hidden`) and becomes a plain gap below a 340 px container width. Contents leaves out every kind that has its own list page, which is every kind except Rules; `In this book` reaches them. A chapter whose entries are all left out is not listed, and only rules chapters and sections remain. The corpus chapter `New entries (page order)` is not shown at all: it is not a Contents row, it has no chapter page, and it is left out of previous and next. A book left with no rows still shows `In this book` and its kind links, and a short note (`This book has no entries to show.`) when it has no list-page kind. A section below chapter level that is itself an entry collapses to one row (its link and muted count) when that entry's page already lists every entry below it; the children are not repeated. A corpus-marked node (`collapse = true` in the curated book file) collapses at any level, including a top-level chapter: its row links to the node's entry, or to its chapter page when it has no entry. An unmarked top-level chapter row never collapses: it always opens its chapter page. No row carries a flag or a note about its position. The colophon is a `<footer>` at the bottom: each distinct copyright notice once, then `Legal and attribution`. A book page highlights the Books tab; the old `#/browse/source/<id>` address is replaced by it. Every top-level Contents row opens its chapter page, `#/browse/book/<id>/<chapter-id>`; the id is the chapter anchor or a slug of its printed title, never its row number. The chapter page is full width with the breadcrumb Books › Book › Chapter: the chapter's first-level sections are `h2` headings and deeper levels are plain indented lists, with no disclosure. A section that is itself an entry and whose page already lists every entry below it keeps only its link. A chapter that is itself an entry offers one link to that entry. An entry opened from a book or a chapter carries `?book=<id>` and ends with the reading footer: `Previous` and `Next in {Book}`, the chapter name added when the step crosses a chapter. Previous and next follow the same order as Contents, so the kinds Contents leaves out are skipped and the catch-all chapter is never a step. Back from a chapter page restores the book page's scroll position, and Back from an entry returns to the chapter page it came from.
- **Share a book set** (#408) sits on Manage books (#504), after the presets and before the book groups, and not on Books: a code changes the active set as a preset does, so it belongs where the set is changed, and Books keeps the count line, the `Manage books ›` link and then the first group. Its question is scrolled into view when a link or code opens it, because it can sit below the first screen. Each side has its own status line and Undo, and a line describes the books only while nothing has changed them since: any later change that succeeds, from the presets, a checkbox, a group button or this section, ends every Undo line on the page. A change that fails to save changes no line. An Undo that was clicked while another change was still saving does nothing once that change has landed, so it never puts back books that the newer change chose. It is one native `<details class="share-set">` with a 44 px summary in `.page-link` colour and weight (`Share these books or enter a code`) and a chevron that turns when it opens, like the other disclosures. It holds the code in a bordered monospace block that selects whole on a tap (`user-select: all`), two 44 px `.btn` buttons (`Copy code`, `Copy link`) with a muted live line (`Copied the code.`), then the `Enter a code` label, a 44 px field and a `Review` button in one row, and an `.error` line that `role=alert` announces. The page keeps the same hairline rhythm as the presets: no card, no tint.
  - **The question** (`section.share-confirm`, above the disclosure) is an `h2.sub-head` that takes focus (`Use this set of books?`, or `Same books` when nothing would change), a bold summary (`Turns on 12 books, turns off 3.`), a muted result line in the form of the count line (`Afterwards: 28 of 439 books active · Paizo Rulebooks`, `Custom` when no preset matches), a muted line for books skipped because this library lacks them, and two `.btn` buttons: `Use this set` (`.btn-primary`) and `Cancel`. Opening a link or reviewing a code only asks; the books change when `Use this set` is pressed. The answer writes once, shows the status line that names what it made (`Applied Paizo Everything.`, or `Now using 27 books.` when no preset matches) with an accent `Undo`, in this section's own `.undo-line` under the disclosure, and takes the code out of the address (`#/books/manage?set=<code>` becomes `#/books/manage`). A link made when the section sat on Books (`#/books?set=<code>`) still works: it opens the same question, and the router rewrites its address. The question belongs to the history entry that holds the link: Back or Forward to another entry takes it away, and Forward to the link's entry asks again while its address still carries the code. A save closes only the question it answered: a newer code that the reader reviewed meanwhile keeps its question and its text, and a save that fails does not show its notice next to it. A save that finishes after the reader has left the page changes no screen, and one that finishes while Quick search is open rewrites no address until the reader is back on the link's entry.
  - Buttons here use the existing `.btn` forms and no new one: one filled `.btn-primary` per question (`Use this set`), and bordered `.btn` for every other action (`Copy code`, `Copy link`, `Review`, `Cancel`), all 44 px. The code block is a muted `--surface-2` block with no border and 0.08em tracking, so it is not mistaken for the field below it, and the status line after an apply names the preset (`Applied Paizo Rulebooks.`) or gives the count (`Now using 27 books.`) at the size of the other muted lines. An error from a bad link is an `.error` with `role=alert` above the question, outside the disclosure, which is closed when a link opens the page. An error from a typed code is the same element inside the form. Undo stays until another selection change succeeds; reviewing a code, a rejected typo, Cancel and a failed write leave it alone. A library whose books carry no slot shows none of this.
- **Manage books** is the route `#/books/manage`, not local state: every change is saved at once, so a reload or Back stays on the page and Settings can link to it (the Library section holds the `Manage books` row). The old `#/books/active` address parses to the same page and the address bar writes the new one. The Books tab stays highlighted. The head is the H1 `Manage books` with `Done` (accent text, weight 600, no chevron, 44 px) at its right, then a count line without the preset name (`412 of 439 books active`). Entering moves focus to the H1, except when a share link opens the page: its question takes the focus, because it is what the link is for. `Done` goes Back when the Books link pushed the history entry that is on screen, otherwise it replaces the entry with Books (an entry that a share link pushed later on the same page is one of those); it returns focus to the `Manage books ›` link.
- **Presets** (`h2` `Presets`): 44 px button rows with the name on the left, a muted tabular count (`28 books`) and a check mark with `aria-pressed="true"` on the one that matches the active set. No fill, no bold, hairlines between rows. A preset that repeats another preset's set, or has no books, is not shown, so `Everything incl. 3rd party` stays hidden until the corpus marks third-party books (#189). Applying one writes the choice once, shows a muted status line (`Applied Paizo Everything.`, a live region) and an accent `Undo` button. The line takes no height until a change shows in it, so the presets are followed at once by the share section (12 px, no empty band); the share section's own status line behaves the same. Undo restores the previous set and lasts until the next change that succeeds or until the reader leaves the page. A preset's change is worked out inside the write, after any earlier write, so two quick taps end on exactly the second preset.
- **Below the presets**, Manage books lists the same groups with a checkbox per book (toggle rows). The only group control is the Adventures series bulk button described under Groups; a tier has none.

## Attached entries

An add-on (variant, mythic version, Stamina rules, a third-party change or revision) is its own entry from its own book. It appears only while that book is active. An inactive base never hides it. Search, lists, facets, backlinks, stars and Recent already work per entry, so an add-on needs no second filter model. There is no rule-set switch and no gated body block.

**The add-on's own page.** The metadata line ends with one link fragment: `Variant of {base}`, `Mythic version of {base}`, `Stamina rules for {base}`, `Changes {book} makes to {base}`, or `Revision of {base}`. The last appears only on the revision entry's own page. There is no `Base …` stat row; a value appears once. Search rows read `Mythic · {book}`, `Stamina · {book}`, or `Changes · {book}`; a class option reads `{Group singular} · {book}`. No breadcrumb.

**The entry's own text ends with one muted `Sources:` line** when the entry has typed citations:
`Sources: Ultimate Combat p. 221; Advanced Player's Guide p. 12`. Each publication uses its
display title and links to its book page, `#/browse/book/<id>`, when the library has book pages.
`p. {page}` appears only when the citation has a page. The line sits after the body and above the
related panels. An entry without rows keeps whatever `Sources:` paragraph its body already prints,
so an unconverted entry and a converted one read the same.

**The `Expanded in:` credit line (#460)** sits directly under the `Sources:` line, in the same muted Metadata role (`.expanded-line`), and only when the entry has `expanded_in` rows: `Expanded in: Monster Summoner's Handbook p. 18`. It credits a book that expands the entry without printing it, so it is never a membership: the metadata line and the book counts do not change.
- Each credit is one `.expanded-credit`, joined by `; `. The book title links to its book page (plain text when the library has no book pages). `p. {page}` is plain text, and a credit without a page prints the title alone.
- A `via` entry turns the page number into a link to that entry, only while the entry is visible under the reader's active books. A hidden or unknown `via` entry leaves plain text; no credit ever links to a hidden entry.
- A credit from a book the reader turned off is dimmed (`data-inactive="true"`) with the `--ink-dim` colour token for its text and both links, never with opacity (an accent link under opacity measured 4.2:1 in the light theme). `--ink-dim` measures 4.76:1 in the light theme and 5.64:1 in the dark one against the page ground. The credit ends with the `Inactive` Label (`.tag`), so the state never rests on dimming alone. The title still links to the book page, as a book row on Books does.
- A revision in force does not replace the credits like it replaces citations: the base's credits stay, then the revision's own, each once.
- At 64 rem and wider the line owns the grid row `expanded` between `sources` and `aon`. With an `Archives of Nethys` line after it the line keeps the larger bottom margin that the 44 px AoN target needs.

**The base page,** after the body, in this order:

- each mythic version: an `h2` `Mythic`, the add-on's title as an `h3` link, its text expanded, then its own muted `Sources` line;
- third-party changes: an `h2` with the book title, in the same form, its own `Sources` line after the text;
- Stamina: one collapsed `details.rel-disclosure` `Stamina · {book}` with a 44 px summary, whose opened body ends with its `Sources` line;
- on a spell, `Variants`: a `.rel-list` of rows, each the title with a muted line `Bard 6, Cleric 5, Druid 6 · Core Rulebook`.

An add-on section names its own citations. A converted add-on prints its typed line after its
text; an unconverted body keeps its printed paragraph, so the section never prints a second line
for the same citation. A mythic version with no printed citation falls back to its book name.

No box, no tint, and no left rule; the stat block keeps the only one.

**Class page.** `Class options` is a `.jump-list` of `.jump` rectangles `{Group} {count}`, each opening `#/browse/kind/class_option?facet=group:{slug}`. The list page has the H1 `Class options`, a count line, and a group filter menu; a class `.scope-pill` appears only when a group serves several classes. Archetype rows show a muted `Replaces {items}` from `replaces`. Option rows show only `{book}`, because the group is the selected filter.

**A revision in force.** A `.notice` reads `Revised by {book}. Changed: {fields}.`, or `Removed by {book}.` / `Merged by {book}.` followed by the printed sentence. Each changed field row carries the Label `changed` (plain muted `.tag`, never accent or bold), then a collapsed `details.rel-disclosure` `Original text · {book}` with a 44 px summary; its body ends with the base's own `Sources` line. The page's `Sources` line follows the revision's text: a revision with typed rows replaces the base citation, and a revision with none keeps the base's rows. A removed or merged page shows the revision's citation under the printed sentence. A list row adds the revision's book as its muted line. Removed feats leave the list, and one prose line under it states the count. When two or more revisions are in force the page shows the original plus `Revised by` and a `.rel-list`; there is no substitution and no last-writer-wins.

**Recovery and unavailable pages (#445).** A page that cannot show what was asked for follows `Entry unavailable`: an H1 that names the state (`Book unavailable`, never the raw id), then one `.error` with `role="alert"` that says what happened and, where it matters, what was kept (only where the page has no content to show), then one or two links out (`Browse all books`, `go to Home`). A state that still shows content uses a `.notice` instead and says what the page does: the kind list reads `That book isn't in your library, so this list shows all spells.`, and a search with a missing book shows the muted `.inactive-books-hint` line with a `Remove it` action. The book-choice recovery screen puts the alert, one sentence of next steps, and `Try again` (filled) and `Reset book choices` (bordered) together, and keeps the engine's own text under a closed `Details for a bug report` disclosure with the shared chevron. The removed-book line above Books is a `.notice`, because the page still shows Books. No message shows an id, a slot, a schema or a manifest.

**Lists and chains.** A mythic feat is its own row sorting right after its base (`Dodge`, `Dodge, Mythic`); there is no `+ Mythic` row tag. The `Leads to` panel and the chain marker ignore add-ons. Under a revision, the chain shows the revision's prerequisites with a revised notice. The chain-marker figures are measured again when the corpus gains add-ons.

**Never:** pills (the `.scope-pill` stays the only one here), cards, coloured borders, row tags, inline diff colours, or an activation control on an entry.

**Owner decisions.** A collapsed Stamina row is enough, with no switch. Druid animal companions are class options. Mythic spells become add-on entries in slice 3.

## Class progression table

A class's progression table (a header row that starts `Level` and has `Base Attack Bonus`) reads
as two parts on every width. The core table (`.progression-table`) holds every column except the
spell columns: Level is pinned (`position: sticky`, opaque in both themes, a 1 px edge and a short
shadow on its right), and it never wraps. An iterative attack bonus (`+11/+6/+1`) breaks after
each slash. At 390 px Level, Base Attack Bonus, the saves and Special fit with no sideways scroll;
Special wraps. A class with extra printed columns (Monk) scrolls sideways; the right edge shows a
strong shadow that the scroll position removes at the end, built with CSS only. Spells per day is
a native `details.progression-spells` below the core table: a 44 px summary `Spells per day`, a
chevron, closed by default. A non-caster has no disclosure. Do not hide a stat column in a
disclosure on any other table.

## Class summary card

A class page opens with `section.class-card.stat-block`, directly under the metadata line: the spell
statistics card's ledger and left accent rule. Rows, in order: Hit Die, Base attack (`Good · +1 per
level`), Good saves, Skill ranks, Class skills, Alignment, Starting wealth, Spellcasting. A row
appears only with a reliable value, so a prestige class shows four rows and a non-caster has no
Spellcasting. Class skills is a `details.class-skills` with a 44 px summary (`16 skills`) that
opens the names by ability. Spellcasting ends with a `.page-link` to the class's spell list. The
card replaces the printed bold-label paragraphs it shows; it does not repeat them.

On a class or prestige-class page, References and Referenced by are closed `details.ref-fold` rows
at the page end, on every width: a 44 px summary with a chevron, the name and the count. A wide
class page is one column with no side panel. Other entries keep them open, in the side panel.

## Class section switcher

A class page with at least three stops (Overview and two more) has `nav.section-switcher` under the
summary card, in place of the `On this page` disclosure: **Overview · Features · Archetypes ·
Spells**, only the stops the class has (Wizard and Arcanist four, Fighter three). A prestige class
has only Overview and Features, so it gets no switcher and keeps the contents disclosure it has
today (#443 tracks its route to Requirements).

It is a row of jump links, not tabs: Navigation role, `--radius` rectangles of 44 px, equal in rhythm,
the stop last chosen filled with `--accent` and marked `aria-current="true"` (the form of the
spell level links). It is `position: sticky` under the masthead (`top: var(--header-h)`), on an
opaque `--bg` that runs out to the page gutters, with a short fade under its edge. The whole page
stays one document, so Find in page and a screen reader's heading list reach every section. A link
moves the page to the start of its section and focus to that section's heading; the address
records the stop as `?s=features`, `?s=archetypes` or `?s=spells` (Overview has no value), which
makes the stop a history entry. A section starts below the masthead and the bar
(`scroll-margin-top`). On a wide screen a class page that has the switcher is one block column in
document order, because a sticky grid item cannot leave its grid row.

## Class-option powers

Approved by the owner on 2026-11-20 (#537; decision
`docs/decisions/2026-11-20-class-option-power-layout.md`, which also holds the fixtures). The work
runs in this order:

1. An app pull request puts the shared power classifier and validator over typed `Block`s in
   `cast-iron-schema`, with a corpus-check warning and native tests from the fixtures.
2. phynics/cast-iron-data#88 rewrites the powers with a script that mirrors those functions, and
   the warning checks the result.
3. The pin pull request of the migrated corpus turns the warning into an error.
4. The renderer calls the same functions.

Archetypes follow in #540.
On a `class_option` entry, a power reads as printed prose. It sits in a plain `div.power`, with no box, no tint and no rule:

1. The head is one baseline-aligned row. On the left is the name, as a heading: `h2` before the first
   top-level body heading, after it one level below the preceding top-level body heading, capped at
   `h6` (`.power-name`: display face, 1.125rem, weight 700). On the right is
   `Level N`, only from `Gained`, in `--step--1` weight 600 `--ink-muted` with tabular figures and no
   wrap. A prerequisite such as `11th-level oracle` stays a `Prereq:` row and never sets the level.
   The level text has a visually hidden prefix ("Gained at"). A dotted `--line` leader takes the
   space between them and disappears when the name wraps.
2. An optional meta line in the Metadata role: the action in words and the uses, joined by `·`
   (`Standard action · 3 + Intelligence modifier times/day`).
3. The base effect in body type, without its `Passive` or `Ability` label. `Flavor` reads as the
   first sentence.
4. One paragraph per upgrade, with the lead `At 11th level,` in weight 600.
5. The other rows (`Special:`, `Prereq:`, `Replaces:`) keep their bold label.

Powers are not added to `On this page`. The layout applies only to a power that passes the canonical
validation, the same check that the corpus runs on class options. A power that fails it renders as
today's labeled rows.

**One-power page** (a talent, hex or rage power whose page is the power). The body has exactly one
candidate power, it passes the validation, its name is the first block, and the name without its
ability tag is the entry title. The
power head folds into the title. The `h1` stays the entry title, never with the tag or the level.
One muted `.entry-facts` line sits directly under the metadata line: the tag in words with the
printed tag, `Level N` from `Gained`, then the action and the uses
(`Supernatural (Su) · Swift action · Once per rage`). The metadata line keeps kind and book. Only
the name and the facts move. The body starts with the optional `Flavor` paragraph (without its
label), then the effect, with no power heading and no power meta line. Auspicious Mark keeps its
`Flavor` paragraph.

## Tokens

Colors: `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--line`, `--accent`, `--accent-ink`, `--focus`, `--error`. Type: `--font-text`, `--font-display`, `--step--1` to `--step-3`. Space: `--space-1` to `--space-6`. Shape: `--radius` (6 px) for rectangles; 999 px only for filters. Touch size: `--touch`.

The stat block keeps its left accent rule; it is part of the identity.
