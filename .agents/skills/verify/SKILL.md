---
name: verify
description: Use after changing the UI, the installer, the service worker, search, or the content, to prove the change works in the real app built into dist/.
---

# Verify a change in the real app

Tests alone do not prove a user-visible change. Build the site, run the acceptance suite, and look at the app.

## 1. Build

From the repository root:

```sh
cargo xtask build-site --public-url /cast-iron/
```

This fetches the pinned corpus and compiles the library into `generated/content/` when its manifest is missing, builds the WebAssembly app with Trunk, and writes `dist/`. Run `cargo xtask check` as well before you finish.

If compiler code, schema types, or the corpus pin changed, run `cargo xtask fetch-sources` and `cargo xtask content` first. An existing manifest suppresses content compilation during the site build. Check that `generated/content/manifest.json` and `dist/content/manifest.json` match before testing. Tests load corpus expectations from `generated/` and the browser reads `dist/`.

## 2. Run one spec

Run at most ONE spec: the new spec the change adds or, when it adds none, the one spec that the feature map (section 5) selects for the changed feature. Run no other spec, and not the full suite: broader coverage comes from the full CI run on `main` after each batch of merges (`gh workflow run ci.yml --ref main`), and on the branch only for the rows of "Test scope" in `AGENTS.md` that ask for it. Docs and xtask changes need no browser run. On a shared host, the full suite takes 12–22 minutes for Chromium and 45–60 minutes with WebKit, and load alone makes tests time out. See "Test scope" in `AGENTS.md`.

```sh
npm --prefix tests/e2e ci
npm --prefix tests/e2e test -- --project=chromium tests/<spec>.spec.ts
```

On a host without the Nix dev shell, install the browsers once first: `npx --prefix tests/e2e playwright install chromium webkit`.

To run one test, pass a name pattern:

```sh
npm --prefix tests/e2e test -- --project=chromium --grep "<name>"
```

When the spec is listed in the `mobile-chromium` Playwright project (`playwright.config.ts`), also add `--project=mobile-chromium`. Add `--project=webkit` on the same specs when the change touches the service worker, installation, storage, or offline behavior.

Report any failure with its output. Do not call a change done while a test fails. When a test fails only during a run under load, run its spec again alone before you change code.

The default command without project flags includes Firefox. Its local startup failure and the informational CI browser projects are recorded in the decisions "Firefox CI disabled" and "CI browser shards and early cache saves" (`docs/decisions/README.md`). Report the specs and projects actually run; a passing Chromium run does not verify WebKit or Firefox.

## 3. Drive the app by hand (new features)

A new feature needs a check that a user can follow. Serve `dist/` and open it:

```sh
node tests/e2e/server.mjs
```

Then open `http://localhost:4318/cast-iron/`. Set `E2E_PORT` to use another port.

Use a real browser. Click through the feature the way a user does, and read the page after each step.

## 4. What to check

- The browser console shows no errors.
- After the app is installed, it works offline. Set the browser to offline and reload.
- Exercise new relationship, spell-list, and book routes offline. `relationships.spec.ts` installs first, waits for service-worker control, then disables the network for each test.
- Take screenshots in both themes (light and dark) at 390 px and 1280 px wide. Open each screenshot and look at it. Check layout, text overflow, and contrast.
- When a page's initial size changes, measure it locally: record the initial rendered text length, DOM node count, and time from reload to the main content, in Chromium and WebKit at 1280 px and 390 px. The numbers are local evidence for `docs/status.md`.

## 5. Feature map

Find the row for the change. Run the e2e tests in that area with `--grep`, and read the listed source files.

