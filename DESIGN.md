# Design

Cast Iron keeps one warm, paper-like identity: the palette, type and tokens in `web/styles.css` (`:root`). This file records the rules that later UI work must follow. Product context lives in `PRODUCT.md`.

The file has six layers, from the most general to the most specific: Foundations, Components, Shell, Views, States and Copy. A rule sits in the layer that owns it, so a view names the components it uses and does not restate them.

## Foundations

### Tokens

All tokens are custom properties in `web/styles.css`. The color tokens have a light and a dark value (the dark ones in `:root:not([data-theme="light"])` under `prefers-color-scheme: dark`, and in `:root[data-theme="dark"]`).

- **Colors:** `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--ink-dim`, `--line`, `--border` (the shorthand `1px solid var(--line)`), `--accent`, `--accent-ink`, `--focus`, `--error`, `--error-bg`, `--ok`, `--mark-bg`, `--mark-ink`, `--backdrop` (the `rgba(0, 0, 0, 0.45)` scrim behind a modal dialog: Quick search, the install guide, the panic notice), `--stat-bg`, `--shadow-edge`.
- **Type:** `--font-text`, `--font-display`, `--font-mono`, `--leading` (1.55), `--leading-tight` (1.2), `--step--2` to `--step-3` (see Type scale).
- **Space:** `--space-1` to `--space-6` (0.25, 0.5, 0.75, 1, 1.5 and 2.5 rem).
- **Shape:** `--radius` (6 px) for rectangles; 999 px only for filters. `--touch` (2.75rem, the 44 px target).
- **Layout:** `--measure` (44rem), `--book-column` (52rem, the column of Books, Manage books and the share panel), `--wide` (66rem), `--header-h` (3.75rem), `--nav-h` (3.5rem), `--letter-h` (3rem), and `--switcher-h` (`calc(var(--touch) + 2 * var(--space-2))`), which `web/styles.css` declares in a second `:root` rule beside the section switcher, not in the first block.

The stat block keeps its left accent rule; it is part of the identity.

### Type scale

Body text is `--font-text` at `--step-0` (1rem) with `--leading`. `h1`, `h2` and `h3` use `--font-display`, weight 600 and `--leading-tight`. All sizes are rem, so they scale with Dynamic Type.

| Token | Size | Used for |
|---|---|---|
| `--step--2` | 0.75rem | `h2.notes-title`, spell level counts, a book's place line, a step direction |
| `--step--1` | 0.875rem | `.hint`, `.undo-line`, the Home category note, footnotes |
| `--step-0` | 1rem | Body text |
| `--step-1` | 1.25rem | `h3`; Books group headings |
| `--step-2` | 1.5625rem | `h2` |
| `--step-3` | 1.95rem | `h1` |

Entry body text (`.rich`) is the one size outside the scale: `1.0625rem` with a `1.6` line height.

### Columns

Every page is one column centred in `main`. The column is `--measure` (44rem) by default. A page that holds a wide layout (`.wide`, the search page, an entry) takes `--wide` (66rem). Rules text inside an entry stays at `--measure`. The Books directory, the book tiers, the presets, the share section and the undo line use a literal `52rem` column that has no token yet (`#590`).

### Breakpoints

The breakpoints are literal values in `web/styles.css`; no token holds them. The `47.99rem` queries are the other side of 48rem.

| Breakpoint | What changes |
|---|---|
| 23.99rem (max) | The class section switcher's links tighten their padding |
| 30rem (max) | `main` side padding drops to `--space-3`; stat rows and the progression table tighten; the switcher's bleed shrinks |
| 36rem | Home's category cards and Starred cells go from one to two columns; the book page's Previous and Next steps sit side by side |
| 40rem | A spell row's meta line is one line; the relation lists take two columns |
| 48rem | The bottom bar becomes the header's top nav; the filter menu's rows flow into columns |
| 56rem | The search results page shows its filter panel as a sidebar and hides the `Filters` button |
| 64rem | Home takes three columns; a wide entry becomes the grid with its `sources`, `expanded` and `aon` rows |
| 340 px (container) | The book page's `In this book` rectangles become plain rows and the Contents leader becomes a gap |

The table lists the changes that the rules in this file depend on, not every query of the stylesheet. Other queries follow the reader's settings: `prefers-color-scheme`, `prefers-reduced-motion`, `forced-colors`, `pointer` and `print`.

### Glyphs

Text is never an icon. A chevron, arrow, cross, check or star is an inline SVG (`currentColor`, `aria-hidden="true"`), and the control keeps the accessible name. Every `<summary>` that opens a disclosure draws the same chevron and hides the platform's own marker. The exceptions are the two Collections pickers, which keep the platform marker, and the `+N more` link in a metadata line, which is text. All icons share the 24 px grid and a 1.8 px round stroke. Two separators stay text because they separate rather than act: the muted `·` in a metadata or count line, and the `›` between breadcrumbs (and in the book trail).

## Components

### Chip roles

A small control can mean five different things. Each role has one form, so a reader can tell what a thing does from its shape. Do not invent a sixth form, and do not borrow one role's form for another.

| Role | Use it for | Form | Class | Never |
|---|---|---|---|---|
| Metadata | What the entry is: kind, book | One muted text line. Links are underlined text, separated by `·`. More than one extra book opens from a `+N more` disclosure. | `.entry-meta`, `.meta-link`, `.meta-more` | Pills, borders, a row of boxes |
| Filter | A choice that narrows a list: search kinds, books and facets; browse facets; the scope of a search. Use it for a group whose options and "All" fit one row at 320 px. | The only pill: `border-radius: 999px`. A selected filter is filled with `--accent` and shows a check mark. The count stays. The scope pill shows a cross instead of the check, because pressing it turns the filter off. | `.chip` (checkbox in `search.rs`), `.scope-choice`, `.scope-pill` | A pill for anything else; a display choice such as row density |
| Filter menu | A group too large for one row (feat types, schools, creature types) | A 44 px button with the card radius, a 1 px `--line` border, the text `{Group}: {value}` and a chevron-down icon. A narrowed menu has an `--accent` border and weight 600, with no fill. It opens an in-flow panel of 44 px link rows (label, tabular count, check on the current row, `aria-current`), "All {plural}" first, and a find field above 10 options. | `.filter-btn`, `.filter-panel`, `.choice-list`, `.choice-row` | A pill; a modal, a backdrop or a focus trap; ARIA `menu` |
| Navigation | A jump to another place: recently viewed, shortcuts, spell-list links, search suggestions (Quick search rows: plain link rows with a muted `Kind · Book` line) | A rectangle with the card radius (`--radius`, 6 px) or a plain link row. It may show `aria-current` wayfinding (the current spell level). | `.jump` in `.jump-list`, `.rel-chips li`, `.rel-list` | A pill; a check mark or other filter state |
| Navigation: related page | A link from a list to a related page (Feat chains, Spell lists, `Manage books ›` on Books), in the filter row | Accent text, weight 600, a chevron-right icon, a 44 px hit area, no border. | `.page-link` | A bordered rectangle beside a filter menu: two same-looking shapes with different jobs |
| Label | A short fact about the item next to it: "or", "repeats", "not in this library", `Inactive` on a book row, `Offline` in the masthead. A count next to a heading or a group name is Label text, never a bordered shape. | Small muted text with no border, no background and no pointer cursor. | `.tag`, `.rel-count` | A border, a link, anything that looks clickable |

Rules that apply to every role:

- Interactive parts keep a 44 px target (`--touch`). A text line that stays compact reaches 44 px through a larger hit area (`.meta-link::after`), not through padding.
- Keyboard focus uses the global `:focus-visible` ring (3 px `--focus`). Do not remove it.
- Selected state never relies on color alone: filters add a check mark. Navigation may mark the current page with `aria-current`; it never shows filter state.
- Use `aria-pressed` for a toggle that keeps its place (the star), and `aria-current` for a link to the current page.
- A selected value appears once, in its control. Filter state is route state. Icons are inline SVG with `aria-hidden="true"`, never text glyphs.

### Buttons

Each button form has one job. Do not add a form, and do not use one where another fits.

| Class | Use it for |
|---|---|
| `.btn` | A bordered 44 px action: `Copy code`, `Review`, `Cancel`, `Refresh` |
| `.btn-primary` | The one filled action of a question or panel: `Use this set`, `Download`, `Copy results`, `Reload` |
| `.btn-quiet` | A low-emphasis action inside a page: `Dismiss`, `See all (N)` |
| `.text-btn` | An accent-text action in a row or line: `Undo`, `Done`, a series' `Turn all on` |
| `.page-link` | A link from a filter row to a related page (see Chip roles and Filter menu) |
| `.count-action` | Accent underlined text inside a count line or an empty state: `Show all` |
| `.star-btn` | The icon-only star (see Toggles) |
| `.preset-row` | A full-width 44 px preset row on Manage books |

All of them keep the 44 px target (`--touch`). Use one `.btn-primary` per question or panel.

### Disclosures

A disclosure is a native `<details>` with a `<summary>` that draws the shared chevron (see Glyphs). This is the default: it needs no script, and it keeps Find in page and the keyboard working.

The one exception is a `button[aria-expanded][aria-controls]` inside a heading. Use it only when the group must stay a heading and builds its rows lazily, as the Books tiers and series do (see Books, Groups).

A summary keeps the 44 px target. The Collections pickers are the exception to the chevron (see Glyphs).

