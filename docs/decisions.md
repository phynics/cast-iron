# Decisions

Decisions that refine `docs/spec.md` and that a later reader cannot derive from the code.

## Platform layer

- **CSP and Trunk's inline script.** Trunk writes its start-up module inline, so a strict `script-src` needs its hash. The `post_build` hook `web/stamp-csp.pl` (registered in `Trunk.toml`) hashes the built inline `<script>` and `<style>` and fills the `__CSP_SCRIPT_HASH__` and `__CSP_STYLE_HASH__` placeholders in the policy. Without the hook the page does not start, so a missing hook shows at once. `trunk serve` injects a live-reload script that this policy blocks; develop against `cargo xtask build-site` output.
- **SRI is off (`no_sri = true`).** Chrome warns on the `integrity` attribute that Trunk puts on the wasm preload, and same-origin files already come from the service worker cache.
- **No JavaScript beyond `sw.js` and the bridge, so no pre-render theme script.** `<html>` gets `data-theme` as the first action of `start()`, from a `localStorage` mirror of the IndexedDB choice. The window before the wasm module loads follows `prefers-color-scheme`. A user whose explicit choice differs from the system can see one background change per cold start.
- **Service worker update policy.** The first install skips waiting and claims the page. Later versions wait until the app posts `{type: "SKIP_WAITING"}`; the app shows "Update ready" and reloads on `controllerchange`. The worker never touches `cast-iron-content-*`.
- **Content caches.** One cache per version, `cast-iron-content-<version>`, keyed by the document-relative `content/<path>` URL. The manifest is stored with the version as `<version>/manifest.json`. `active_library` in IndexedDB is the only signal of "ready"; the boot check confirms that the cache holds every manifest file and sends the user back to the installer if not.
- **Install order.** Scan the cache for verified files (resume), check quota against the bytes still missing plus 5%, download with three workers and three tries per file, verify size and SHA-256 before `cache.put`, store the manifest, re-hash every file, write `active_library` (and clear `pending_install`) in one IndexedDB transaction, and only then delete the other content caches.
- **`Library` is `SendWrapper<Rc<_>>`.** Leptos 0.8 context requires `Send + Sync`; the browser runs one thread, so the wrapper never panics.
- **Hash links and `<base>`.** `<base href>` makes `#/x` resolve against the base. `normalize_location()` removes `index.html` and query strings at boot so that plain `#/...` anchors stay same-document navigations. The skip link is handled in code for the same reason.

## Reference screens

- **Search keeps its state in the route and edits it in place.** Typing replaces the history entry after 80 ms; toggling a filter pushes one, so Back undoes a filter but never a keystroke. `RouteView` does not rebuild `Search` when only the search route changes (a memo maps every search route to one value), and the Reader frame leaves scroll and focus alone for search-to-search changes. `route::replace` now writes the route signal through a thread-local handle: it is called from async tasks, where `use_context` finds nothing and the signal silently stayed stale.
- **Facet filters are tied to the selected kinds.** Facet groups show for the kinds that are selected (spell: school, class, spell level; feat: feat type; creature: type, CR, size; item: category, slot; class: class type). Changing the kinds drops facets that no selected kind offers, so no hidden filter stays active. Values come from `facet_values`; counts are not shown because they would be global, not conditional.
- **Search timings are exposed as data attributes** on `.results-region`: `data-search-ms` (the `SearchIndex::search` call only, from `performance.now()`), `data-index-ms` (first index load), and `data-query`. The index loads on the main thread. The original POC measured about 8 ms for its 4 MB index. The schema-2 desktop measurement and its limits are in `docs/status.md`; no real-phone memory result supports partitioning yet.
- **Entries render in one pass.** The POC's longest entry (`classes/cleric`, 420 blocks) builds in 13 ms with the shard cached, 39 ms cold, 60 ms with the CPU throttled 4x, with no long task. Progressive chunking would only add delay and break find-in-page, so there is none. Revisit if a source brings entries 5x larger.
- **Heading ids are the compiler's anchors, unprefixed,** because `Route::Entry` anchors and `scroll_to_anchor` use raw ids. A heading without an anchor gets `sec-<slug>`. A content anchor equal to an app id such as `main` would collide; the compiler's slugs do not produce those today.
- **Body-link reverse lookup is not built.** The References section lists outgoing body links, one per target. Typed relations have their own inverse index for racial traits, archetypes, and feat dependents; a general body-link reverse lookup would need an index over all shards.
- **Layout.** Below 48rem the primary nav is a fixed bottom bar with four destinations: Home, Browse, Books, and Settings; at 64rem and wider the entry screen puts References in a sticky side column and Search puts filters in a side panel. The rule `main:has(> .wide, > .search-page, > .entry)` widens only the reference screens; Settings, Legal, and the installer keep the reading measure. Grid and flex parents set `min-width: 0` so that a wide table scrolls inside its own wrapper and never widens the page (without it, mobile Chrome zoomed the whole page out).
- **Bookmarks.** A bookmark whose entry is missing from the library shows its saved title, tagged "not in this library", and keeps all actions except opening it. Deleting a collection asks inline and says that bookmarks stay. Recently viewed entries live in `meta` under `recent` (12 items, id and title, newest first).