| Feature | `--grep` pattern (spec file) | Source files |
|---|---|---|
| Search results, ranking, and offline search | `Search operates\|Equal relevance\|cold index\|catch-all\|Desktop results page` (`search.spec.ts`) | `crates/search`, `crates/web/src/views/search.rs` |
| Search filters, chip counts, and the Publication filter | `Filter chips\|Filter menus\|Chip counts\|Publication filter\|Publication name box` (`search.spec.ts`) | `crates/web/src/views/search.rs`, `crates/web/src/views/scope.rs`, `crates/web/src/views/sources.rs`, `crates/web/src/route.rs`, `crates/content-compiler/src/pipeline.rs` |
| Search and inactive books | `hidden matches\|inactive first source` (`search.spec.ts`) | `crates/web/src/discovery.rs`, `crates/web/src/views/search.rs` |
| Scoped Quick search and kind-list scopes | `scope` (`search.spec.ts`) | `crates/web/src/views/quick_search.rs`, `crates/web/src/views/scope.rs`, `crates/web/src/views/search.rs`, `crates/web/src/route.rs` |
| Spelling corrections | `correction` (`search.spec.ts`) | `crates/search`, `crates/web/src/views/search.rs`, `crates/web/src/views/quick_search.rs` |
| Book section search results and saved section and Contents addresses | `book section` (`search.spec.ts`, `mobile.spec.ts`) | `crates/schema/src/books.rs`, `crates/content-compiler/src/pipeline.rs`, `crates/content-compiler/src/legacy_sections.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/book.rs`, `crates/web/src/library.rs`, `crates/web/src/route.rs`, `sources/legacy-sections.json` |
| Installer, integrity, recovery | `installs completely\|integrity\|Interrupted\|install screen` (`install.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/content_cache.rs`, `web/sw.js` |
| Offline launch, no runtime server | `Offline launch\|runtime server` (`offline.spec.ts`) | `web/sw.js`, `crates/web/src/app.rs` |
| Entry rendering | `complex formatting` (`reference.spec.ts`) | `crates/web/src/views/rich.rs`, `crates/web/src/views/entry.rs` |
| Footnotes: the Notes group and its markers (#507) | whole file (`footnotes.spec.ts`) | `crates/web/src/views/footnotes.rs`, `crates/web/src/views/rich.rs`, `crates/web/src/views/entry_contents.rs`, `web/styles.css` |
| Cross-references and history | `Cross-reference\|Back/forward` (`reference.spec.ts`) | `crates/web/src/route.rs`, `crates/web/src/views/rich.rs` |
| Body-link backlinks ("Referenced by") | `Referenced by\|Show all` (`backlinks.spec.ts`) | `crates/content-compiler/src/backlinks.rs`, `crates/web/src/views/backlinks.rs`, `crates/web/src/library.rs` |
| Home categories and class categories | `Home categories\|category-only\|class categories` (`journeys.spec.ts`, `reference.spec.ts`) | `crates/web/src/views/browse.rs` |
| Kind list pages, letter groups, and filter menus | whole file (`browse-groups.spec.ts`: letter groups, feat type, item category, creature type and CR and spell school menus, book-filtered lists) | `crates/web/src/views/browse.rs`, `crates/web/src/views/filter_menu.rs`, `crates/web/src/views/categories.rs`, `crates/web/src/route.rs` |
| Books page, book pages, chapter pages (`In this chapter`), off line and reading footer eyebrow, and contents tree | whole file (`books.spec.ts`) | `crates/web/src/views/browse.rs`, `crates/web/src/views/book.rs`, `crates/web/src/views/book_find.rs`, `crates/web/src/route.rs` |
| Books tab opening speed | `Books tab opens` (`books-open.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/browse.rs`, `crates/web/src/discovery.rs` |
| Adventure Path, Module, and Society series lines | `Adventures\|Modules\|Pathfinder Society\|bulk toggle` (`adventures-series.spec.ts`) | `crates/web/src/views/book_series.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/active_books.rs` |
| Class summary card and folded class references | whole file (`class-card.spec.ts`) | `crates/web/src/class_summary.rs`, `crates/web/src/views/class_card.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/backlinks.rs` |
| Class page section switcher | whole file (`class-sections.spec.ts`) | `crates/web/src/class_sections.rs`, `crates/web/src/views/section_switcher.rs`, `crates/web/src/route.rs`, `crates/web/src/app.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/rich.rs`, `crates/web/src/views/related.rs`, `web/styles.css` |
| Archetype list and compatibility filter on a class page | whole file (`class-archetypes.spec.ts`) | `crates/web/src/archetypes.rs`, `crates/web/src/views/archetype_list.rs`, `crates/web/src/views/filter_menu.rs`, `crates/web/src/views/rich.rs`, `crates/web/src/views/related.rs`, `crates/web/src/views/entry.rs`, `web/styles.css` |
| Console guard: window for WebKit's "Context is stopped" teardown error | whole files (`teardown-window.spec.ts`, no browser; `console-guard.spec.ts`, WebKit only) | `tests/e2e/tests/fixtures.ts`, `tests/e2e/tests/teardown-window.ts`, `docs/decisions/2026-11-17-webkit-context-stopped-at-teardown.md` |
| Races, racial traits, classes, and archetypes | `race racial traits` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/graph.rs` |
| Class spell lists and source filters | `spell lists filter\|numbered spell series` (`relationships.spec.ts`) | `crates/web/src/views/spells.rs`, `crates/web/src/views/search.rs` |
| Class option groups on class pages and lists | whole file (`class-options.spec.ts`; also `mobile-chromium`) | `crates/web/src/class_options.rs`, `crates/web/src/also_available.rs`, `crates/web/src/views/related.rs`, `crates/web/src/views/categories.rs`, `crates/web/src/views/entry.rs` |
| Class-option powers: head, meta line, order, one-power page, excluded blocks (#537) | whole file (`class-option-powers.spec.ts`) | `crates/web/src/powers.rs`, `crates/web/src/views/rich.rs`, `crates/web/src/views/entry.rs`, `crates/schema/src/power.rs`, `web/styles.css`, `DESIGN.md` |
| Relationship sections, folded References and Referenced by, capped relationship lists (#572, #573) | whole file (`relationship-sections.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/views/backlinks.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/util.rs`, `web/styles.css` |
| Feat prerequisites and chains | `feat pages\|category hub\|every feat chain` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/views/chains.rs` |
| Book contents and reading order | `book contents\|book bar` (`relationships.spec.ts`) | `crates/web/src/views/book.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/graph.rs`, `crates/web/src/reading.rs` |
| Revised, removed, and merged feats (Elephant revisions) | `Elephant\|revis\|merge\|remove` (`revisions.spec.ts`) | `crates/web/src/revisions.rs`, `crates/web/src/views/related.rs`, `crates/web/src/views/chains.rs`, `crates/web/src/views/search.rs` |
| Add-on Changes section | `Elephant` (`changes.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/views/util.rs`, `crates/web/src/graph.rs` |
| Mythic Adventures add-ons | `Mythic` (`mythic.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/related.rs`, `crates/web/src/views/quick_rows.rs`, `crates/web/src/views/util.rs`, `crates/web/src/graph.rs` |
| Pathfinder Unchained Stamina add-ons | `Stamina\|Unchained\|Core only` (`stamina.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/related.rs`, `crates/web/src/views/util.rs`, `crates/web/src/graph.rs` |
| Spell variants | `variant` (`variants.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/related.rs`, `crates/web/src/views/source_variants.rs`, `crates/web/src/views/quick_rows.rs` |
| Publication choices and list rows that name their parent (#575) | whole file (`source-variants.spec.ts`: the choice in browse and search, racial trait, affliction and archetype rows with their parent, the `Alternate` marker, fold links that never read the same, creatures that still fold) | `crates/web/src/views/source_variants.rs`, `crates/web/src/row_parents.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/related.rs` |
| Entry header: metadata line, parent and facts line, star, and chip roles | `metadata line\|star is\|Chips keep\|Keyboard focus is visible\|A label has no border\|blank band between the header\|A feat names\|A trait names\|An archetype names\|A racial trait names` (`entry-header.spec.ts`) | `crates/web/src/views/entry.rs`, `crates/web/src/views/sources.rs`, `crates/web/src/views/related.rs`, `crates/web/src/class_options.rs`, `web/styles.css`, `DESIGN.md` |
| Archives of Nethys links | whole file (`aon-links.spec.ts`) | `crates/web/src/views/sources.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/book.rs` |
| Entry page `Expanded in` credit line (#460) | whole file (`expanded-in.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/sources.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/revisions.rs`, `web/styles.css`, `DESIGN.md` |
| Expansion credit never counts as membership (#459) | whole file (`summon-monster-expansion.spec.ts`) | `crates/schema/src/books.rs`, `crates/content-compiler/src/pipeline.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/book.rs`, `sources/corpus.toml` |
| Keyboard and subsystem reading | `Keyboard` (`accessibility-reading.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/book.rs`, `web/styles.css` |
| Home stars and legacy saved records | `Home keeps legacy\|More than six starred` (`bookmarks.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/entry.rs` |
| Quick search suggestions, recent searches, and recently viewed | `Quick search\|Recent search\|Recently viewed\|header icon\|highlight\|Suggestions` (`search.spec.ts`) | `crates/web/src/views/quick_search.rs`, `crates/web/src/views/quick_rows.rs`, `crates/web/src/views/recent_searches.rs`, `crates/web/src/views/recent.rs`, `crates/web/src/store.rs` |
| Continue reading per book | `continue point` (`continue-reading.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/views/continue_reading.rs`, `crates/web/src/views/quick_search.rs`, `crates/web/src/views/book.rs`, `crates/web/src/views/entry.rs` |
| Active books and preference recovery | whole file (`active-books.spec.ts`: book visibility, Manage books, presets, new-install default, recovery) | `crates/web/src/discovery.rs`, `crates/web/src/library.rs`, `crates/web/src/store.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/book_find.rs` |
| Recovery messages: unreadable book choices, an unknown or removed book, an unknown book in a list or search link, an unknown entry, Page not found, an unknown chapter, an entry of a turned-off book | whole file (`recovery-copy.spec.ts`; also `mobile-chromium`) | `crates/web/src/app.rs`, `crates/web/src/library.rs`, `crates/web/src/views/book.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/not_found.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/active_books.rs`, `crates/web/src/views/legal.rs`, `crates/web/src/views/diagnostics.rs`, `crates/web/src/views/util.rs`, `crates/web/src/route.rs`, `web/styles.css` |
| Book share codes (Manage books, `#/books/manage?set=`; the old `#/books?set=` link redirects) | whole file (`books-share.spec.ts`; also `mobile-chromium`) | `crates/web/src/book_code.rs`, `crates/web/src/book_share.rs`, `crates/web/src/views/share_set.rs`, `crates/web/src/views/active_books.rs`, `crates/web/src/views/book_find.rs`, `crates/web/src/route.rs`, `crates/web/src/app.rs`, `crates/web/src/library.rs`, `crates/content-compiler/src/book_slots.rs`, `sources/published-books.txt` |
| Home page: stars, teaching line, footer, wide layout | whole file (`home.spec.ts`) | `crates/web/src/views/home.rs`, `crates/web/src/views/bookmarks.rs`, `crates/web/src/app.rs`, `web/styles.css` |
| Legacy ids to canonical ids after a library cutover | `Legacy bookmarks` (`canonical-cutover.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/library.rs`, `crates/web/src/route.rs` |
| Slash opens Quick search | `slash` (`slash-shortcut.spec.ts`; also `mobile-chromium`) | `crates/web/src/app.rs`, `crates/web/src/views/quick_search.rs`, `crates/web/src/views/search.rs` |
| Aliased ids, Level 0 spells, primary sources, and anchors | `aliased entry\|Level 0\|primary-source\|Sources line\|Duplicate heading\|article contents` (`reference.spec.ts`) | `crates/web/src/views/entry.rs`, `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/sources.rs`, `crates/web/src/library.rs` |
| Rule heading links and primary reading | `Rules text search\|Entry reading` (`reference.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/search.rs` |
| Search bar, Home saved entries, and end-to-end research | `Home search\|Home categories\|Books navigation` (`journeys.spec.ts`) | `crates/web/src/app.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/search.rs` |
| Quick reference page, the Table tab, and the Quick reference row in Quick search | whole file (`quick-reference.spec.ts`; also `mobile-chromium`) | `crates/web/src/views/quick.rs`, `crates/web/src/views/quick_search.rs`, `crates/web/src/views/quick_rows.rs`, `crates/web/src/app.rs`, `crates/web/src/route.rs`, `crates/web/src/views/icons.rs`, `web/styles.css` |
| Header Install button and Settings install option | whole file (`install-button.spec.ts`) | `crates/web/src/pwa.rs`, `crates/web/src/views/header_action.rs`, `crates/web/src/views/settings.rs`, `crates/web/js/bridge.js` |
| App-shell update and offline bookmarks | `App update waits` (`shell-update.spec.ts`) | `web/sw.js`, `crates/web/js/bridge.js`, `tests/e2e/server.mjs` |
| Diagnostics, install and start timings, Quick search and results timings, Copy and Clear (`#/settings/diagnostics`) | whole file (`diagnostics.spec.ts`; also `mobile-chromium`) | `crates/web/src/diagnostics.rs`, `crates/web/src/views/diagnostics.rs`, `crates/web/src/install.rs`, `crates/web/src/library.rs`, `crates/web/src/store.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/quick_search.rs`, `crates/web/src/views/entry.rs`, `docs/research/iphone-measurement.md` |
| Route focus move yields to the reader and to a newer navigation (#419) | whole file (`route-focus.spec.ts`) | `crates/web/src/focus_guard.rs`, `crates/web/src/app.rs`, `crates/web/src/route.rs`, `crates/web/src/views/entry.rs` |
| Views that are replaced when rendered again (`into_fresh`) render once while their page is on screen: the star, `Show all`, a section jump and the archetype choices keep the same nodes and focus (#624); the star keeps keyboard focus while it saves, on an entry and on Home (#627) | whole file (`view-identity.spec.ts`) | `crates/web/src/views/fresh.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/related.rs`, `crates/web/src/views/backlinks.rs`, `crates/web/src/views/legal.rs`, `crates/web/src/views/spells.rs`, `crates/web/src/app.rs` |
| Router follows a burst of hash changes, and the panic notice (#468) | whole file (`router-burst.spec.ts`) | `crates/web/src/route.rs`, `crates/web/src/app.rs`, `crates/web/src/lib.rs`, `crates/web/src/panic_notice.rs`, `crates/web/src/views/search.rs`, `web/styles.css` |
| Content updates | whole file (`update.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/views/settings.rs` |
| PWA install metadata | `installable` (`pwa.spec.ts`) | `web/manifest.webmanifest`, `web/index.html` |
| Mobile layout | whole file (`mobile.spec.ts`; the `mobile-chromium` project only) | `web/styles.css`, `crates/web/src/views/browse.rs` |
| Legal and attribution | `Licensing` (`legal.spec.ts`) | `crates/web/src/views/legal.rs` |
| Report a problem | whole file (`report-problem.spec.ts`; also `mobile-chromium`) | `crates/web/src/report.rs`, `crates/web/src/app.rs`, `.github/ISSUE_TEMPLATE/app-problem.yml` |
| Publication scope | none: `python3 scripts/check-publication.py` after building the default | `scripts/check-publication.py`, `docs/research/licensing-audit.md` |
| Content compiler and default library | none: `cargo test -p cast-iron-content-compiler`, CI determinism job (two corpus builds, same bytes) | `crates/content-compiler`, `crates/corpus`, `crates/xtask`, `sources/corpus.toml` |

## Keep this file current

When you add a feature, add its row to the feature map, with its e2e test area and source files. When you rename or move a file listed here, update the row. A feature that is missing from this table cannot be verified by the next agent. `cargo xtask check` reads this table and fails when it disagrees with `tests/e2e/tests` (all `*.spec.ts` files below it). It fails when a spec is not named, when a name matches no file, when a `--grep` pattern is not a regular expression or matches no test, and when a test is reached by no row. In the second column, write each selector followed by its specs in parentheses: `` `pattern` (`a.spec.ts`, `b.spec.ts`) `` or `whole file (`a.spec.ts`)`, with `\|` for `|` in a pattern. A pattern is matched as Playwright's `--grep` matches it: against the spec's file name, its `test.describe` titles and the test title. The check assumes `--grep` is case-insensitive, which Playwright's command line does (it builds the pattern with the `i` flag), so `race` matches `Race`; a spec's patterns need not repeat each case. A pattern must match a test in every spec it lists, and each top-level alternative (`a\|b`) must match a test, so a dead alternative fails the check. A row that has no e2e spec starts its second column with `none`. A test whose title is not a plain string (a template with `${}`, a variable) is listed as unverifiable; use plain titles.

## Record decisions and results

A decision that a later reader cannot derive from code is a new file `docs/decisions/YYYY-MM-DD-<slug>.md` (front matter rules: `docs/decisions/README.md`); no other file changes. A pin, a release or an acceptance result is a new file `docs/status/YYYY-MM-DD-<slug>.md`. `docs/status.md` changes only where its current state changed. Never append to an old entry. `cargo xtask check` fails on invalid front matter, a `superseded_by` that names no file, a README that lists an entry, and a broken relative link. `cargo xtask decisions-index` renders the index into `target/docs-index/`.

## Corpus and schema changes

Save the previously published library's `content/` directory outside `dist/` and `generated/` before rebuilding, so that you can install it in a browser and activate the replacement. Compile the new library with `cargo xtask content`. When the change touches ids, the legacy alias map (`aliases.toml` in the corpus), or books, also resolve every id of the last PSRD release against the new library; `docs/releasing.md` gives the commands and the tag that builds it.

Install the previous published library in a browser, bookmark an entry affected by the change, then activate the replacement. Verify the bookmark and entry before and after activation, then reload offline. Record which schema versions were exercised. A file-level ID check does not prove browser update behavior.

Compile twice and compare all generated files when compiler output changes: `rm -rf generated`, run `cargo xtask content`, hash the tree, and repeat.

Run `python3 scripts/check-publication.py` against the built distribution for corpus changes. It screens `dist/content` by default; pass a generated directory to screen another library. Record actual counts and limitations in the status log: add `docs/status/YYYY-MM-DD-<slug>.md`.

## Browser test expectations

- Derive expected entries and counts from the compiled corpus. Confirm the fixture exists before navigating.
- Scope repeated labels to their region, such as `.entry-rel`, `.toc`, or `.entry-list`. An entry body and its navigation can contain the same title.
- Use exact link names when selecting one entry. A name filter for `fireball` legitimately returns both Fireball and Delayed Blast Fireball on a whole-class list.
- Open a contents disclosure before asserting on its children. Closed branches build their children lazily.
- Follow relation direction from the schema: racial trait to race, archetype to class, and dependent feat to named prerequisite. Parent lists use the inverse index.
- Open the book directory's disclosure before selecting its contents link. Expand the archetype disclosure before selecting a source group or archetype. Read a details element's `open` property as a Boolean; an empty `open` attribute still means open.
- Verify book selection through Back and Forward as well as clicks. A chapter URL change must update the open contents without replacing the page.

## iPhone Simulator checks

The published-site baseline is recorded in `docs/research/iphone-journeys.md`. Keep simulator observations separate from desktop WebKit acceptance and from the current local build. Downloading the rules library, installing a Home Screen web app, and applying a replacement update are different checks. Record each independently.

When device automation tools are available, open the device with its host and device IDs, then use the returned session-bound command for all interaction. A driver error is not an app failure. Record the error and uncompleted steps rather than treating a partial walkthrough as a pass.

## Capture a UI verification batch

After building, run `node tests/e2e/capture-ui.mjs /tmp/cast-iron-ui-check`. It starts and stops its own distribution server on port 4399, installs the library in fresh browser contexts, and records desktop and mobile screens in both themes. It writes the PNGs, `measurements.json` (overflow, alphabet and spell-level target sizes, selected scope colors, console errors) and `manifest.json` (provenance: commit, shell and content version, corpus tag, engine, viewport, theme, settled state, masthead probe). Open representative PNGs to inspect them. `--search-only` limits capture to settled search states.

Its fixtures follow the corpus and the app, not fixed names: the source-variants title comes from the compiled catalog and must render as a disclosure, and the Books page is opened through its tiers and a book row. When a screen changes, fix the script; a failed wait is a stale fixture until proven otherwise.

Conditional views (the storage failure before the reader starts, a pending app-shell update, a pending library update, both together, and the iOS Add to Home Screen guide) have their own script: `node tests/e2e/capture-conditional.mjs <out-dir> [--only=<group>,...]`. It uses port 4398 and the test server's hooks, takes phone (390×844) and desktop (1280×900) in both themes, and labels the iOS frames as WebKit engine simulation, never as device evidence. Results of the first run: `docs/status/2026-11-17-conditional-view-captures.md`.

Wait for both selected scope and result kinds before treating a screenshot as filter evidence. A screenshot taken immediately after a click can still show the previous state. Article contents tests must also click a heading whose URL is already current, since no hashchange event occurs in that case.

The masthead is `position: sticky`. Take viewport shots. A full-page shot of a scrolled page has no masthead at the top of the image, and `getBoundingClientRect()` still says y=0, so a DOM measurement does not prove the frame shows it. `capture-lib.mjs` reads each saved PNG back and warns when the masthead is missing; `node tests/e2e/diagnose-masthead.mjs <out-dir>` reproduces the artifact. Keep capture sets out of the repository: commit the manifest, attach the images to the issue.

## Remaining physical iPhone acceptance

Use a physical iPhone for these checks. Simulator status-bar changes do not disable networking, and browser accessibility trees do not prove VoiceOver output.

1. In Safari, star a known entry and confirm it appears on Home with working open and unstar actions.
2. Install the Home Screen app and download its library if needed. Confirm its library installs independently; Safari and Home Screen storage are separate, so do not expect stars to carry over automatically.
3. Enable VoiceOver through iOS accessibility settings. Traverse primary tabs, the search bar and scopes, book activation controls, source-choice disclosures, article contents, and star controls. Check spoken names, selected/expanded state, focus order, and error announcements.
4. Disable both cellular data and Wi-Fi on the physical device. Force-close and relaunch the Home Screen app. Search for a rule, open a book chapter, follow a source choice, and reopen a star. Confirm that no download or recovery screen replaces the installed library.
5. Restore networking and test shell and library updates separately. Preserve the same star and disabled-book choice across each update, then repeat the offline restart.

Record the actual iOS version, release/build, and outcomes. Do not mark these checks complete from desktop WebKit or a simulator relaunch with networking available.
