# UI implementation history

Updated 2026-11-03. This preserves earlier implementation and acceptance context. [GitHub Issues](https://github.com/phynics/cast-iron/issues) now owns outstanding work; do not maintain this as a second tracker. Completed checkboxes refer to local work, not deployment. The user's latest direction takes precedence over earlier navigation and collection decisions. `docs/spec.md` remains unchanged.

Inputs are the user's requests, the [published iPhone walkthrough](research/iphone-journeys.md), and the [AoN rules directory](https://aonprd.com/Rules.aspx?Name=Source&Category=None). The initial Gemini attempt could not render images. The user subsequently supplied the manually guided review; its findings and evidence limits are recorded in [the Gemini follow-up](research/gemini-ui-review.md).

## Confirmed direction

- Keep four primary tabs: Home, Browse, Books, and Settings. Keep search as a bar.
- Browse contains entry categories. Books contains the book directory and rules browser. Remove the shared By category / By book picker.
- Present rules through Books and search. Remove the standalone Rules category and Home Rules shortcut. A starred rule can still be opened from Home, and reading links must continue to work.
- Use AoN's book > topic > subsection hierarchy as the directory baseline. Preserve the earlier original-book-order requirement. Do not substitute alphabetical categories for book contents.
- Remove collections and the separate bookmark management screen. Home lists starred entries with direct open and unstar actions.
- Add controls to activate and disable books.

The working default for activation is a persistent visibility preference over the installed library, not an uninstall operation. All installed books start active. Disabling a book removes its entries from ordinary search and category discovery. Its row remains available in Books so it can be reactivated. Existing stars and direct entry links remain usable with a disabled-book notice. These defaults are implemented locally. They refine the user request without changing the installed corpus.

## Implement in this order

### UI-01: Separate Browse and Books

- [x] Make Browse open the category directory and Books open the publication directory directly.
- [x] Remove the shared By category / By book picker, redundant headings, and category-mode book lists.
- [x] Remove Rules from Browse, category shortcuts, and Home shortcuts. Keep Rules text as a search scope.
- [x] Redirect old general Rules-category URLs to Books. Preserve entry URLs, rule cross-references, and search URLs.

Acceptance: each primary tab has one clear purpose. A user reaches a rule through Books or search, without choosing a browsing mode first. Back and Forward preserve the selected book and expanded section. A saved rule still opens from Home.

Areas: `crates/web/src/views/browse.rs`, `categories.rs`, `home.rs`, `route.rs`, and `app.rs`.

### UI-02: Map the AoN directory to installed books and rules

- [x] Capture a reproducible inventory of the linked AoN Rules page: book labels, topics, subsections, and destination URLs.
- [x] Map that inventory to canonical Cast Iron book IDs and installed rule entries. Record title aliases and unmapped sections.
- [x] Use expandable book and topic sections with readable subsection links. Keep category entry lists in Browse.
- [x] Preserve printed chapter order where established. Use AoN labels and nesting as mapping clues, with review warnings for unverified placement.
- [x] Keep missing books and missing rules explicit. An unavailable book must not offer a working activation control or imply its content is installed.
- [x] Review licensing before importing any additional content. The navigation baseline does not authorize copying the entire AoN database.

Acceptance: a user can follow Core Rulebook to Combat to Special Attacks, and read onward without leaving the book context. Every imported directory section resolves to installed content or has an explicit coverage status. Existing IDs and publisher evidence remain intact.

Dependency: UI-01. Read [book-order evidence](research/book-order-evidence.md) before changing `sources/psrd-data/book-order.toml`.

### UI-03: Add persistent active-book controls

- [x] Give each installed book an Active toggle in Books, with clear active and disabled states.
- [x] Persist the selection through restart and app or library updates. Define how newly installed books enter the selection.
- [x] Apply one shared book-selection rule to search results, category lists and counts, relationship discovery, spell lists, and feat chains.
- [x] Resolve book identity consistently where an entry's source label differs from its placement in `books.json`. Do not filter by display-name string matching.
- [x] Keep disabled books visible in the directory for reactivation. Disabling does not delete downloaded files or stars.
- [x] Keep starred and directly linked entries readable, with a notice when their book is disabled. Do not silently reactivate the book.
- [x] Provide a useful empty state when no books are active, with a direct way to activate them.

Acceptance: disabling one book removes its entries and counts from ordinary discovery, including after offline restart. Reactivating restores them without a download. Stars survive both actions. Zero active books produces an actionable empty state.

Dependency: UI-02 establishes the canonical book mapping. This task needs a small state and filtering design before code changes.

### UI-04: Replace bookmark management with Home stars

- [x] List all starred entries on Home. Remove the current six-entry truncation without leaving the remaining entries inaccessible.
- [x] Give each row a separate entry link and an accessible Unstar control. Unstarring must not open the entry.
- [x] Keep the starred state consistent between Home and entry pages. Use the same star terminology and icon treatment.
- [x] Remove Open bookmarks, the dedicated manager, collection creation, collection membership controls, rename, and deletion UI.
- [x] Preserve existing starred EntryIds. Existing collection members become ordinary Home stars, without duplicate rows or lost entries.
- [x] Redirect old bookmark and collection URLs to Home. Keep old stored fields readable if needed for a safe upgrade.
- [x] Keep missing-entry stars visible with their saved title and Unstar action. A disabled book is different from missing content.

Acceptance: a profile with more than six stars, including entries in several old collections, retains every unique star after upgrade. The user can open or unstar any row from Home. No collection or bookmark management screen remains in the UI.

Areas: `home.rs`, `bookmarks.rs`, `entry.rs`, `store.rs`, and `route.rs`.

### UI-05: Shorten quick lookup within long rules

- [x] Carry the matched rule heading into search results when it identifies the requested section.
- [x] Open `grapple` at Grapple within Special Attacks instead of the article introduction.
- [x] Add an article contents control for long rules so the complete subsystem remains readable and navigable.
- [x] Preserve stable entry IDs, anchored URLs, Back and Forward, and book previous/next navigation.

Acceptance: Rules text search for `grapple` opens the correct section directly. The reader can still reach the whole article and neighboring sections offline. A title match and an entry without heading matches still open normally.

Evidence: the simulator found Special Attacks first with Rules text, but opened at Aid Another. Any serialized search or content change must follow the existing format-version rules.

### UI-06: Keep core class rules easy to reach

- [x] Reduce the expanded archetype list's prominence above the main class rules. Use a compact summary, disclosure, or direct jump while keeping options discoverable.
- [x] Keep class and archetype relationships, source grouping, and parent links available.

Acceptance: Fighter's core information is reachable without passing 23 expanded archetype links. Archer remains easy to discover from Fighter and links back to it.

### UI-07: Make repeated names easier to distinguish

- [x] Review trait lists and search rows that repeat the same name from different publications.
- [x] Keep source labels easy to scan. Consider a grouped name with source variants only if distinct text and IDs remain clear.
- [x] Apply active-book filtering before presenting source variants. Do not merge distinct rules silently.

Acceptance: the combat-trait list makes the Advanced Player's Guide and Ultimate Campaign copies of Anatomist distinguishable before opening either one. A user can still star the intended entry.

### UI-08: Explain the Safari-to-Home-Screen transition

- [x] Explain Home Screen installation before users download and save substantial research in Safari.
- [x] Explain that a Home Screen app may need its own library download and may not carry over Safari's stars, as observed in this simulator run.
- [x] Make app installation, library download, app-shell update, and library update distinct actions in the copy.
- Follow-up ownership: [PR #7](https://github.com/phynics/cast-iron/pull/7) implements portable stars. Do not create another export/import task; it is not part of this historical baseline.

Acceptance: a new iPhone user can tell where to install and download, and understands what may happen to existing Safari data. Verify the final wording on a physical iPhone before claiming behavior across iOS versions.

### UI-09: Verify the combined journeys

- [x] Update affected acceptance tests on a real `dist/` build. Replace collection-manager assertions with Home star and upgrade assertions.
- [x] Cover category-only Browse, book-only Books, rule search and heading jumps, book activation, zero active books, and old-route recovery.
- [x] Verify that app and library updates preserve stars and active-book preferences. Preserve verified-library activation and interrupted-download behavior.
- [x] Exercise character research through a race, traits, class, archetype, spell list, equipment, and character-building rules.
- Remaining acceptance belongs to [#12](https://github.com/phynics/cast-iron/issues/12): complete physical-device subsystem reading with links, history, and book navigation. PR #7 records a separate manual browser Armor journey.
- [x] Inspect light and dark layouts at phone and desktop widths, including long titles, many stars, expanded book branches, and controls near the fixed tab bar.
- [#12](https://github.com/phynics/cast-iron/issues/12) owns direct keyboard, VoiceOver, focus, contrast, and touch-target acceptance. Screenshots alone do not establish these properties.
- [#12](https://github.com/phynics/cast-iron/issues/12) owns physical-iPhone offline standalone restart. Simulator status-bar airplane mode is insufficient.
- [x] Run `cargo xtask check`, affected browser tests, and any corpus upgrade checks required by the implementation. Record local, CI, deployment, and device results separately.

Dependency: UI-01 through UI-08, with targeted tests added alongside each change rather than deferred to the end.

Local evidence: native checks and the real distribution build pass. All 67 browser cases are verified across the full run and the corrected WebKit fixture rerun. The complete manual subsystem read, VoiceOver, and physical-iPhone restart remain open; screenshot and automated checks do not substitute for those checks.

### UI-10: Integrate the supplied Gemini visual review

- [x] Record the user's manually guided screenshot review and its static-image limits.
- [x] Map duplicate findings to UI-01 and UI-05 through UI-08. Implement primary-reading hierarchy, clear selected scopes, prestige requirement links, reference deduplication, Home layout, and school labels.
- [x] Check disputed findings against code. Metadata badges were already links. Differently settled screenshots alone do not establish a filter-state bug.

Evidence: [Gemini follow-up](research/gemini-ui-review.md). The earlier blocked tool attempt remains part of the history; the supplied completed review supersedes it. Runtime and assistive checks remain separate.