## Browser acceptance suite and CI

- **The suite tests the real `dist/`.** `tests/e2e/server.mjs` serves an existing `dist/` at `/cast-iron/`, and `playwright.config.ts` refuses to start without one. It also exposes `/__test/*` hooks (corrupt, stall, abort, slow, or refuse a path; overlay an alternative library; read the request log). The hooks exist on this test server only. One worker runs the tests in turn, because they share that server. The default port is 4318 and `E2E_PORT` changes it.
- **Offline means two things.** Tests call `context.setOffline(true)` and also make the server drop every connection. Chromium's offline emulation does not cover the browser's own check of `sw.js`, which reached the server in the first run, so the allowed exception is exactly `sw.js`.
- **Update test library.** The test builds version 2 in a temp directory (a copy of the library with `legal.json` changed, hashes and manifest recomputed, version by the compiler's rule) and serves it through the overlay. The client does not derive versions, so any `v...` string would do.
- **Browsers.** Chromium (and a 390 px Pixel 7 project for `mobile.spec.ts`) gate CI. WebKit passes all tests locally on macOS; its project turns Playwright traces and screenshots off, because they inject an inline `<style>` into WebKit pages of persistent contexts and the page CSP refuses it, which would be a false console error. WebKit runs in CI with `continue-on-error` until a CI run shows it stable. Firefox is disabled in CI as recorded below.
- **Compiler determinism is a CI check, not a browser test.** The `content` job compiles twice and compares a hash over all of `generated/`.
- **Pages.** `pages.yml` builds as CI does and deploys with the official Pages actions. The repository must have Pages set to the "GitHub Actions" source, which is a repository setting that no workflow changes.

## Public POC rollout, 2026-11-03

- The public POC uses the OGL for the selected Core Rulebook and Bestiary rules, with Paizo's Community Use Policy for permitted descriptive references. The policy review and its limits are in `docs/research/licensing-audit.md`. This does not authorize artwork, campaign books, or additional sources.
- GitHub Issues is the maintained project contact shown on the legal page and in the README. It provides a public contact route without publishing personal contact details. External navigation occurs only when the user follows the link.
- `python3 scripts/check-publication.py` screens the built library before upload in CI and Pages. It checks hashes, source scope, attribution, OGL references, and a finite list of named setting material. Its generated report records the limits. It is evidence for the selected POC, not a legal classifier or a substitute for reviewing a new source.

## Expanded book coverage and prestige classes

- The expansion uses every available book in the pinned snapshot, rather than fetching a newer or different dataset. Selection is explicit and generated from a reviewed 16-book inventory. Unsupported standalone types remain listed in coverage; many are represented inside the selected parent rules trees. This is broader book coverage, not a claim of every published PF1e book or complete standalone category coverage.
- Prestige classes have a distinct schema kind while retaining `ClassFields` and existing IDs. The content schema is version 2; the new client deliberately reads versions 1 and 2 so an old verified library stays usable during an update. The search format stays unchanged. Technology Guide stores Technomancer as a section, so the reviewed source configuration explicitly classifies that URL as a prestige class.
- Placeholder `PFSRD` source labels inherit the reviewed enclosing publication. Explicit non-placeholder source labels remain intact. The catalog and legal page therefore identify the actual books rather than a generic source bucket.
- Named stories and setting material are omitted before deduplication and rendering. Their locations and reasons are part of coverage. Absent optional equipment fields are treated as absent, rather than rejecting the whole record.
- Abrakarn Viper has both creature statistics and construction-item metadata at one upstream URL. The creature remains canonical and retains the supplemental item fields and both provenance paths. Its public identity is unchanged. Conflicting nested copies remain visible in the validation notes.
- The full-corpus text-preservation test now follows URL-addressed records inside trees and exempts only reported policy removals or typed copies rendered separately. Its independent text extraction decodes HTML entities at text-node boundaries and ignores discretionary soft hyphens, matching what users read.

## Relationships and original book order (schema 3)

Product context: `PRODUCT.md`. Data findings and counts: `docs/research/relationship-data.md`.

- **Three new kinds, all from existing upstream families.** `trait` (298 records, plus the 17 `drawback` records as category `Drawback`; 20 religion traits are omitted for Product Identity), `racial_trait` (342), and `archetype` (221 records, 218 promoted). IDs stay URL-derived, so a nested copy that used to be inline text now has the id its url gives. All 9,112 earlier ids and titles survive, with no kind change. The nested copies are identical to the typed records, so the replacement by an entry link drops no text. Schema 3 is required because older clients cannot read the kinds and fields.
- **Families that stay without a category, on purpose.** `animal_companion` (68; 40 of them are in no selected tree, so they are absent), `mythic_spell` (269, absent; a kind would need a relation to the base spell through `spell_source`), `vehicle`, `settlement`, `haunt`, `resource`, `army`, `kingdom_resource` (all in rules trees). None is in the categories that `PRODUCT.md` asks for. They remain in the coverage report. Add them when a category is asked for, not before.
- **Class text typed as an archetype is not an archetype.** Three APG records under the Antipaladin (`Class Features`, `Class Skills`, `Ex-Antipaladins`) carry the type `class_archetype`. The compiler promotes only records whose url sits below a section that ends in `Archetypes`, reports the rest as unpromoted, and leaves them as text in their class.
- **Relations are held by the source entry.** `Part` (racial trait to race), `Archetype` (archetype to class), `Prerequisite` (feat to feat). The inverse is read from `relations.json`, one sorted array of `[source, kind, target]` edges (1,531 edges, 173,675 bytes, 14,491 gzip), never from the shards. The client must tolerate its absence in libraries of schema 1 and 2.
- **No `SpellListMember` relation.** `SpellFields.levels` is the only source of the class and level of a spell. Edges would copy 8,010 distinct pairs into every reader and could drift from it. `lists.json` is an index of that field (128,922 bytes): spells once in a table, lists as positions. Lists whose class has no class entry stay visible with `class` empty.
- **Parents are found in a fixed order.** For a racial trait: the race record that nests it, then a race record above it in the url, then a race with the url's race name (an APG alternate trait goes to the Core Rulebook race, not to an Advanced Race Guide race of the same name). For an archetype: a class above it in the url, then the class whose title equals the upstream `subtype`. A record with no parent is listed in coverage; none occurs today.
- **Prerequisites keep the text and add names.** `FeatFields.prerequisites` is the upstream wording (the extractor now also reads `description`, where 811 of the 1,002 wordings are). `prerequisite_feats` holds feats that the wording names, matched by exact title. A shared title resolves only inside the dependent feat's book. An either-or list sets the relation label `or`; this is a heuristic, so `prerequisite_feats` means "named", not "required". Numeric requirements and races are counted in coverage but not typed.
- **An excluded url excludes everything below it.** `content-policy.toml` matched exact urls only, so promoting traits to typed records would have let 20 deity-specific traits past a container that the policy removes.
- **The book view has curated chapters and upstream order, and it is not a category list.** `book-order.toml` maps printed chapter titles to url prefixes. Entries nested in a tree keep their place; entries that no tree nests follow `upstream-order.json`, which `scripts/extract-upstream-order.py` takes from the databases' nested sets. The compiler does not open the databases, and the input digest covers only the JSON, so the extracted order is committed and reviewed like `legal-notices.json`. A heading appears in the tree only when it holds a nested entry. The url decides which publication's tree holds an entry; 33 entries differ from their `source`. A chapter that is one rules tree is that tree. Entries that no chapter claims go to coverage as unplaced (0 today).
- **Chapter evidence is recorded per book.** The Core Rulebook's first 15 chapters now match the publisher-hosted contents PDF. Its nested placement and appendix grouping still need work. Other outlines remain drafts based on upstream headings and recollection. `docs/research/book-order-evidence.md` records publisher evidence; `docs/research/aon-book-sections.md` and its generated Legacy menu inventory record website navigation, which does not establish print order.
- **Races carry their group as a tag** (`standard_race`, `core_race`, `featured_race`, `uncommon_race`, `monster_race`), because the 35 monster races from the Bestiaries share the Races category with the 44 player races.
- **The client reads schemas 1, 2, and 3.** The new files are not in the required set, so an old library stays usable. The three files sit in the manifest and are hashed like every other file.

## Reference navigation

- Category browsing and book browsing are separate routes. Categories offer alphabetical lists, facets, spell lists, and feat chains. Book pages render only the curated contents tree, preserving its order and showing source notes and review warnings. They do not substitute category lists when chapter data is missing.
- Relation and spell-list indexes load from the installed cache on demand, once per library. Missing schema-3 files in an older library disable the corresponding panels and offer an update route. Ordinary use has no network fallback.
- Racial traits and archetypes link to their parents, and parent entries show the inverse lists. Feat panels retain prerequisite wording alongside named feat links. An `or` label remains visible; these extracted links are not an eligibility calculator.
- `books.json` carries optional source notes and review questions with serde defaults. These additions preserve compatibility with existing schema-3 readers, so the schema remains 3. Entry pages link back to book position and offer previous/next navigation in the curated order.

## Verification lessons from the reference UI

- A site build reuses an existing generated manifest. Compiler, schema, and source changes therefore require explicit content compilation before building. Acceptance fixtures read `generated/`, while the test server serves `dist/`; mismatched versions invalidate the comparison. The verification skill records the sequence.
- Entry titles and headings recur in relationship panels, body text, and book navigation. Browser expectations select within the relevant region. Whole-list name filtering is substring matching, and collapsed book branches create children lazily. Tests target the relevant region, expand branches before inspecting children, and derive expected matches from the corpus.
- The relationship acceptance suite disables the network after installation and service-worker control. This verifies lazy index loading from the installed library as part of ordinary navigation. Local passes, CI results, deployment, and live-site checks remain separate evidence in `docs/status.md`.

## Navigation after the iPhone walkthrough

- Search is a control available from each reading screen, not a primary destination. The search results route remains for URLs, filters, history, and keyboard lookup. Its heading remains available to screen readers without consuming visual space above the bar. Header submission reads the input value at submit time to avoid stale query state.
- Home shows the six newest saved entries and recent reading. The existing bookmark and collection manager remains reachable from Home, preserving saved URLs and management actions. Books has a separate primary destination so reading a publication does not require going through category browsing.
- A `grapple` query on the published iPhone build initially surfaced equipment and feats. Quick search scopes expose Rules text directly. Rule body headings feed the existing search alias field, so a section such as Grapple can surface its containing Special Attacks entry. The index layout stays at version 1, and content stays at schema 3. This improves discovery without claiming complete prerequisite or character-build logic.
- App-shell and rules-library updates are separate lifecycles. Settings checks both; a waiting shell requires Reload, while a library replacement requires Download update. The shell-update acceptance test changes the service-worker version through a test-server hook, verifies it waits, then confirms activation preserves the active library and Home bookmarks offline. The hook is not shipped with the app.
- The iPhone report records the published five-tab build, not the replacement navigation. Device-driver failures left Home Screen installation, offline standalone relaunch, and the full character workflow unverified on the simulator. Automated local results are recorded separately.

## Planned simplification after the reference walkthrough, 2026-11-03

These are accepted product directions awaiting implementation in [the UI task list](ui-tasks.md). They supersede earlier decisions to retain the bookmark and collection manager. They do not describe shipped behavior.

- Browse and Books remain separate primary tabs. Remove the shared browsing-mode picker. Category discovery belongs in Browse. Rules belong in the book directory and search, with existing entry links and starred rules still readable.
- Use the AoN Rules source directory as the baseline for book, topic, and subsection discovery. This is a navigation and mapping baseline, not evidence of printed chapter order or permission to import new content. Preserve the user's original-order requirement and current evidence warnings.
- Remove collections and the separate bookmark manager. Home lists starred entries and lets the user open or unstar them. Preserve existing stars when retiring collection UI and routes.
- Add book activation controls. The proposed default is a persistent filter over downloaded content rather than deletion. Detailed filtering and saved-entry behavior are documented as working defaults in UI-03, not additional confirmed user requirements.


## Reference simplification implementation

- Active-book preferences store disabled canonical book IDs under the existing metadata key `disabled_books`. Absence and an empty set mean all books active. Unknown disabled IDs survive updates, and newly added books default active. Disabling does not remove cache files or stars.
- `Library` keeps canonical reads. `Discovery` snapshots eligible IDs using book placement, with an explicit catalog-source fallback for older libraries or missing placement. Ordinary discovery uses this scope before counting, limiting, or projecting relations and spell lists. Search publication filters still describe attribution sources, which can differ from book placement.
- Library loading reads preferences and placement before rendering discovery. A corrupt preference value offers Retry or Reset active books, preserving downloaded content and stars. Toggle writes publish a revision only after persistence. That revision rebuilds discovery screens, while Books remains mounted to preserve its controls and open sections.
- Collection UI and manager routes are retired. Existing stored records remain readable, and each unique starred EntryId appears on Home. Missing entries keep their saved title and an Unstar action. Legacy saved-entry URLs recover to Home, and the old Rules-category URL recovers to Books.
- Article contents and heading-aware search use existing typed headings and entry anchors. Matching rule results read installed shards, and exact named rule sections rank ahead of longer title prefixes. Search format 1 and content schema 3 are unchanged. Article contents uses stylesheet classes because inline indentation violated the strict CSP in the first browser run. Clicking an already-current section explicitly scrolls to it.
- Primary rule text precedes relationship discovery. Compact jumps retain research access. Prestige requirements resolve unambiguous feat and skill names only inside their stated sections, with punctuation normalized. Duplicate names remain text instead of choosing a source silently.
- AoN's Rules directory is a navigation baseline and a reproducible mapping inventory. Printed-order evidence and unresolved-section reporting remain separate. No new rule content or source was imported.


## Release branch and issue tracking, 2026-11-03

The app is drawing positive reactions, and the user requested stricter
development hygiene. Separate integration from publication and track outstanding
work in GitHub Issues. Main remains the integration branch. A dedicated
`release` branch advances only through explicit, reviewed promotion of tested
main commits. It starts at the last successful published main SHA, rather than
the unrelated UI work in draft PR #7. Normal fast-forward pushes preserve shared
history and let a later reader inspect the exact release range.

Pages already uses Actions artifacts, so changing its legacy source-folder field
is not the migration. Restrict the `github-pages` environment to the branch
`release`, and guard both workflow jobs against other refs, including manual
dispatch. Preserve the project path, permissions, deployment serialization, and
existing CI/browser checks. The environment rule blocks main publication before
the workflow PR merges. No additional approval gate is needed.

GitHub Issues owns new development and remaining acceptance work. Docs retain
observations, corpus limits, decisions, and historical results with canonical
issue links. Completed local UI tasks are history, not a new backlog. Physical
device and printed-order gaps stay open until their specific evidence exists.
See [contributing](../CONTRIBUTING.md), [release operations](releasing.md), and
[migration issue #10](https://github.com/phynics/cast-iron/issues/10).

## Firefox CI disabled, 2026-11-03

- Stop installing and running Firefox in CI. In [run 41697800843](https://github.com/phynics/cast-iron/actions/runs/41697800843), all 37 Firefox tests failed, mostly after the shared 90-second installation wait. The sequential suite took 57 minutes 55 seconds. Although informational, the step delayed completion of the required browser job. Keep the Playwright project for targeted debugging. Re-enable CI coverage after the installer hang is diagnosed and verified. Chromium, mobile Chromium, and informational WebKit coverage remain enabled.

## Portable stars and publication choices

- Home offers manual star transfer through a versioned JSON file. Export and import work without a backend. Import validates the complete bounded archive before writing and merges permanent entry IDs in one transaction. Existing stars win, unknown entry IDs remain usable as missing-entry stars, and repeated imports add no duplicates. Safari and the Home Screen app still have independent storage; the file is a transfer mechanism, not automatic synchronization.
- Search and category lists group equal normalized titles only within the same entry kind, after active-book filtering. Each publication choice retains its original ID and content. Search ranks and paginates groups while reporting the underlying publication-entry count. Matching rule headings remain attached to each choice. Prestige requirement names with several installed matches expose publication choices rather than selecting an arbitrary record.
- The AoN mapping audit includes typed rule headings and preserves candidate anchors. Missing sources, missing sections in installed sources, and ambiguous targets have separate dispositions. These labels describe the installed snapshot, not the presence of material elsewhere on the web. The inventory does not change printed book order or authorize new content imports.

- The two reported Combat navigation labels use reviewed display aliases for connector casing. This avoids applying automatic capitalization to proper names throughout the corpus. Canonical titles, IDs, and rule bodies remain unchanged.

## Rule target evidence and new-source preparation

- Exact titles alone do not identify a reading target when a chapter container, introduction, and complete article share a name. The AoN audit now compares source-specific opening passages and explicitly selects the dedicated article where appropriate. Identical paragraph positions collapse root/leading-heading aliases; unresolved ties stay explicit.
- Whole-chapter context uses a separate evidence method and retains opening-text gaps. The two Core Rulebook cases match the named canonical chapter and many descendant headings. This does not establish complete prose or alter printed order.
- Evidence binds to hashes of the typed entry files and book index. Applying stale evidence fails rather than silently guessing a new target. Fetched article HTML stays outside the repository; committed evidence contains hashes, metadata, and selected IDs/anchors.
- Missing-source research counts actual source-tagged records and source manifests at exact pins. It does not equate a book title or character-option catalog with complete book prose. A native Pf Data adapter should start with the missing Magic Basics passage before a new-book import.

- Star export/import was removed. Stars stay local to the browser profile; no archive file format is offered. This supersedes the manual star transfer decision above.

## Reviewed primary-source records during the migration

- Pin Pf Data 1e independently and convert its descriptions in native Rust to the existing schema-3 nodes. Source formats, directives, and renderer code do not reach the client. Compiler-only additions do not change the serialized schema or search format.
- Keep full-corpus experiments outside the production library. Unknown constructs, unresolved references, missing credits, and Product Identity screening remain explicit publication gates. The comic fixture requires primary evidence for its appendix's OGC designation and Section 15 notice. Paizo Blog rules have a separate Community Use permission route; that permission excludes comic excerpts.
- Allow whole records into production only through `sources/pathfinder-data-1-e/approved.json`. The initial selection restores the missing Core Rulebook page 206 opening passage. It has its own permanent native-protocol ID and retains all previous IDs. Existing parent and reviewed topic IDs provide navigation; unreviewed topics remain in coverage.
- The selected passage belongs beneath the existing Magic article. Its page attribution and the publisher evidence for the Magic chapter establish that placement. This does not revise printed chapter order or claim completeness for the full upstream corpus. PSRD-Data remains the fallback while the full migration is blocked.
