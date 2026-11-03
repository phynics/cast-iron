# Status

Updated 2026-11-03. The repository is public. [PR #4](https://github.com/phynics/cast-iron/pull/4) merged at `12acb7d`; its [Pages deployment](https://github.com/phynics/cast-iron/actions/runs/41590678502) succeeded. Terra observed the published schema-3 library on an iPhone Simulator. Branch `navigation-journeys` contains the replacement navigation and search improvements below; those changes are not yet deployed.

## Acceptance checklist (spec section 11)

"Local" means the Playwright suite in `tests/e2e` passed against a real `dist/` build served at `/cast-iron/` by `tests/e2e/server.mjs`. Last local full run: 55 passed in 2.3 minutes (chromium 27, webkit 27, mobile-chromium 1), including relationship navigation, spell lists, feat prerequisites, source search, book history, and mobile layout. `cargo xtask check` also passed.

| Item | State | Evidence |
|---|---|---|
| Static GitHub Pages deployment works | Live pass | [Deployment run 41576342801](https://github.com/phynics/cast-iron/actions/runs/41576342801) succeeded for `a6636f3`. Chromium installed the library from [the public site](https://phynics.github.io/cast-iron/), verified the contact and OGL notice, reloaded offline, and found Fireball offline. No page errors or third-party requests occurred. |
| Application is installable where browser support allows | Local pass | `Application is installable where browser support allows` |
| Representative rules dataset installs completely | Local pass | `Representative rules dataset installs completely` |
| Download integrity is verified | Local pass | `Download integrity is verified` (corrupted shard rejected; retry fetches only that file) |
| Search operates entirely offline | Local pass | `Search operates entirely offline` (p95 < 100 ms asserted) |
| Category and sourcebook browsing works | Local pass | `Category and sourcebook browsing works` checks every publication; `Expanded books and separate class categories work offline` checks 16 books and the distinct categories. |
| Rule entries preserve complex formatting | Local pass | three `Rule entries preserve complex formatting: …` tests |
| Cross-reference navigation works | Local pass | `Cross-reference navigation works`, plus the deep-link test |
| Back/forward navigation works | Local pass | `Back/forward navigation works` |
| Bookmarks persist across restarts | Local pass | `Bookmarks persist across restarts` (persistent browser profile, relaunched offline) |
| Interrupted installation can recover safely | Local pass | `Interrupted installation can recover safely` |
| Offline launch works after a new browser session | Local pass | `Offline launch works after a new browser session` |
| No application feature requires a runtime server | Local pass | `No application feature requires a runtime server` (zero server requests after install, apart from the browser's own `sw.js` update check) |
| The compiler generates repeatable artifacts | Local pass | Two final full-corpus compiles were byte-identical. CI job `content` compiles twice and compares. |
| Licensing information is preserved and accessible | Local pass | `Licensing information is preserved and accessible` |

Phase 6 also requires that bookmarks survive a content update. `Content update keeps user records` covers this. A separate real-browser check installed the actual published schema-1 POC library in the new app, bookmarked Arcane Archer, reloaded offline, installed schema 2, and confirmed the bookmark and offline reading survived. `scripts/check-library-upgrade.py` also confirmed all 1,937 published IDs and titles are preserved.

## Schema-2 rollout baseline

Content version `v91f4bc8ffd606a17`, schema 2. Classes has 27 entries; Prestige classes has 19, including Technology Guide's Technomancer.

| Measure | Value |
|---|---|
| Entries | 9,112 across 16 publications |
| Download | 58,487,268 bytes raw |
| WASM | 1,212,369 bytes raw; 450,990 bytes gzip |
| Search index | 19,264,125 bytes raw |
| Search latency | p50 2.5 ms, p95 4.8 ms, max 5.4 ms over 20 queries in desktop Chromium |
| Index load | 24.2 ms from the Cache API |
| Links | 1,304 resolved, 128 out of selection, 38 broken upstream |
| Intentional omissions | 432 source occurrences, including duplicated story copies and partial-body omissions; no malformed-record rejections |

These are desktop measurements, not real-phone memory or speed results. Light and dark Prestige classes screens were inspected at 390 px and 1280 px. The original library stayed readable under the new app before the schema-2 update was installed.

| Publication | Entries |
|---|---:|
| Advanced Player's Guide | 768 |
| Advanced Race Guide | 516 |
| Bestiary | 545 |
| Bestiary 2 | 428 |
| Bestiary 3 | 360 |
| Bestiary 4 | 288 |
| Core Rulebook | 1,612 |
| Game Mastery Guide | 146 |
| Monster Codex | 395 |
| Mythic Adventures | 375 |
| NPC Codex | 342 |
| Technology Guide | 166 |
| Ultimate Campaign | 65 |
| Ultimate Combat | 618 |
| Ultimate Equipment | 2,066 |
| Ultimate Magic | 422 |

The selection has 64 typed directories and 101 complete rules trees. Unsupported standalone record families and duplicated section-file representations remain listed in coverage. Later books absent from the pinned snapshot are not included.

## Published relationships and book order, schema 3

Content version `vdb8e844bfc068446`, schema 3. Verified locally: `cargo xtask check` passes; two `cargo xtask content` runs give the same tree hash; `scripts/check-library-upgrade.py` finds all 9,112 earlier ids and titles with no kind change and no dangling reference in the three new files; the full Chromium, WebKit, and mobile browser suite passes against the built site (47 tests). The five relationship/navigation tests also passed with the browser offline in Chromium and WebKit (10 checks). Desktop and mobile screenshots in both themes were inspected; the screenshot run reported no console errors or horizontal overflow. Data findings: `docs/research/relationship-data.md`. Decisions: `docs/decisions.md`.

| Measure | Value |
|---|---|
| Entries | 9,967 (was 9,112): 218 archetypes, 342 racial traits, 295 traits |
| Relations | 1,531 edges: 342 `part`, 218 `archetype`, 971 `prerequisite` |
| Feat prerequisites | 1,002 feats with wording; 597 name a feat; 969 of 1,298 name-like requirements resolve (74.7%) |
| Spell lists | 23 class names, 8,010 distinct pairs (8,504 upstream); 8 names have no class entry: Arcanist, Bloodrager, Elementalist Wizard, Hunter, Investigator, Shaman, Skald, Warpriest |
| Book view | 16 books, 9,967 entries placed, 0 unplaced; Core Rulebook main chapter order verified against the publisher PDF, other outlines and nested placement remain drafts |
| Download | 61,827,011 bytes raw (was 58,487,268, +5.7%); largest shard unchanged at 524,249 bytes |
| New files | `relations.json` 173,675 (14,491 gzip); `lists.json` 128,922 (28,157); `books.json` 1,067,959 |

The client reads schemas 1, 2, and 3. It now uses the new files for racial traits, archetypes, feat links and chains, class spell lists, book contents, and previous/next book navigation. Category browsing has dedicated shortcuts and facets; source filters remain part of the search URL. Missing schema-3 files in an older library offer an update path. Book contents display source evidence and review warnings.

AoN source pages group database records rather than chapters. Legacy PRD section links are captured in a reproducible 18-menu, 167-link inventory, including sections for books absent from the current corpus. Details: `docs/research/aon-book-sections.md`, `docs/research/legacy-prd-sections.json`, and `docs/research/book-order-evidence.md`.

## Navigation and common journeys, branch `navigation-journeys`

Primary destinations are Home, Browse, Books, and Settings. Search is a bar, and Home shows up to six saved entries plus recent reading. Quick scopes expose Rules text, Spells, and Feats. Rule headings feed the existing search alias field: `grapple` with Rules text selected surfaces Special Attacks first in the checked build. A native regression requires that entry in the first three results.

Content version `v171f0348e5717ee0` has 9,967 entries, schema 3, 61,956,172 bytes raw, and a 20,329,611-byte search index. Search format remains 1. The upgrade check preserves all 9,967 prior IDs and titles with no kind changes or dangling derived references. Publication checks and `cargo xtask check` pass. The full built-site suite passes 55 tests. Desktop and mobile screenshots in both themes were inspected; the screenshot run reported no horizontal overflow or console errors.

Composed offline journeys cover quick lookup, creating a collection and reopening a bookmark from Home, racial traits, class options and archetypes, class spell levels, traits, equipment, rules, and reading onward from book contents. A new app-shell test verifies that the worker waits for Reload, preserves the installed library and Home bookmarks, and works after an offline reload. The library-update test verifies replacement activation separately.

### Published iPhone Simulator baseline

Terra used the T3 Device panel on iPhone 18 Pro, iOS 27.2, Safari. The published five-tab build installed library `vdb8e844bfc068446`, reported 9,967 entries, checked for updates, opened Advanced Player's Guide contents and Read Races, and saved a bookmark. A grapple lookup initially showed equipment and feats. Safari reported persistent storage not granted.

The collection-name input command failed, and its retry failed with an expired reference. Device testing stopped. The full character workflow, comprehensive subsystem read, previous/next controls, Home Screen installation, offline standalone relaunch, replacement updates, dark theme, and the new navigation remain unverified on the simulator. See [iPhone journey observations](research/iphone-journeys.md). Automated acceptance and simulator observations are separate evidence.

## Not verified yet

- GitHub CI and deployment for the replacement navigation are pending. The schema-3 reference UI deployed successfully in run 41590678502. The 16-book schema-2 deployment succeeded in run 41579539956. POC PR CI passed on GitHub in runs 41569305309 and 41569305238. The first Pages attempt failed before Pages was enabled. Deployment run 41576342801 then succeeded and the public site passed the live smoke check.
- Firefox: the tests exist, but Playwright's Firefox does not start on the development machine (macOS 27). CI runs Firefox as non-blocking.
- Real devices: Android Chromium, iOS Safari, installed-app (standalone) relaunch.
- Persistent storage: headless Chromium reports "not granted". On real browsers the result depends on browser heuristics.

## Known gaps

- Licensing review is scoped to the 16 selected books. The publication screen covers all 9,967 entries and found no issue, but does not classify every sentence or establish blanket rights to new sources. See `docs/research/licensing-audit.md`.
- Paizo Community Use Policy: on 2026-11-03 the official policy, FAQ, reinstatement post, and licensing overview were fetched and compared with the built Legal page. The notice matches the policy text, and the page shows a contact link. No registration is required. This does not clear every sentence of the dataset, and Paizo can withdraw the permission at its discretion. Details are in the audit.
- Book-order fidelity is incomplete. Core Rulebook main chapters are verified; other outlines, nested placement, and appendix grouping need publisher evidence and mapping work. Zero unplaced entries proves coverage, not printed order.
- Advanced Race Guide standard racial traits remain embedded in race bodies rather than standalone records. Alternate-trait replacement links are not modeled.
- Eight spell-list names lack a class entry, as listed above. The lists cover installed spells; they do not establish complete coverage of those classes or their books. Feat chains contain extracted named links, including alternatives, rather than complete eligibility logic.
- No "Referenced by" list for body links (needs a reverse link index). Typed relations have inverse lookups in the entry screens; ordinary body links still lack a reverse index.
- Category lens counts and source counts are global. Search field facets omit counts; the UI does not calculate counts conditional on every active filter.
- In spell and creature `spells` stat lines, upstream lists become lines with breaks, so list structure is lost.
- Heading ids are not prefixed, so a content anchor could collide with an app element id such as `main`.
- Firefox, and a single flaky run of `Back/forward navigation works` on Chromium, which could not be reproduced (see `docs/decisions.md`).

## Next (post-POC, spec section 12)

1. Keep the licensing review current when changing the source selection. The POC is public under the scoped OGL and Community Use review.
2. Review `sources/psrd-data/book-order.toml` against the printed books. Add standalone support for the families that remain in rules trees or are excluded (animal companions, mythic spells, vehicles, settlements, haunts, campaign resources and armies) when a category needs them. Newer books require a new source pin or source dataset and licensing review.
3. Run the suite on real devices and record the results here.
