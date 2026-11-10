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

## 2. Run the affected specs

Run only the specs that the change can break: the spec files you changed, plus the specs of the change's row in the feature map (section 5). The full suite runs in CI (`gh workflow run ci.yml --ref <branch>`), not locally. On a shared host, the full suite takes 12–22 minutes for Chromium and 45–60 minutes with WebKit, and load alone makes tests time out. See "Test scope" in `AGENTS.md`.

```sh
npm --prefix tests/e2e ci
npm --prefix tests/e2e test -- --project=chromium tests/<changed>.spec.ts tests/<affected>.spec.ts
```

On a host without the Nix dev shell, install the browsers once first: `npx --prefix tests/e2e playwright install chromium webkit`.

To run one test, pass a name pattern:

```sh
npm --prefix tests/e2e test -- --project=chromium --grep "<name>"
```

When the spec is listed in the `mobile-chromium` Playwright project (`playwright.config.ts`), also add `--project=mobile-chromium`. Add `--project=webkit` on the same specs when the change touches the service worker, installation, storage, or offline behavior.

Report any failure with its output. Do not call a change done while a test fails. When a test fails only during a run under load, run its spec again alone before you change code.

The default command without project flags includes Firefox. Its local startup failure and the informational CI browser projects are recorded in `docs/status.md`. Report the specs and projects actually run; a passing Chromium run does not verify WebKit or Firefox.

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
| Search and filters | `Search operates` (`search.spec.ts`) | `crates/search`, `crates/web/src/views/search.rs` |
| Book section search results | `book section result` (`search.spec.ts`) | `crates/schema/src/books.rs`, `crates/content-compiler/src/pipeline.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/book.rs` |
| Installer, integrity, recovery | `installs completely\|integrity\|Interrupted` (`install.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/content_cache.rs`, `web/sw.js` |
| Offline launch, no runtime server | `Offline launch\|runtime server` | `web/sw.js`, `crates/web/src/app.rs` |
| Entry rendering | `complex formatting` (`reference.spec.ts`) | `crates/web/src/views/rich.rs`, `crates/web/src/views/entry.rs` |
| Cross-references and history | `Cross-reference\|Back/forward` (`reference.spec.ts`) | `crates/web/src/route.rs`, `crates/web/src/views/rich.rs` |
| Body-link backlinks ("Referenced by") | `Referenced by` (`backlinks.spec.ts`) | `crates/content-compiler/src/backlinks.rs`, `crates/web/src/views/backlinks.rs`, `crates/web/src/library.rs` |
| Browsing and class categories | `browsing\|Expanded books` | `crates/web/src/views/browse.rs` |
| Books sorting and contents tree | `Books sort by category\|Expanded books show a hierarchical\|Expanded books, their contents tree` (`books.spec.ts`, `mobile.spec.ts`) | `crates/web/src/views/browse.rs`, `crates/web/src/route.rs` |
| Races, racial traits, classes, and archetypes | `race racial traits` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/graph.rs` |
| Class spell lists and source filters | `spell lists filter` (`relationships.spec.ts`) | `crates/web/src/views/spells.rs`, `crates/web/src/views/search.rs` |
| Feat prerequisites and chains | `feat pages\|category hub` (`relationships.spec.ts`) | `crates/web/src/views/related.rs`, `crates/web/src/views/chains.rs` |
| Book contents and reading order | `book contents\|Expanded books\|hierarchical` | `crates/web/src/views/book.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/graph.rs`, `crates/web/src/reading.rs` |
| Publication choices | `Source variants` (`source-variants.spec.ts`) | `crates/web/src/views/source_variants.rs`, `crates/web/src/views/search.rs`, `crates/web/src/views/browse.rs`, `crates/web/src/views/related.rs` |
| Keyboard and subsystem reading | `Keyboard` (`accessibility-reading.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/book.rs`, `web/styles.css` |
| Home stars and legacy saved records | `Home keeps legacy\|More than six starred` (`bookmarks.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/entry.rs` |
| Quick search suggestions, recent searches, and recently viewed | `Quick search\|Recent searches\|Recently viewed` (`search.spec.ts`) | `crates/web/src/views/quick_search.rs`, `crates/web/src/views/quick_rows.rs`, `crates/web/src/views/recent_searches.rs`, `crates/web/src/views/recent.rs`, `crates/web/src/store.rs` |
| Continue reading per book | `Continue reading\|continue point` (`continue-reading.spec.ts`) | `crates/web/src/store.rs`, `crates/web/src/views/continue_reading.rs`, `crates/web/src/views/quick_search.rs`, `crates/web/src/views/book.rs`, `crates/web/src/views/entry.rs` |
| Active books and preference recovery | `Book visibility\|No active books\|Invalid book choices` (`active-books.spec.ts`) | `crates/web/src/discovery.rs`, `crates/web/src/library.rs`, `crates/web/src/store.rs`, `crates/web/src/views/browse.rs` |
| Rule heading links and primary reading | `Rules text search\|Entry reading` (`reference.spec.ts`) | `crates/web/src/views/entry_contents.rs`, `crates/web/src/views/entry.rs`, `crates/web/src/views/search.rs` |
| Search bar, Home saved entries, and end-to-end research | `Home search\|Browse connects\|Books navigation` (`journeys.spec.ts`) | `crates/web/src/app.rs`, `crates/web/src/views/home.rs`, `crates/web/src/views/search.rs` |
| App-shell update and offline bookmarks | `App update waits` (`shell-update.spec.ts`) | `web/sw.js`, `crates/web/js/bridge.js`, `tests/e2e/server.mjs` |
| Content updates | `Content update` (`update.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/views/settings.rs` |
| PWA install metadata | `installable` | `web/manifest.webmanifest`, `web/index.html` |
| Mobile layout | `Mobile viewport\|Expanded books, their contents tree` (`mobile.spec.ts`) | `web/styles.css`, `crates/web/src/views/browse.rs` |
| Legal and attribution | `Licensing` (`legal.spec.ts`) | `crates/web/src/views/legal.rs` |
| Report a problem | `Report a problem` (`report-problem.spec.ts`) | `crates/web/src/report.rs`, `crates/web/src/app.rs`, `.github/ISSUE_TEMPLATE/app-problem.yml` |
| Publication scope | none: `python3 scripts/check-publication.py` after building the default | `scripts/check-publication.py`, `docs/research/licensing-audit.md` |
| Content compiler and default library | none: `cargo test -p cast-iron-content-compiler`, CI determinism job (two corpus builds, same bytes) | `crates/content-compiler`, `crates/corpus`, `crates/xtask`, `sources/corpus.toml` |

## Keep this file current

When you add a feature, add its row to the feature map, with its e2e test area and source files. When you rename or move a file listed here, update the row. A feature that is missing from this table cannot be verified by the next agent.

## Corpus and schema changes

Save the previously published library's `content/` directory outside `dist/` and `generated/` before rebuilding, so that you can install it in a browser and activate the replacement. Compile the new library with `cargo xtask content`. When the change touches ids, the legacy alias map (`aliases.toml` in the corpus), or books, also resolve every id of the last PSRD release against the new library; `docs/releasing.md` gives the commands and the tag that builds it.

Install the previous published library in a browser, bookmark an entry affected by the change, then activate the replacement. Verify the bookmark and entry before and after activation, then reload offline. Record which schema versions were exercised. A file-level ID check does not prove browser update behavior.

Compile twice and compare all generated files when compiler output changes: `rm -rf generated`, run `cargo xtask content`, hash the tree, and repeat.

Run `python3 scripts/check-publication.py` against the built distribution for corpus changes. It screens `dist/content` by default; pass a generated directory to screen another library. Record actual counts and limitations in `docs/status.md`.

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

After building, run `node tests/e2e/capture-ui.mjs /tmp/cast-iron-ui-check`. It starts and stops its own distribution server on port 4399, installs the library in fresh browser contexts, and records desktop and mobile screens in both themes. It writes measurements for overflow, alphabet and spell-level target sizes, selected scope colors, and console errors. Open representative PNGs to inspect them. `--search-only` limits capture to settled search states.

Wait for both selected scope and result kinds before treating a screenshot as filter evidence. A screenshot taken immediately after a click can still show the previous state. Article contents tests must also click a heading whose URL is already current, since no hashchange event occurs in that case.

## Remaining physical iPhone acceptance

Use a physical iPhone for these checks. Simulator status-bar changes do not disable networking, and browser accessibility trees do not prove VoiceOver output.

1. In Safari, star a known entry and confirm it appears on Home with working open and unstar actions.
2. Install the Home Screen app and download its library if needed. Confirm its library installs independently; Safari and Home Screen storage are separate, so do not expect stars to carry over automatically.
3. Enable VoiceOver through iOS accessibility settings. Traverse primary tabs, the search bar and scopes, book activation controls, source-choice disclosures, article contents, and star controls. Check spoken names, selected/expanded state, focus order, and error announcements.
4. Disable both cellular data and Wi-Fi on the physical device. Force-close and relaunch the Home Screen app. Search for a rule, open a book chapter, follow a source choice, and reopen a star. Confirm that no download or recovery screen replaces the installed library.
5. Restore networking and test shell and library updates separately. Preserve the same star and disabled-book choice across each update, then repeat the offline restart.

Record the actual iOS version, release/build, and outcomes. Do not mark these checks complete from desktop WebKit or a simulator relaunch with networking available.