### Callouts and status lines

**Callouts.** `.notice` (an accent-tinted box with the word `Notice`) says what a page does or what changed while the page still shows content. `.error` (an error-tinted box with the word `Error`) says that something failed. Neither is a pill or a card.

**Status lines.** A muted line that follows a control or a loading step, with no box:

- `.status`: a live `role="status"` line for loading and progress (`Loading entry…`, the installer's progress text).
- `.hint`: a smaller muted sentence beside a control (`The filters could not be loaded. Search still works.`).
- `.undo-line`: the status line after a change on Manage books, with an accent `Undo`.
- `.share-note`: the muted live line after `Copy code` and `Copy link`.
- `.inactive-books-hint`: the one-sentence hint on the search results page, with a `Turn on` action when an inactive book holds more matches, or a `Remove it` action when a book in the address is missing from the library.

### Toggles

**Star.** The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred cell on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

**Toggle row.** Use it for a stored on/off choice that is saved right away: the rows on Manage books. The whole row is a `<label>` of at least 44 px (56 px with a meta line) that holds a native checkbox (no `role=switch`). It has no chevron and does not navigate. A row has one accessory: a chevron (it opens something) or a checkbox (it toggles something), never both. The checkbox never disables itself while it saves, because that drops keyboard focus; writes queue instead. Class: `.toggle-row`.

### Filter menu

A group with more options than fit one row at 320 px is one filter menu (`views/filter_menu.rs`), not a wall of pills.

- **Button.** `{Group}: {value}`, such as `Type: All types`, 44 px high, `--radius`, a 1 px `--line` border, a chevron-down SVG at the right edge. The visible text is the accessible name. Narrowed: an `--accent` border and weight 600, no fill. Long values ellipsise. While the choices load it reads `Type: Loading…` and is disabled. If they cannot load, the button is hidden and a hint says search still works.
- **Panel.** Non-modal and in the page flow under the filter row: `position: static`, at most `50dvh` with its own scroll, no backdrop, no focus trap, no `<dialog>`, no sheet. A fixed panel with a find field would sit under the iOS keyboard, so the panel scrolls with the page. From 48 rem the rows flow into columns.
- **Contents.** A find field (`Find a type`, shown above 10 options) with a status line (`46 types`, or `No types match "xyz".` and Clear). Then a list of 44 px link rows: `All types` first, then A to Z, each with a tabular count at the right and a check on the current one. Options are links to the filtered route with `aria-current="true"`, not radios and not an ARIA `menu`, because the route is the state.
- **Behaviour.** Opening focuses the find field (the current row when there is no field). Escape closes the panel and returns focus to the button. Choosing a type navigates (a history entry) to the same list with `?facet=field:value`, resets the letter, and focuses the button on the new page. Choosing the current type only closes the panel. Back and Forward walk the filter states.
- **The list reacts in place.** The count line reads `1,324 of 3,583 feats · Show all`; the value is not repeated outside the button. A value absent from the active books keeps its URL, the button shows its default text, and the list says `No Combat feats in your active books.` with `Show all feats` and `Manage books` (a link to `#/books/manage`). Quick search takes the facet as its scope (`Search Combat feats`).

**Multi-select.** A group whose values combine (the schools of a spell list) uses the same button and panel with checkbox rows. The button reads `School: All schools`, `School: Evocation` for one, `School: 2 chosen` for more, and is narrowed (an `--accent` border) from the first. A row is a `<label>` of 44 px with a native checkbox, the label and a tabular count; `All schools` stays first as a link with the same box drawn before it, ticked while nothing is chosen (the box is decoration; `aria-current` carries the state). Activating it while it is current only closes the panel, as the current row of a single-choice menu does; with schools chosen it clears them. A row's accessible name carries its noun (`Evocation, 22 spells`). The route is still the state: `#/spells/Wizard/3?school=evocation&school=conjuration`, the choices in the order made, a spell of any of them stays. A tick is one history entry, so Back and Forward walk the choices, and the page is not rebuilt for it: the panel stays open and the focus stays on the row. Above ten options a find field narrows the rows by label, as in a single-choice menu, and opening focuses it; with ten or fewer, opening focuses the first ticked row, or `All schools`. A level link keeps the chosen schools. The count line reads `45 of 199 spells · Show all`, and `Show all` clears the schools and the name filter. Use it only for a group where a spell can sensibly be in several values; a single-choice group stays a link list.

`.page-link` is accent text with weight 600 and a chevron-right SVG, a 44 px hit area and no border. It sits at the right end of the filter row and leads to a related page (`Feat chains`). `.count-action` is the same accent text, underlined, for an action inside a count line or an empty state (`Show all`).

### Facts list

`dl.facts` is a list of `dt` and `dd` pairs. Each pair is a `div` with a hairline under it, the term in `--ink-muted` in a column of 7 to 10 rem and the value beside it. It holds facts that a reader scans: the installer's `Download size` and `Entries`, the Settings rows, and an entry's `Provenance` disclosure. It is not a table.

### Sheets

A sheet is a modal `<dialog>`. Only two exist; every other panel stays in the page flow (the filter menu is never a dialog).

- **`.install-sheet`:** the iOS install guide, `Add Cast Iron to your Home Screen`. Three steps with an icon each (Share, Add to Home Screen, Add), then a `Done` `.btn`. It is at most 22rem wide, a `--surface` box with the card radius and a dimmed backdrop. Esc and `Done` close it and return focus to the control that opened it.
- **`.panic-notice`:** the notice that a panic shows (`Cast Iron hit an error and may have stopped following your taps. Your library and bookmarks are safe. Reload to carry on.`). It is an `alertdialog` on `--error-bg` with an `--error` border, built from plain DOM outside the reactive tree, with a `.btn-primary` `Reload` that takes focus. Esc does not close it; only a reload does.

## Shell

### Masthead

The sticky header holds the brand, the primary nav (from 48rem; below it the nav is the bottom bar), the header search icon, the header action (Install or Update) and the Offline mark. Its height stays `--header-h` (3.75rem) and every control keeps a 44 px target.

- **Brand.** An original wordmark (#194; decision `docs/decisions/2026-11-21-svg-wordmark.md`): "Cast Iron" in outlined Rozha One (SIL OFL 1.1, `LICENSES/OFL-1.1-Rozha-One.txt`) in `web/icons/wordmark.svg`, with a 4-unit round stroke (of 200 units to the em) added to the path so the hairlines and serifs survive at 390 px. The brand link is 44 px high; the mark inside it is 1.4rem high and 6.455 times as wide, and its left edge is the brand's left edge, so the brand still lines up with the page title. A hanging hole in the capital I was tried and dropped: Rozha's stem is too narrow for it.
- **One file, both themes.** The SVG is an alpha mask (`.brand-mark`), not an image: the paint is `--ink` under a static diagonal oil-slick gradient (blue, violet, teal, amber, each mixed into `--ink` at 30 to 34 %), so the mark takes the light and the dark theme from the tokens. Nothing moves, so Reduce Motion needs no case. In forced-colors mode the paint is `CanvasText`.
- **Name.** The mark is `aria-hidden`; the brand link carries a `.sr-only` "Cast Iron", once. The installer's masthead uses the same two spans.
- **Search stays secondary.** The idle header search icon is a bare 20 px glyph in `--ink-muted` with no fill and no border; hover and focus bring `--ink` and the `--surface-2` fill. It never competes with the mark.
- **Offline mark.** Label text with an offline glyph, never a pill: a 20 px glyph (`views/icons.rs`, `aria-hidden`) and `Offline` in `--step--1` `--ink-muted`, with no border, fill or radius, not focusable. Its sentence `Everything here still works.` is visually hidden text after the word, so the accessible text reads `Offline. Everything here still works.`. Below 64rem only the glyph shows: the word `Offline` is visually hidden too, and the accessible text does not change.
- **Header action.** It has the header search icon's form: a bare 20 px glyph (a download arrow for Install, the update glyph for Update) in `--ink-muted`, no border and no fill at rest; hover brings `--ink` and the `--surface-2` fill with the `--radius` corners; 44 px target; no entry animation. From 64rem its label (`Install`, `Update ready`) sits beside the glyph; below 64rem the glyph shows alone and the accessible name stays on the button (the target is 44 px at every width). The update pip stays. A library update names itself `Library update ready. Open Settings`, as its status does.
- **Limits.** No remote font, no animation, no image of the wordmark in the WASM. The artwork must stay clear of Paizo and Pathfinder marks (`AGENTS.md`, rules 10 and 11).

### Primary navigation

Four tabs in one order, **Home, Books, Table, Settings**, each an icon above or beside its label. Below 48rem they are the bottom bar: one equal column per tab, each at least 44 px wide and `--nav-h` tall (at 390 px, about 97 px wide). From 48rem they are the header's top nav, icon beside label, after the brand. The tab for the current section carries `aria-current="page"`, and a page inside a section keeps its tab: kind lists and spell lists belong to Home, a book and its chapters to Books, Legal to Settings. Table is the die icon (`views/icons.rs`, the set's 24 px grid and 1.8 px round stroke); its page is Quick reference, and nothing else belongs to it. The header keeps the `--wide` column edge from 48rem, so the header is one row at every width from 48rem: four tabs beside the brand, the search icon, the header action and the Offline mark. Below 64rem the header action and the Offline mark are glyph-only and the column gap is `--space-2`; from 64rem they carry their labels.

Which tab a route highlights, today:

| Route | Tab |
|---|---|
| Home | Home |
| Kind lists, Spell hub, Spell lists, Feat chains | Home |
| Books, Manage books (and a share link), Book page, Chapter page, the removed-book Books | Books |
| Quick reference | Table |
| Settings, Diagnostics, Legal | Settings |
| Entry pages | none |
| Search | none |
| Not found | none |

Entry pages, Search and Not found highlight no tab today; `#571` decides whether they should.

### Breadcrumbs

A breadcrumb lists only the pages between a tab destination and the current page. It never names Home, Books, Table or Settings, and a page directly under a destination has none. When no items remain, the `<nav>` is not rendered.

| Page | Breadcrumb |
|---|---|
| A kind's list page | none |
| Feat chains | `Feats` |
| Spell hub | `Spells` |
| Spell list | `Spells › Spell lists` |
| Manage books | none |
| Book page | none |
| Home, Search, Books, Quick reference, Settings, Diagnostics, Legal, Not found | none |
| Entry that is not a Rule | `Home › {Kind} › {parent entries} › {printed section parts}` |
| Rule entry | `Books › {parent entries} › {printed section parts}` |
| Creature entry | `Home › {Kind} › {parent entries}`; a creature's printed taxonomy is never a crumb |
| Chapter page | `Books › {Book} › {Chapter}` |
| The old `#/browse/source/<id>` page, in a library without book pages | `Home` |

Entry pages and chapter pages still name `Home` or `Books`, which this rule forbids. `#571` decides the target; until it lands, do not copy their crumbs. The chapter page is a known conflict with the rule above.

### Page head

A page head is, from the top: the breadcrumb when one exists (see Breadcrumbs), the `h1`, a muted count line on a list page, notices, and the filter row. List pages holds the full order and the sticky parts.

A page's H1 takes `tabindex="-1"` only when the page moves focus to it itself: Home, Manage books, the kind lists, the spell hub and the spell lists. The other H1s (Books, Book page, Chapter page, entries, Search, Quick reference, Settings, Legal, Diagnostics, Feat chains, Not found) are plain; the frame focuses `<main>` for them (see Focus on route change).

### Focus on route change

After each route change the frame scrolls (to the anchor, the section, the offset that Back restores, or the top) and then moves focus to `<main id="main" tabindex="-1">`, so a screen reader announces the new page. A reader's own later focus wins: the page scrolls and leaves focus alone.

The frame leaves focus alone in these cases:

- The first load of the app.
- Search: the page focuses its own field, and an edit from one search route to another moves nothing.
- A change inside the page that is on screen: opening a chapter of the book that is shown, choosing a school on the spell list that is shown, typing in the `Find a book` field of Books or Manage books, and a jump between the sections of an entry page.

Two pages move focus themselves after the frame has settled: Manage books (to its H1, or, when a share link opened it, to the question the link is for) and Settings with `?section=updates` (to the `Updates` section).

The skip link focuses `<main>` without changing the hash.

### Footer

`footer.app-footer` closes every screen that shows the frame, in `--step--1` muted text over a hairline. The first line says `Cast Iron is an unofficial fan project. It is not published, endorsed, or approved by Paizo Inc.` and links `Legal and attribution`. The second holds `Source on GitHub` and `Report a problem`, separated by a muted `·`. `Report a problem` is a plain anchor that opens the content issue form on an entry and the app issue form on any other page, so GitHub is contacted only when it is followed. The installer's footer has the first sentence only.

## Views

### Home

Home is `Your rules reference` as an `h1` (it takes `tabindex="-1"`), then, from the top:

1. A `.notice` when no book is active (`Open Manage books`), and the one-time third-party notice with a `Dismiss` `.btn-quiet`.
2. `Starred` (an `h2`), only once the reader has a star.
3. The category directory: `Categories` (the primary kinds as `.cats` cards: name, count and blurb), `More categories` (the other kinds as quiet cards) and `Shortcuts` (`.jump` links to `Spell lists by class and level` and `Feat chains`, only when the library has spell lists). Counts are totals for the active books, and the line above the cards says so.

Home no longer shows Recently viewed. It holds Starred and the category directory, and the teaching line remains until the first star. Starred shows six entries, then `See all (N)`. Its rows are cells of the same grid as the category cards (`.cats`): one column below 36rem, two from 36rem, three from 64rem, with the same column gap, so each cell has the edges of the card column below it. Each cell keeps its own hairline row: title, muted kind, star. The rows read left to right and then down (the DOM order).

### List pages

A list page is a kind's A to Z list (`#/browse/kind/<kind>`). Feat chains, the spell hub and the spell lists follow the same head. The spell hub (`#/spells`) is the head and the class picker: the count line reads `31 spell lists`, and it has no filter row. A class list (`#/spells/<class>[/<level>]`) is the head, a filter row with `School` (multi-select, below) and the `Class page` `.page-link`, the name field, the level bar (`All levels`, then the levels with their counts: it is not sticky, because up to eleven 44 px targets wrap to two rows at 390 px), a quiet text toggle at the end of the level bar (`Compact rows` or `Wide rows`, a `.text-btn` named for what a press does) and the rows. The name field is route state (`?name=`); typing replaces the history entry rather than adding one. Its count line reads `199 spells` (`26 cantrips` at level 0). Home, entries, Search, Settings, Legal, the Books directory and the book reader are not list pages.

From the top, at 390 px:

1. The masthead (sticky): the brand and the header search icon.
2. A breadcrumb, only when a real parent page exists (see Breadcrumbs).
3. The `h1` (the kind's plural label) and, under it, the count line: `3,583 feats`. They stack and never share a row. The count line names the kind, not "entries". It is a live region.
4. Notices.
5. The filter row (not sticky): filter menus on the left, a `.page-link` on the right, and an open menu's panel under the row. Feats, spells, traits, items, archetypes, class options, racial traits and afflictions have it. Classes and races have no filter: they sit under group headings (see Grouped lists). Creatures have the `Type` and `CR` menus. Racial traits have `Race: All races`, with a find field (`Find a race`) because there are more than 10 races; its labels are the race titles from the catalog and the empty state reads `No Dwarf racial traits in your active books.`. Afflictions have `Type: All types` with the six types (`Poison`, `Disease`, `Curse`, `Addiction`, `Insanity`, `Infestation`), so no find field; the empty state reads `No Infestation afflictions in your active books.`. A library compiled before the `affliction_type` facet hides the menu.
6. The letter bar (sticky, 48 px, `--letter-h`), only above 200 entries. It is the only sticky element below the masthead.
7. The letter heading with its count, then the rows. A row is a title, plus a muted line: the parent, when the kind has one, and the book when a title is printed in several, or the revision's book when the row shows a revision (see Repeated titles).

No lede, no subhead and no explanatory prose sit above the rows. The blurb that describes a category lives on its Home card. Provenance prose goes below the list.

### Repeated titles

A list folds entries into one `{title} · N publication entries` disclosure only when they are printings of one rule: the same kind, title and parent (race, class, option group, creature). Entries with different parents are separate rows whose muted line names the parent first: `{Parent} · {Book}`. A variant link names its book, adding a number when two would read the same; two links in one fold never read the same.

- **The parent.** A racial trait names its race (`Dwarf · Advanced Race Guide`), an archetype its class (`Rogue · Advanced Player's Guide`), a class option its group (`Rage power · Core Rulebook`), an affliction the creature, template or rule it belongs to (`Giant Ant · Bestiary`). An alternate racial trait starts the line with `Alternate` (`Alternate · Dwarf · Advanced Race Guide`), so standard and alternate traits of one race read apart. An entry with no parent (the 41 archetypes whose class is not stated, most creatures, spells and feats) keeps the book alone, and folds by title as before.
- **Narrowed by the parent.** When the list is narrowed by its parent (the `Race`, `Class` or `Group` menu), the parent is the menu's value and the line leaves it out: `Alternate · Advanced Race Guide`, `Core Rulebook`. When the list does not span books (`?source=`) the book is left out: `Dwarf`.
- **A fold names its parent too.** The disclosure carries the same muted `.row-source` text inside its summary, parent only (`Dwarf`, or `Alternate · Dwarf`), so two folds of one title read apart.
- **Variant links.** A link reads `{Book}`; the kind is the list's. If two links would still read the same, a final `(n)` keeps them apart. The parent never tells them apart, because the fold key holds it: every link of a fold shares it. No link names a page: no fold of the corpus has two printings in one book. A panel that is not a list of one kind (the requirements panel of a prestige class) keeps `{Book}, {Kind}`.
- **States.** The list builds its rows once the parent lookup (`relations.json`, `option-groups.json`) has resolved, and shows `Loading…` until then, so no row is built twice and an open fold, the focus and the scroll position never reset. An affliction reads the catalog and needs no wait. A library without the lookup, or one that fails to load, keeps the book alone. A parent whose book is off still names itself in plain text, with no `Inactive` Label in a list row.
- **Order.** Inside a letter group, rows sort by title, then by parent title, then by id.
- **Accessibility.** The row link's accessible name is the title. The muted line follows as text, so a screen reader reads `Attributes, Dwarf · Advanced Race Guide`. No `aria-label` override.
- **Phone and wide.** The muted line wraps under the title when it is long, as class-option rows do.

### Grouped lists

A short kind whose few groups are better seen than chosen is one list under headings, not a filter. Classes are listed under Core, Base, Hybrid, Occult, Unchained, Alternate and NPC, in the order of the books (`views/categories.rs`). A heading is an `h2.letter-head` with the count of its classes; there is no filter row and no letter bar. A group no active book holds shows no heading, and a class without a group is listed last under `Other`, so none is lost. Races are listed under Core races, Featured races, Uncommon races and Other races, the order of the Advanced Race Guide's race chapters. Use a filter menu instead when the groups run to dozens.

### Chain marker

A feat row carries a quiet marker when its feat leads to more than 5 other feats down its chain (the transitive "leads to" relation, each feat counted once). The marker is a small muted inline SVG (a root joined to two feats, `aria-hidden`, with a hover title) after the title, inside the row's link. A visually hidden suffix gives the row its accessible name: `Power Attack, leads to 92 feats`. The threshold is `CHAIN_MARK_MIN` in `views/browse.rs`; with it, 70 of 3,583 feats (2.0 %) are marked. The numbers come from the relations graph, read once when the page opens (`Relations::long_chains`); the rows do not wait for them. Add-ons are not chain followers and never count toward the marker or its number. No other row tag exists.

### Search results page

**Results page controls.** Below the field is one scope row, `.search-controls`: the quick kinds (`All entries`, `Rules text`, `Spells`, `Feats`) are Filter pills in a row that scrolls sideways and never wraps, and the compact `Filters` button stays at its end. The button is the Filter-menu shape (44 px, card radius, 1 px `--line`; an `--accent` border while filters are on) and reads `Filters · N` when N filters are active. Below 56 rem it opens the filter panel in the page flow under the row (at most `50dvh` with its own scroll, no backdrop, no trap, no dialog); opening moves focus into the panel, and Escape or the button closes it and returns focus to the button. From 56 rem the panel is the sidebar and the button is hidden. The breakpoint stays at 56 rem, not the 48 rem of the filter menu's column flow, because a 16 rem sidebar next to the results needs that width; do not move it to 48 rem. There is no Search button, because results are live. The status area follows the row: a small count line and, when an inactive book holds more matches, the one-sentence hint with a `Turn on` action.

**Book sections on the results page.** Entries come first. The matching book sections fold into one Navigation row above the entries, `Book sections · 37 matches`, a native `<details>` in page flow: 44 px, `--radius`, a 1 px `--line` border, label left and a chevron at the end that turns when it opens (never a pill, never a filter state). Its accessible name carries the count. The `<details>` owns the 1 px border and the radius, so the open list sits in one container and the entries start below it. Opening lists at most eight sections (a note says `Top 8 of 37 book sections. Refine the search to see the rest.` when there are more) and keeps focus on the row; the row folds again with Enter or Space. The open state is `sections=1` in the search URL: a toggle replaces the history entry, Back from a section returns with the row open, and a new query or filter folds it. When a query matches only book sections, they are listed open under a `Book sections · 14 matches` heading, because nothing sits below them. The same fold applies at every width.

### Quick search

The header search icon opens Quick search. While the reader types, it lists suggestions; with an empty field it lists a Quick reference row, Recent searches (or the starter searches when none) and then Recently viewed. The empty-state groups are the content of the layer, not a popup, so they stay expanded and Escape closes the layer at once.

- **Rows.** Suggestions and recently viewed entries are plain navigation rows: the title, then a muted `Kind · Book` line, 44 px tall, with no border or pill. The Continue row reads `Continue {Book}` and puts the saved entry's title on the muted line.
- **Quick reference row.** While the field is empty and no scope is set, the first option is `Quick reference`, with the muted line `Conditions · Actions · Combat modifiers`. It is a plain row of the same form, and it opens `#/quick` (the Table tab) like any row opens its page, replacing the layer's history entry. It sits in a group of its own with a screen-reader heading and no visible one. It is a destination, not a search: it is never saved as a recent search and never counts as a search sample in diagnostics. Typing any text hides it; so does a scope, because a scoped layer was opened from a list to search that list. Removing the scope or clearing the text brings it back.
- **Recently viewed.** It shows the stored visits whose ids still resolve through the alias map and whose book is active; a retired entry or an inactive book is hidden. The Continue row is its first row when the reader has a saved place in an active book. Five rows show; `Show more (N)` expands the rest with `aria-expanded`, and `Show fewer` folds it back. Each group has its own `Clear`: Recent searches clears only searches; Recently viewed clears the visits and the continue points. Each recently viewed row also has a remove button, and Delete on the highlighted row does the same.
- **Order and keyboard.** The Quick reference row comes first when it shows, then Recent searches, then Recently viewed. Down and Up move through the groups in order, and Enter opens the focused row. The rows are options in a listbox per group; the field keeps focus and names every group's listbox through `aria-controls`, while `aria-activedescendant` names the focused option.

### Scoped search

On a list page the header search icon opens Quick search already limited to that list. The field reads `Search feats` (or `Search Combat feats` once a facet narrows the list), and a pill under the field names the scope: `Feats` with a cross. The pill is the Filter role and wraps to as many lines as the scope needs rather than being cut (a scope of four schools takes three lines at 390 px), its whole area (at least 44 px) removes the scope, and its accessible name is `Remove scope: Feats`. Removing it searches everything and keeps the typed text. Suggestions and the results page that Enter opens use the scope.

The results page shows the same pill above its category row, with the same name and the same one-tap removal, and its count line names the scope: `12 results in Combat feats · Show all feats` (the control widens to the whole kind and keeps the text), or `18 results in Feats`. The spell hub scopes to `Search spells`. A class list scopes to its class and the chosen schools (`Search Evocation or Conjuration Wizard spells`; several values of one field read joined by "or", a class last). The level is not part of the scope (see `docs/decisions/2026-11-15-spell-hub-class-lists-and-the-school-multi-select.md`). A class-option group is a scope like any facet: `Search Rage powers`, with the pill `Rage powers` (a group label names the kind, so it takes no noun), and the results line `12 results in Rage powers · Show all class options`. A racial-trait race reads `Search Dwarf racial traits`. An affliction type reads `Search Disease afflictions`. The search filter panel offers `Group` for class options and `Race` for racial traits and `Type` for afflictions. A scope is derived from the page that is open at that moment: on any other page the icon opens an unscoped Quick search.

A recent search stores the scope it ran in (the kind and any facet), not only its text. The row shows the text and a muted scope label (`grapple` and `Feats`); tapping it opens the results in that scope. The same words in another scope are another recent search. A search over everything stores no scope, and the starter searches take the scope of the page.

Focusing a field never navigates. The scope control is never a second field in the header.

### Quick reference

`#/quick` (`views/quick.rs`) is the Table tab. It is an `h1` and four groups, each an `h2.sub-head` over a `.jump-list` of `.jump` chips: **Conditions** (A to Z), **Actions**, **Combat modifiers**, **Every session**. The groups are one Rust constant (`GROUPS`): a label, an entry id and, for a few chips, a `sec-` heading anchor inside the entry. The page copies no rules text. A chip opens the entry (and scrolls to the anchor when it has one), so the rule is read where it is maintained. A chip whose id is not in the library is not drawn, and a group with no chip left is not drawn. Chips follow the chip roles above: navigation, a rectangle with the card radius, never a pill, never a count. There is no chip row on Home; Starred is the personal list.

### Entry page

`#/entry/<id>` is the entry page. From the top at 390 px, the shipped order is:

1. The breadcrumb (see Breadcrumbs).
2. The head: the `h1` with the star beside it, the metadata line (kind, the parent where there is one, book, an add-on's `Variant of …` fragment), and the muted `.entry-facts` line where the entry has typed facts to classify it (see Class page, Parent and classification).
3. Notices: a book that is turned off, a revision in force, and a collision of revisions.
4. The original-text disclosure of a revised entry.
5. On a class page, the summary card and the section switcher, and `On this page` where the page keeps it.
6. The body, ending in the footnotes group.
7. The `Sources:` line, the `Expanded in:` line and the Archives of Nethys line.
8. The relation panels (see Attached entries for the add-ons, and the related panels for the rest).
9. The reading footer, when the entry was opened from a book (see Books, Reading footer).
10. `References` and `Referenced by`.
11. The entry footer: each distinct copyright notice, the licence sentence with `Legal and attribution`, and a closed `Provenance` disclosure.

The sections below are the parts of the page that have rules of their own.

#### Attached entries

An add-on (variant, mythic version, Stamina rules, a third-party change or revision) is its own entry from its own book. It appears only while that book is active. An inactive base never hides it. Search, lists, facets, backlinks, stars and Recent already work per entry, so an add-on needs no second filter model. There is no rule-set switch and no gated body block.

**The add-on's own page.** The metadata line ends with one link fragment: `Variant of {base}`, `Mythic version of {base}`, `Stamina rules for {base}`, `Changes {book} makes to {base}`, or `Revision of {base}`. The last appears only on the revision entry's own page. There is no `Base …` stat row; a value appears once. Search rows read `Mythic · {book}`, `Stamina · {book}`, or `Changes · {book}`; a class option reads `{Group singular} · {book}`. The page has the entry breadcrumb (see Breadcrumbs).

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

**Class page.** `Class options` is a `.jump-list` of `.jump` rectangles `{Group} {count}`, each opening `#/browse/kind/class_option?facet=group:{slug}`. The list page has the H1 `Class options`, a count line, and a group filter menu that reads the group facet from the search index (`option-groups.json` gives its labels); a class `.scope-pill` appears only when a group serves several classes. Archetype rows show a muted `Replaces {items}` from `replaces`. Option rows show only `{book}`, because the group is the selected filter.

**Parent and classification.** An entry that belongs to a parent (a class option, an archetype, a racial trait) names it in the metadata line, as a link. Typed facts that classify the entry (feat types, a trait's detail, an archetype's replaced and altered features, a one-power option's tag) form the `.entry-facts` line under it. No end panel repeats them.
- A feat reads `Feat · {book}` with its types in the facts line (`Combat`, `Combat · Teamwork`).
- A trait reads `{Category} trait · {book}`; the kind link opens the Traits list filtered to that category. The trait's detail (`Half-Elf`) is its facts line. A trait with no category keeps `Trait`.
- A racial trait reads `Alternate racial trait` or `Standard racial trait` (a link to the Racial traits list), then its race as a link, then the book. A race whose book is off keeps its link and ends with the `Inactive` Label.
- An archetype reads `Archetype · {Class}`, one link per class, then the book. Its facts line is `Replaces {names} · Alters {names}`, the text of its row in the class's Archetypes list; the line is absent while the library has no trades for it.

**A revision in force.** A `.notice` reads `Revised by {book}. Changed: {fields}.`, or `Removed by {book}.` / `Merged by {book}.` followed by the printed sentence. Each changed field row carries the Label `changed` (plain muted `.tag`, never accent or bold), then a collapsed `details.rel-disclosure` `Original text · {book}` with a 44 px summary; its body ends with the base's own `Sources` line. The page's `Sources` line follows the revision's text: a revision with typed rows replaces the base citation, and a revision with none keeps the base's rows. A removed or merged page shows the revision's citation under the printed sentence. A list row adds the revision's book as its muted line. Removed feats leave the list, and one prose line under it states the count. When two or more revisions are in force the page shows the original plus `Revised by` and a `.rel-list`; there is no substitution and no last-writer-wins.

**Lists and chains.** A mythic feat is its own row sorting right after its base (`Dodge`, `Dodge, Mythic`); there is no `+ Mythic` row tag. The `Leads to` panel and the chain marker ignore add-ons. Under a revision, the chain shows the revision's prerequisites with a revised notice. The chain-marker figures are measured again when the corpus gains add-ons.

**Never:** pills (the `.scope-pill` stays the only one here), cards, coloured borders, row tags, inline diff colours, or an activation control on an entry.

**Owner decisions.** A collapsed Stamina row is enough, with no switch. Druid animal companions are class options. Mythic spells become add-on entries in slice 3 of #250.

#### Footnotes

A footnote is never rules text, so it must not look like it (#507). On an entry page the notes leave the flow of the text and sit in one quiet group at the end of the body, `aside.notes` (a complementary landmark named by its `Notes` heading). The corpus keeps a note as a level-six heading with a `footnote-` anchor plus the one paragraph after it; `views/footnotes.rs` classifies that pair from the typed nodes, so the serialized body does not change. Only that paragraph belongs to the note: a monster's note sits inside its stat block, and the lines after it stay in the body.

- **Group.** A hairline `--line` rule above it, then an `h2.notes-title` in small uppercase muted type (`--step--2`, weight 600). The group is always open: a disclosure would hide text that readers look for after they follow a marker. It is the last child of `.rich`, never in the contents list ("On this page" lists sections only), and it has no card, fill or accent border, so it never competes with a stat block or a quote.
- **Type.** Notes use `--step--1` in `--ink-muted` with a `1.5` line height, below the `1.0625rem` rules text. Sizes are rem, so they scale with Dynamic Type. The label (`1`, `B`) is `--ink-dim`, weight 600, tabular, in a fixed column; a screen reader hears `Note 1`. The note's DOM id is the id its heading would have had (`h-footnote-1`), so an old link to `#footnote-1` still lands on it.
- **Markers.** A marker (`^1^`, the `^B^` after a monster's bonus feat) in a paragraph becomes `sup.fn-ref`, a quiet annotation and not part of the word before it (#517): `max(0.7em, 0.6875rem)`, so smaller than the text with a floor for small stat lines; `--ink-muted`, the colour of the notes; weight 400 and no underline, so it never reads as a bold or accented part of a feat name. It is a link to its note. Hover and keyboard focus bring back `--accent` and an underline, and the focus ring stays. The accessible name says what the mark is: `Note 1`, and `Bonus feat, note B` for a `B` whose note reads `Bonus feat.` (the note decides, because other books use `B` for other things). The same treatment applies to every marker, number or letter, linked in the corpus or not, because a footnote mark is one kind of thing. The hit area is an invisible `::after`, so it takes no space: about 29 px tall and as wide as the comma after it. It grows up, down and to the right, never left onto the feat name, and on a touch screen (`pointer: coarse`) it is about 36 px tall. A linked marker follows its explicit anchor. A bare marker links to the first note with its label that follows it, or to the last one before it. A number straight after a digit (`10^2^`) is an exponent, not a marker. A marker in a table cell or a stat row is the same link (#534): the same size, colour, name and back link, and the note lists its back links in the order the page renders in. A marker that cites no note of the entry, such as an exponent, stays a plain superscript. A way back to a marker inside a closed disclosure (the spell columns of a class progression) opens the disclosure first.
- **Way back.** Each note ends with one icon-only link per marker that cites it (`Back to note 1 in the text`). The icon is small and the hit area reaches 44 px through `::after`, so a note stays compact. A note whose marker is missing from the text (a common case) shows its label and text and no way back.
- **Never.** Do not render a note as a heading, in rules-text size or colour, or inside a stat block. Do not put `h6` footnote headings in the contents. Do not size the group with `px`.

#### Class progression table

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

#### Relationship sections

Relationship panels are sections, not cards: an `h2` at `--step-1`, hairline or chip rows, a muted hint under the heading when the relation needs one. They follow the add-on sections in the same stack and the same unboxed form. A feat's `Requirements` lists the feats it names; the printed prerequisite line stays in the body and appears once. A feat whose body prints no `Prerequisites` line keeps `As printed:` in the panel, so no text is lost, and a feat that names no active feat and prints its line in the body has no panel.

A count beside a panel heading is Label text inside the heading (`Leads to 61`), so the heading's accessible name keeps it. A relationship list (`Leads to`, `Racial traits`) shows 8 rows, then `Show all {N}` in the Referenced-by form; `Racial traits` caps each of its groups.

#### Class summary card

A class page opens with `section.class-card.stat-block`, directly under the metadata line: the spell
statistics card's ledger and left accent rule. Rows, in order: Hit Die, Base attack (`Good · +1 per
level`), Good saves, Skill ranks, Class skills, Alignment, Starting wealth, Spellcasting. A row
appears only with a reliable value, so a prestige class shows four rows and a non-caster has no
Spellcasting. Class skills is a `details.class-skills` with a 44 px summary (`16 skills`) that
opens the names by ability. Spellcasting ends with a `.page-link` to the class's spell list. The
card replaces the printed bold-label paragraphs it shows; it does not repeat them.

On a class or prestige-class page, References and Referenced by are closed `details.ref-fold` rows
at the page end, on every width: a 44 px summary with a chevron, the name and the count. A wide
class page is one column with no side panel. Below 64 rem every entry folds them the same way. From
64 rem an entry other than a class shows them open in the side panel.

#### Class section switcher

A class page with at least three stops (Overview and two more) has `nav.section-switcher` under the
summary card: **Overview · Requirements · Features · Archetypes · Spells**, only the stops the class
has (Wizard and Arcanist four, Fighter three). Requirements starts at the top-level `Requirements`
heading, so a prestige class has Overview, Requirements and Features and gets the bar (#443): its
introduction can run past 2,000 px at 390 px before the Requirements. The bar replaces the
`On this page` disclosure on base classes, whose list was 40 or more headings. A prestige class keeps
its disclosure (where it has four headings or more) beside the bar, below it, because the disclosure
reaches one feature and the bar does not. A page with only Overview and Features has no bar.

It is a row of jump links, not tabs: Navigation role, `--radius` rectangles of 44 px, equal in rhythm,
the stop last chosen filled with `--accent` and marked `aria-current="true"` (the form of the
spell level links). It is `position: sticky` under the masthead (`top: var(--header-h)`), on an
opaque `--bg` that runs out to the page gutters, with a short fade under its edge. The whole page
stays one document, so Find in page and a screen reader's heading list reach every section. A link
moves the page to the start of its section and focus to that section's heading; the address
records the stop as `?s=requirements`, `?s=features`, `?s=archetypes` or `?s=spells` (Overview has no value), which
makes the stop a history entry. A section starts below the masthead and the bar
(`scroll-margin-top`). On a wide screen a class page that has the switcher is one block column in
document order, because a sticky grid item cannot leave its grid row.

#### Archetype list

A class page lists its archetypes in the Archetypes section, right under the `Archetypes` heading and
before the section's introduction, also on a class that has no section switcher (#170, part D; decision
`docs/decisions/2026-11-22-archetype-list-and-compatibility.md`). It replaces the printed blurbs
(name, `Class:`, `Modifies or Replaces:`, `Description:` for each archetype) and the `Archetypes by
Feature` matrix, and the `Archetypes` panel that used to close the page. The introduction stays.

- **Row.** `li.arch-row` of the browse list: the archetype's name as a link to its own page, the book
  as a muted `.row-source` at the right, and under them one muted line, `Replaces Arcane bond,
  Cantrips`, with `Alters …` after a `·` when the archetype alters features. The line carries the
  printed names. A row opens the archetype and shows no text inline. The list is one column at every
  width; the rows keep the browse list's target (44 px on a touch screen).
- **Count line.** `31 archetypes`, or `12 of 31 archetypes · Show all` once a choice narrows it. The total
  leaves the chosen archetype out. It is a live region, and `Show all` is a `.count-action` button that
  clears both choices and moves focus to the first menu button, because the pressed button goes away
  with the choice.
- **Two filter menus**, the shared ones of "Filter menu" (`views/filter_menu.rs`), unchanged in form:
  `Keep: Any feature` (the multi-select menu: checkbox rows, each with how many archetypes touch the
  feature, a find field above ten rows; the list leaves the archetypes that change any chosen
  feature) and `Combine with: Any archetype` (the single-choice menu: a link row per archetype with
  how many others it fits, a find field above ten archetypes; the list leaves the archetypes that change nothing it
  changes, and not the chosen one). Both together narrow further. Choosing an archetype closes its
  panel and returns focus to the button.
- **State is route state**, as on every list page: `?keep=<anchor>` (once per feature, in the order
  chosen) and `?with=<archetype id>` beside `a`, `book` and `s`. A choice is a history entry, Back
  undoes it, a shared link opens filtered, and the page is not rebuilt for it (the frame leaves scroll
  and focus alone). A stop of the section switcher keeps the choices.
- **Trades only.** Compatibility is worked out from the trades recorded for each archetype and from
  nothing else, not from the printed matrix or the text; while a choice is active a hint under the
  count line says so. The gaps in the data are tracked in phynics/cast-iron-data#173.
- **Not checked.** A row whose trades the data cannot place (a trade that names no class feature, or
  no trades row) is never counted as compatible. While a choice is active it is listed apart under a
  `Not checked` heading with a hint that says what is missing, never among the compatible rows. When the
  chosen archetype cannot be checked, nothing is compatible: every other row stands under `Not checked`.

#### Class-option powers

Approved by the owner on 2026-11-20 (#537; decision
`docs/decisions/2026-11-20-class-option-power-layout.md`, which also holds the fixtures and the delivery plan). `crates/web/src/powers.rs` plans the layout and `views/rich.rs` renders it.
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
5. The other rows (`Special:`, `Prereq:`, `Replaces:`) keep their bold label, each with the
   paragraphs that continue it. `Choice` and `Info` keep their label too and stand before the
   effect. A power with a passive row and an action row shows the passive text first and the
   action as its own line; the meta line takes the action from the action row's label. `Benefit`
   is the one effect label that stays visible, because nothing else carries it.

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

### Books

Books is where a reader reads; Manage books is where a reader changes which books count.

#### Books head

The H1, one live count line (`aria-live=polite`) that names the matching preset or `Custom` (`28 of 439 books active · Paizo Rulebooks`; `No books active` when none), and, in the filter-row slot, the `Find a book` field and a `.page-link` `Manage books ›`. There is no lede, no hint, no sort control, no group Activate or Deactivate button and no `Only Rulebooks`. The one control besides `Manage books ›` is the `Find a book` field.

#### One order

Six tiers, in this order (`Rulebooks`, `Player Companions`, `Campaign Setting`, `Adventures`, `Other`, `Third party`), and inside a tier the oldest book first, undated books last, ties by title. An old `?sort=` address opens `#/books` and the address bar says so.

#### Find a book

**Find a book** (#587) narrows the groups of Books and of Manage books by name. It is the Filter-menu find field in the form of a page field (a 44 px `input[type=search]`, no new component), with a visible label `Find a book` above it (the placeholder repeats it) and `autocomplete="off"`.

- **Place.** Books: in the filter row under the count line, left of `Manage books ›`. The field is full width at 390 px, and from 48rem at most 22rem wide, beside the link. Manage books: under the count line and above `Presets`; a share link's question stays above the field and keeps the focus a link gives it.
- **Matching.** A substring of the full title or of the short display title (`#1 Burnt Offerings`, a module without `Pathfinder Module:`), without regard to case, accents or punctuation. It never re-sorts: matches keep book order inside each group, and the find never replaces that order with an alphabetical one.
- **Groups.** A tier or series that holds a match is open and lists only its matches; a group with none is not in the page, so it is not in the accessibility tree either. Each visible heading's count reads `3 matches`. What the reader opens or folds during a search is not remembered, and the open groups the reader had come back when the field clears. The count line above the field still counts all books.
- **Status line.** A `p.status` with `role="status"` under the field: `12 books match "bestiary"` (`1 book matches`), or `No books match "xyz123".`, each followed by a `.count-action` `Clear` that empties the field and keeps the focus in it. The line takes no height while the field is empty.
- **Route.** The text is route state: `#/books?q=bestiary`, `#/books/manage?q=bestiary`. Typing replaces the history entry, written on the next animation frame from the route as it is then (a stale field cannot write into another page); no key makes an entry. Back from a book page restores the text and the filtered groups. An old `?sort=` address keeps its handling. A blank text is no search, and the address drops it.
- **Keys.** Escape clears the field. Enter does nothing; the results are live.
- **Manage books.** Checkboxes work inside a filtered view, and the count line follows them. A series bulk button (`Turn all on`) still acts on its whole series, not on the matches only, and its label names the series; the button is a group control, not a control of the result. Typing while a link's question is open leaves the question where it is (it belongs to the history entry) and takes the code out of the address, as an answer does.
- **Page rhythm.** On Manage books the field and its status line are a block of their own above the presets, with `--space-3` above it and no card or tint.

#### Groups

**Groups** (Books and Manage books) are an `h2` in `--font-display`, `--step-1`, weight 700, `--space-5` above, that holds one disclosure `button[aria-expanded][aria-controls]`: a 16 px chevron SVG at the left and a muted, weight 400, tabular count at the right (`31 books` when every book in the group is active, otherwise `28 of 31 active`, including `0 of 151 active`). No fill and no box. Rulebooks starts open and the other groups closed; the open set is kept in memory for the session, so Back to Books finds the same groups open before the scroll position returns. Groups differ from rows by type (serif 700 against sans 600), the space above, the chevron and the right-aligned count; rows have 1 px `--line` rules.

#### Adventures

**Adventures** nests the same disclosure one level deeper (#159): **Adventure Paths** (each path is `Rise of the Runelords`, its volumes ordered by issue number and titled `#1 Burnt Offerings`), **Modules** (one list ordered by publication date, without the `Pathfinder Module:` prefix), and **Pathfinder Society** (each `Season 3` holds its scenarios by number). Publications with no series are a plain **Other** list at the end and are never dropped. Every series block starts open inside the tier, so opening Adventures shows the same rows it did before the grouping; a reader folds a path or a season to shorten the list. A series heading is an `h3` (or `h4` for a path or season) in `--font-display`, `--step-0`, weight 700, with the same chevron and right-aligned tabular count, in the tier's form (`6 books`, or `4 of 6 active`; on Manage books too). No box, fill or border; nesting is shown by the space above. On Manage books each path, `Modules` and each season also shows a 44 px `Turn all on` / `Turn all off` text button; the tier itself still has no bulk control.

#### Book row

**A book row on Books is a link only** (`a.book-directory-row`): the title (sans, weight 600), a muted meta line, and a trailing chevron (inline SVG, `aria-hidden`), at least 56 px. The meta line is a month and year from a fixed English table and then the grouped count (`Aug 2009 · 2,994 entries`; the count alone when the date is unknown). A book that is not active ends the line with the `Label` role text `Inactive`. The row carries no checkbox. Do not hide content behind an inline disclosure in a row: the book page holds the content.

#### Book page

**Book page** (`#/browse/book/<id>`), in this order: the H1 (no breadcrumb), the same meta line as the row, a `Continue reading: {entry title}` link when this book has a saved place, a plain muted line while the book is turned off, `In this book`, `Contents`, and the colophon. The off line reads `This book is turned off. Its entries don't appear in search or lists.` and ends with a `.count-action` link `Manage books` to `#/books/manage`, a plain link that opens Manage books as the link on Books does (focus on its `h1`, no scroll to the book's row, no route parameter); an active book shows no line and no control. Books are switched on or off only on Manage books. `In this book` is an `h2` and a `.jump-list` of `.jump` links reading `{Kind} {count}`, in Home category order, for the kinds the book holds (Rules has no list page). Each opens `#/browse/kind/<kind>?source=<id>`. Below a 340 px container width the rectangles become plain 44 px rows with a hairline, label left and a tabular count right (a container query; at 390 px they stay rectangles). A book page highlights the Books tab; the old `#/browse/source/<id>` address is replaced by it.

#### Contents

`Contents` carries at most one muted regular-weight line (`Chapter order not yet checked against the printed book`) while the book's order is a draft, then one expanded, nested list of chapters and sections. There are no disclosure buttons: a branch row shows its title (a link when the chapter is itself an entry) and a muted tabular count of the entries below it. A top-level chapter row joins the title and count with a dotted leader, as on a printed contents page; the leader is decorative (`aria-hidden`) and becomes a plain gap below a 340 px container width. Contents leaves out every kind that has its own list page, which is every kind except Rules; `In this book` reaches them. A chapter whose entries are all left out is not listed, and only rules chapters and sections remain. The corpus chapter `New entries (page order)` is not shown at all: it is not a Contents row, it has no chapter page, and it is left out of previous and next. A book left with no rows keeps `In this book` and its kind links, and `Contents` then carries one muted line, `Everything in this book is listed above.`, with no review line; when the book has no list-page kind either, the line is `This book has no entries to show.` A section below chapter level that is itself an entry collapses to one row (its link and muted count) when that entry's page already lists every entry below it; the children are not repeated. A corpus-marked node (`collapse = true` in the curated book file) collapses at any level, including a top-level chapter: its row links to the node's entry, or to its chapter page when it has no entry. An unmarked top-level chapter row never collapses: it always opens its chapter page. No row carries a flag or a note about its position. The colophon is a `<footer>` at the bottom: each distinct copyright notice once, then, when the harvest stored the book's Archives of Nethys page, the line `View on Archives of Nethys · Copy link`, then `Legal and attribution`.

#### Chapter page

Every top-level Contents row opens its chapter page, `#/browse/book/<id>/<chapter-id>`; the id is the chapter anchor or a slug of its printed title, never its row number. The chapter page is full width. Its breadcrumb today is `Books › {Book} › {Chapter}`, a known conflict with the Breadcrumbs rule, which `#571` decides. The chapter's first-level sections are `h2` headings and deeper levels are plain indented lists, with no disclosure. A section that is itself an entry and whose page already lists every entry below it keeps only its link. A chapter that is itself an entry offers one link to that entry. The off line (as on the book page, with its `Manage books` link) follows the `h1`, then that link. When the chapter holds entries of kinds that have a list page, an `In this chapter` heading and a `.jump-list` of `{Kind} {count}` links follow, in the form of `In this book`: counted from the chapter's subtree, in Home category order, each opening the book-filtered list (`#/browse/kind/<kind>?source=<id>`). A rules-only chapter has no such section; a turned-off book still shows it. Back from a chapter page restores the book page's scroll position, and Back from an entry returns to the chapter page it came from.

#### Reading footer

An entry opened from a book or a chapter carries `?book=<id>` and ends with the reading footer: `Previous` and `Next in {Book}`, the chapter name added when the step crosses a chapter. Previous and next follow the same order as Contents, so the kinds Contents leaves out are skipped and the catch-all chapter is never a step. The footer appears only when the route carries `?book=`; an entry opened without it (from search, a bookmark or a link) has none, even when a book places it. Today the footer also holds a position bar above the two steps: the eyebrow `In the book`, plus `· draft order` while the book has open order questions (the condition of the review line under `Contents`), and a trail of links from the book to the chapters that hold the entry.

#### Share a book set

**Share a book set** (#408) sits on Manage books (#504), after the presets and before the book groups (its error and question sit above the `Find a book` field, so a link's question is the first thing on the page), and not on Books: a code changes the active set as a preset does, so it belongs where the set is changed, and Books keeps the count line, the filter row (the `Find a book` field and the `Manage books ›` link), the field's status line and then the first group. Its question is scrolled into view when a link or code opens it, because it can sit below the first screen. Each side has its own status line and Undo, and a line describes the books only while nothing has changed them since: any later change that succeeds, from the presets, a checkbox, a group button or this section, ends every Undo line on the page. A change that fails to save changes no line. An Undo that was clicked while another change was still saving does nothing once that change has landed, so it never puts back books that the newer change chose. It is one native `<details class="share-set">` with a 44 px summary in `.page-link` colour and weight (`Share these books or enter a code`) and a chevron that turns when it opens, like the other disclosures. It holds the code in a bordered monospace block that selects whole on a tap (`user-select: all`), two 44 px `.btn` buttons (`Copy code`, `Copy link`) with a muted live line (`Copied the code.`), then the `Enter a code` label, a 44 px field and a `Review` button in one row, and an `.error` line that `role=alert` announces. The page keeps the same hairline rhythm as the presets: no card, no tint.

  - **The question** (`section.share-confirm`, above the `Find a book` field and the presets) is an `h2.sub-head` that takes focus (`Use this set of books?`, or `Same books` when nothing would change), a bold summary (`Turns on 12 books, turns off 3.`), a muted result line in the form of the count line (`Afterwards: 28 of 439 books active · Paizo Rulebooks`, `Custom` when no preset matches), a muted line for books skipped because this library lacks them, and two `.btn` buttons: `Use this set` (`.btn-primary`) and `Cancel`. Opening a link or reviewing a code only asks; the books change when `Use this set` is pressed. The answer writes once, shows the status line that names what it made (`Applied Paizo Everything.`, or `Now using 27 books.` when no preset matches) with an accent `Undo`, in this section's own `.undo-line` under the disclosure, and takes the code out of the address (`#/books/manage?set=<code>` becomes `#/books/manage`). A link made when the section sat on Books (`#/books?set=<code>`) still works: it opens the same question, and the router rewrites its address. The question belongs to the history entry that holds the link: Back or Forward to another entry takes it away, and Forward to the link's entry asks again while its address still carries the code. A save closes only the question it answered: a newer code that the reader reviewed meanwhile keeps its question and its text, and a save that fails does not show its notice next to it. A save that finishes after the reader has left the page changes no screen, and one that finishes while Quick search is open rewrites no address until the reader is back on the link's entry.

  - Buttons here use the existing `.btn` forms and no new one: one filled `.btn-primary` per question (`Use this set`), and bordered `.btn` for every other action (`Copy code`, `Copy link`, `Review`, `Cancel`), all 44 px. The code block is a muted `--surface-2` block with no border and 0.08em tracking, so it is not mistaken for the field below it, and the status line after an apply names the preset (`Applied Paizo Rulebooks.`) or gives the count (`Now using 27 books.`) at the size of the other muted lines. An error from a bad link is an `.error` with `role=alert` above the question, outside the disclosure, which is closed when a link opens the page. An error from a typed code is the same element inside the form. Undo stays until another selection change succeeds; reviewing a code, a rejected typo, Cancel and a failed write leave it alone. A library whose books carry no slot shows none of this.

#### Manage books

**Manage books** is the route `#/books/manage`, not local state: every change is saved at once, so a reload or Back stays on the page and Settings can link to it (the Library section holds the `Manage books` row). The old `#/books/active` address parses to the same page and the address bar writes the new one. The Books tab stays highlighted. The head is the H1 `Manage books` with `Done` (accent text, weight 600, no chevron, 44 px) at its right, then a count line without the preset name (`412 of 439 books active`). Entering moves focus to the H1, except when a share link opens the page: its question takes the focus, because it is what the link is for. `Done` goes Back when the Books link pushed the history entry that is on screen, otherwise it replaces the entry with Books (an entry that a share link pushed later on the same page is one of those); it returns focus to the `Manage books ›` link.

#### Presets

**Presets** (`h2` `Presets`): 44 px button rows with the name on the left, a muted tabular count (`28 books`) and a check mark with `aria-pressed="true"` on the one that matches the active set. No fill, no bold, hairlines between rows. A preset that repeats another preset's set, or has no books, is not shown. `Everything incl. 3rd party` shows, because the corpus marks third-party books (#189). Applying one writes the choice once, shows a muted status line (`Applied Paizo Everything.`, a live region) and an accent `Undo` button. The line takes no height until a change shows in it, so the presets are followed at once by the share section (12 px, no empty band); the share section's own status line behaves the same. Undo restores the previous set and lasts until the next change that succeeds or until the reader leaves the page. A preset's change is worked out inside the write, after any earlier write, so two quick taps end on exactly the second preset.

**Below the presets**, Manage books lists the same groups with a checkbox per book (toggle rows). The only group control is the Adventures series bulk button described under Groups; a tier has none.

### Settings

`#/settings` is an `h1` `Settings` with no count line, then seven `section.settings-section` blocks in a fixed order, each named by its `h2` through `aria-labelledby`. One `--measure` column; the left edge follows the brand from 48rem.

1. **Updates** (`#set-updates`, `tabindex="-1"`). Always first, with or without a pending item (#206); the header action opens Settings with `?updates` and moves focus here.
2. **Appearance.** `Theme` and `Spell row density` as native radios in `.choice` fieldsets.
3. **Books.** A facts row `Active` with the count line of Books (`28 of 443 books active · Paizo Rulebooks`), then a `.page-link` `Manage books`.
4. **Library.** Facts rows `Rules library` (`v0.1.37 (v28481793…)`), `Entries`, `Size on this device` and `Installed`; a `.btn` `Verify library`; its result is a `p.status` with `role="status"` (`Verifying…`, `Library verified. …`, or the failure sentence).
5. **Storage.** Facts rows `Used`, `Available to this app` and `Persistent storage`; `Request persistent storage` is a `.btn` that shows only while storage is not granted.
6. **Install.** One of three states: a facts row `Installed as an app`, the sentence with `Install app`, or the muted hint that the browser menu may offer it.
7. **Help.** A facts row `App version` (`Released 7 Nov 2026 (5ec392ee…)`), then `.page-link` rows `Diagnostics`, `Report a problem` (the footer's link, opens a new tab) and `Legal and attribution`.

**The update rows.** What waits sits at the top of Updates, one `div.update-row` each, library first, with a facts row and one control:

- Library: a facts row `Rules library` / `v0.1.38 · 214 MB`, then the installer panel in its update form: one `.btn-primary` `Download update` and one muted line `Your stars and book choices stay. The current library keeps working until the new one is verified.` The size and the reassurance each appear once. The panel shows progress and failures in place of the button during and after a download, and nothing else (no facts block, no iPhone paragraph). Offline, the panel and the button give way to the muted line `Connect to download it.` A record from an earlier session has no file list yet: the row shows a `.btn` `Check now` and the muted line `Check first, then download it.`, and mounts no panel.
- App shell: a facts row `App` / `Update ready`, then a `.btn` `Reload`.

Below the rows, or alone when nothing waits: the `Check for updates automatically` checkbox as a `.toggle-row`, the muted sentence about the daily check, a `.btn` `Check now` with `Last checked: 21 Nov 2026`, `You are offline. Connect to check for updates.` while offline, and a live `p.status` for the result of a check.

No maintainer words on this page: no schema, manifest or content hash outside the version values; those live in Diagnostics, which holds the content format. Updates is always first; pending items sit at its top.

### Diagnostics

`#/settings/diagnostics` belongs to Settings. It is an `h1` and two paragraphs that say what the page holds (counts, lengths and times, not what was searched for; nothing is sent anywhere), then one row of `.btn` buttons (`Copy results` as the `.btn-primary`, `Refresh`, `Clear diagnostics`), a live status line, one `section.settings-section` per report section with its text in a focusable `<pre>`, and a `Back to Settings` link. While the report is collected it reads `Collecting...`.

### Legal

`#/legal` belongs to Settings. It opens with the `h1` `Legal and attribution` and the contact sentence, then an `On this page` jump list: a `nav` named `On this page` with an `h2.sub-head` and a `.jump-list` of `.jump` links, one for each `h2` of the page in page order (`Community Use notice`, `Sources using the Community Use Policy` and `Section 15 notice pending` when they show, `Product Identity declaration`, `Open Game License`, `Section 15: copyright notices`, `Dataset`, `Coverage`). A link scrolls to its heading and moves focus to it (the headings take `tabindex="-1"`); headings keep their ids. The sections follow in that order, one `--measure` column. No notice is folded away: the Community Use notice and the Product Identity declaration are always open.

Each licence text is a native `details.legal-license` that starts closed, with the shared chevron, a 44 px summary and the licence name as the label; the text stays in the page. The Section 15 lines and the dataset lines are open lists. A dataset line gives the dataset name and its repository; the commit hashes sit in the maintainer details.

Coverage keeps its summary tables open: `Publications`, `Entries by category`, `Excluded from this library`. Everything made for the people who maintain the corpus sits in one closed `details.coverage-more` with the summary `Details for maintainers`, built when it first opens like the other coverage details: the dataset commits, every excluded upstream path, `Rejected records`, `Links between entries`, `Upstream content removed on purpose` and `Relationships, spell lists, and book order`. The excluded material stays listed in the summary tables (AGENTS.md rule 6).

While `legal.json` or `coverage.json` loads, the place of its content shows `Loading…` (`role="status"`). A file that cannot be read shows the page's `.error` with `Open Settings` and the closed `Details for a bug report` disclosure (see Recovery and unavailable pages); the other file's content still shows.

### Installer screen

The installer screen shows when no library is installed yet. It has the masthead (the brand only), then `main.install-main`: the `h1` `Install the rules library`, a lede that says the library downloads once and works offline afterwards, an optional `.notice`, and one `.install-panel`. The panel shows `Library version` (the corpus tag with a short id, `v0.1.37 (v28481793…)`, never the bare content hash), the download size and the entry count as `dl.facts`, a paragraph that starts `On iPhone?` and names the Home Screen install and its separate library, a `Download` `.btn-primary`, and then a progress bar named `Download progress` with a live `.status` line. A failure is an `.error` with `role="alert"` and a `.btn-primary` `Retry` (`Check again` when the manifest must be fetched again); after a failed integrity check the panel adds that verified files are kept, so a retry continues where it stopped. The footer carries the unofficial line without the legal link. The same panel updates a library from Settings; there it shows only `Download update` with one muted reassurance line, then the progress and failure states, with no facts and no iPhone paragraph (see Settings, The update rows). While it reads the file list it shows `Checking the library…`.

### Not yet designed

These views exist and have no design section yet. Each entry names the ticket that writes it.

- Not found and the other recovery states: `#582`
- Search filter panel contents: `#579`

The installer screen is described above under Installer screen, and the iOS guide and the panic notice under Sheets. No ticket follows for those three.

## States

### Loading

Loading text is a `.status` line with `role="status"` (see Callouts and status lines) at the place where the content will appear: `Loading entry…`, `Loading contents…`, `Loading chapter…`, `Loading feat chains…`. A filter menu that is waiting reads `{Group}: Loading…` and is disabled. Starred on Home shows nothing while it loads and nothing when it is empty. The app frame itself shows `Loading Cast Iron…` until the library is ready.

### Empty

A filtered list that has no rows says so in one sentence and offers the way out: `No Combat feats in your active books.` with `Show all feats` and `Manage books` (see Filter menu). The search results page and the spell lists draw their empty state as `.empty`: a 1 px dashed `--ink-muted` box on `--surface` with the card radius. Home with no active book shows a `.notice`, `No books are active. Turn some on in Manage books.`, with Manage books as the link; Manage books itself says `No books are active. Turn some on below.`. When a narrowed list has no rows, its count line reads `0 of 1,885 feats` and drops `Show all`; the block below it offers that link once.

### Recovery and unavailable pages (#445)

A page that cannot show what was asked for follows `Entry unavailable`: an H1 that names the state (`Book unavailable`, never the raw id), then one `.error` with `role="alert"` that says what happened and, where it matters, what was kept (only where the page has no content to show), then one or two links out (`Browse all books`, `go to Home`). A state that still shows content uses a `.notice` instead and says what the page does: the kind list reads `That book isn't in your library, so this list shows all spells.`, and a search with a missing book shows the muted `.inactive-books-hint` line with a `Remove it` action. The book-choice recovery screen puts the alert, one sentence of next steps, and `Try again` (filled) and `Reset book choices` (bordered) together, and keeps the engine's own text under a closed `Details for a bug report` disclosure with the shared chevron. The storage failure screen (a store the browser will not open: a private window or strict privacy settings) has the installer's masthead and column, the `h1` `Cast Iron can't open its storage`, one `.error` with `role="alert"` (`This browser won't let Cast Iron save its library on this device. A private window or strict privacy settings can cause this.`), one sentence of next steps (`Open Cast Iron in a normal window, or allow site data for this site, then try again.`), `Try again` (filled, 44 px, reloads the page) and the engine's text under the closed `Details for a bug report` disclosure. A quota error during a download is not this screen; the installer shows it. The removed-book line above Books is a `.notice`, because the page still shows Books. No message shows an id, a path, a file name, a slot, a schema or a manifest; the engine's text sits under the closed `Details for a bug report` disclosure.

| State | `h1` | Message | Links |
|---|---|---|---|
| Entry unavailable (the link names no entry) | `Entry unavailable`, or the catalog title when the stub exists | `.error`: `This entry isn't in your library. The link may come from another version of Cast Iron.` | `Search for it` (to a search for the last id segment), `go to Home` |
| Entry unavailable (its file is missing or damaged) | same | `.error`: `Part of the library can't be read on this device. Open Settings and verify the library.`, with the details disclosure | `Open Settings`, `go to Home` |
| Not in this library (a retired entry) | `Not in this library` | `.notice`: `This saved entry is not part of the current library ({reason}).` | `Go to Home`, a plain link |
| Page not found | `Page not found` | `.error`: `There is nothing at this address. The link may be old or mistyped.` The address is not printed | `Search the rules`, `go to Home` |
| Chapter unavailable | `Chapter unavailable`, and the document title says the same | `.error`: `This chapter isn't in this book. The link may be old or mistyped.` | `Open the book`, `Browse all books` |
| A book's contents or chapter cannot load | the page keeps its `h1` (a chapter shows `Chapter unavailable`) | `.error`: `This book's contents can't be read on this device. Open Settings and verify the library.`, with the details disclosure | `Open Settings` |

The entry page of a turned-off book shows a `.notice`: `This entry's book is turned off, so it doesn't appear in search or lists.`, then `It stays in your Starred list.` only when the entry is starred, then the link `Manage books`.

### Offline

While the device has no connection the header shows the Offline mark (see Shell › Masthead) at the right end: the glyph and `Offline`, with the visually hidden `Everything here still works.`. Below 64rem the glyph shows alone.

## Copy

### Vocabulary

A book is active or inactive (count lines such as `28 of 443 books active`, the `Inactive` Label, checkbox names `Active {title}`). A reader turns a book on or off, and one book's state reads `This book is turned off.` Never write disabled, enabled or reactivate; stored keys such as `disabled_books` are not copy.

### Ellipsis

Loading and progress text uses the single character `…` (`Loading entry…`, `Type: Loading…`, `Loading Cast Iron…`, `Collecting…`) and is `role="status"`.

### Numbers

Counts group their thousands with a comma through `fmt::count` (`3,583 feats`, `1,324 of 3,583 feats`); there is no locale setting. A count in a line, a row or a list is set with tabular figures (`font-variant-numeric: tabular-nums`) so columns of counts line up.
