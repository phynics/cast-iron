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

This compiles the content when `generated/content/manifest.json` is missing, builds the WebAssembly app with Trunk, and writes `dist/`. Run `cargo xtask check` as well before you finish.

If compiler code, schema types, or source configuration changed, run `cargo xtask content` first. An existing manifest suppresses content compilation during the site build. Check that `generated/content/manifest.json` and `dist/content/manifest.json` match before testing. Tests load corpus expectations from `generated/` and the browser reads `dist/`.

## 2. Run the acceptance suite

```sh
npm --prefix tests/e2e ci
npx --prefix tests/e2e playwright install chromium webkit
npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium
```

To run one test, pass a name pattern:

```sh
npm --prefix tests/e2e test -- --project=chromium --grep "<name>"
```

Report any failure with its output. Do not call a change done while a test fails.

The default command without project flags includes Firefox. Its local startup failure and the informational CI browser projects are recorded in `docs/status.md`. Report the projects actually run; a passing Chromium run does not verify WebKit or Firefox.

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

## 5. Feature map

Find the row for the change. Run the e2e tests in that area with `--grep`, and read the listed source files.

| Feature | `--grep` pattern (spec file) | Source files |
|---|---|---|
| Search and filters | `Search operates` (`search.spec.ts`) | `crates/search`, `crates/web/src/views/search.rs` |
| Installer, integrity, recovery | `installs completely\|integrity\|Interrupted` (`install.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/content_cache.rs`, `web/sw.js` |
| Offline launch, no runtime server | `Offline launch\|runtime server` | `web/sw.js`, `crates/web/src/app.rs` |
| Entry rendering | `complex formatting` (`reference.spec.ts`) | `crates/web/src/views/rich.rs`, `crates/web/src/views/entry.rs` |
| Cross-references and history | `Cross-reference\|Back/forward` (`reference.spec.ts`) | `crates/web/src/route.rs`, `crates/web/src/views/rich.rs` |
| Browsing and class categories | `browsing\|Expanded books` | `crates/web/src/views/browse.rs` |
| Races, racial traits, classes, and archetypes | `race racial traits` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/graph.rs` |
| Class spell lists and source filters | `spell lists filter` (`relationships.spec.ts`) | `crates/web/src/views/spells.rs`, `crates/web/src/views/search.rs` |
| Feat prerequisites and chains | `feat pages\|category hub` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/views/chains.rs` |
| Book contents and reading order | `book contents\|Expanded books` | `crates/web/src/views/book.rs`, `crates/web/src/graph.rs`, `sources/psrd-data/book-order.toml` |
| Portable stars | `stars export and import` (`star-transfer.spec.ts`) | `crates/web/src/star_transfer.rs`, `crates/web/src/views/star_transfer.rs`, `crates/web/src/store.rs` |
| Publication choices | `Source variants` (`source-variants.spec.ts`) | `crates/web/src/views/source_variants.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/related.rs` |
| Keyboard and subsystem reading | `Keyboard` (`accessibility-reading.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/book.rs`, `web/styles.css` |
| Home stars and legacy saved records | `Home keeps legacy\|More than six starred` (`bookmarks.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/entry.rs` |
| Active books and preference recovery | `Book visibility\|No active books\|Invalid book choices` (`active-books.spec.ts`) | `crates/web/src/discovery.rs`, `crates/web/src/library.rs`, `crates/web/src/store.rs`, `crates/web/src/views/browse.rs` |
| Rule heading links and primary reading | `Rules text search\|Entry reading` (`reference.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/search.rs` |
| Search bar, Home saved entries, and end-to-end research | `Home search\|Browse connects\|Books navigation` (`journeys.spec.ts`) | `crates/web/src/app.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/search.rs` |
| App-shell update and offline bookmarks | `App update waits` (`shell-update.spec.ts`) | `web/sw.js`, `crates/web/js/bridge.js`, `tests/e2e/server.mjs` |
| Content updates | `Content update` (`update.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/views/settings.rs` |
| PWA install metadata | `installable` | `web/manifest.webmanifest`, `web/index.html` |
| Mobile layout | `Mobile viewport` (`mobile.spec.ts`) | `web/styles.css` |
| Legal and attribution | `Licensing` | `crates/web/src/views/legal.rs`, `sources/psrd-data/legal-notices.json` |
| Publication scope | none: `python3 scripts/check-publication.py` after building | `scripts/check-publication.py`, `docs/research/licensing-audit.md` |
| Content compiler | none: `cargo test -p cast-iron-content-compiler`, CI determinism job | `crates/content-compiler` |

## Keep this file current

When you add a feature, add its row to the feature map, with its e2e test area and source files. When you rename or move a file listed here, update the row. A feature that is missing from this table cannot be verified by the next agent.

## Corpus and schema changes

Save the previous release's `content/` directory outside `dist/` and `generated/` before rebuilding. Run `python3 scripts/check-library-upgrade.py <previous-content-directory>` after building. It checks old IDs and titles, class classification, and dangling references in the schema-3 indexes. It reports kind changes without failing, so inspect that list separately.

Install the previous published library in a browser, bookmark an entry affected by the change, then activate the replacement. Verify the bookmark and entry before and after activation, then reload offline. Record which schema versions were exercised. A file-level ID check does not prove browser update behavior.

Compile twice and compare all generated files when compiler output changes. Run `python3 scripts/check-publication.py` against the built distribution for corpus changes. Record actual counts and limitations in `docs/status.md`.

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

When T3 Device tools are available, open the device with its host and device IDs, then use the returned session-bound command for all interaction. A driver error is not an app failure. Record the error and uncompleted steps rather than treating a partial walkthrough as a pass.

## Capture a UI verification batch

After building, run `node tests/e2e/capture-ui.mjs /tmp/cast-iron-ui-check`. It starts and stops its own distribution server on port 4399, installs the library in fresh browser contexts, and records desktop and mobile screens in both themes. It writes measurements for overflow, alphabet and spell-level target sizes, selected scope colors, and console errors. Open representative PNGs to inspect them. `--search-only` limits capture to settled search states.

Wait for both selected scope and result kinds before treating a screenshot as filter evidence. A screenshot taken immediately after a click can still show the previous state. Article contents tests must also click a heading whose URL is already current, since no hashchange event occurs in that case.

For AoN navigation mapping changes, run `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/scripts -v`. Regenerate the mapping with the command in `docs/research/aon-mapping-resolution.md`; compare two outputs. Mapping status is evidence and must not be reported as imported content.

## Remaining physical iPhone acceptance

Use a physical iPhone for these checks. Simulator status-bar changes do not disable networking, and browser accessibility trees do not prove VoiceOver output.

1. In Safari, star a known entry. Open Home > Move stars > Export stars and save the file in Files.
2. Install the Home Screen app and download its library if needed. Open Home > Move stars and import the file. Confirm the intended entry opens, a repeated import adds zero stars, and Unstar affects only that entry.
3. Enable VoiceOver through iOS accessibility settings. Traverse primary tabs, the search bar and scopes, book activation controls, source-choice disclosures, article contents, the file input, and star controls. Check spoken names, selected/expanded state, focus order, and error announcements.
4. Disable both cellular data and Wi-Fi on the physical device. Force-close and relaunch the Home Screen app. Search for a rule, open a book chapter, follow a source choice, and reopen a star. Confirm that no download or recovery screen replaces the installed library.
5. Restore networking and test shell and library updates separately. Preserve the same star and disabled-book choice across each update, then repeat the offline restart.

Record the actual iOS version, release/build, and outcomes. Do not mark these checks complete from desktop WebKit or a simulator relaunch with networking available.

The book-content audit uses `scripts/audit-book-content.py` and `scripts/inventory-missing-sources.py`. Apply selected passage/chapter evidence with the extractor's `--evidence` option. Follow `docs/research/aon-mapping-resolution.md` for the baseline, bounded article cache, and deterministic replay. Evidence is tied to typed-entry and book-index hashes; a corpus change requires a fresh audit. Keep chapter target identity, opening-text gaps, and actual imported content separate in reports.
