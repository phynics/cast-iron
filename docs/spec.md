# Cast Iron (PF1 Reference): Technical Specification v0.1

Status: Proposed
Target: Offline-first Progressive Web Application
Platform: Android, iOS, desktop browsers
Hosting: GitHub Pages
Language: Rust
Reference application: Masterwork Tools: Pathfinder Open Reference

This file holds the specification as the project owner wrote it. The working name "PF1 Reference" means the project "Cast Iron". Record later decisions in `docs/decisions.md`. Do not rewrite this file to match the implementation.

---

## 1. Product definition

PF1 Reference is an installable, fully offline rules reference for Pathfinder First Edition.

The application provides fast access to the complete legally redistributable first-party PF1e rules corpus, including structured navigation, full-text search, filtering, cross-references, and personal bookmarks.

It is a reference application, not a game management system.

### Core principles

1. Offline operation is mandatory. After initial installation, no application feature requires a network connection.
2. Complete corpus. The intended distribution includes all available, legally redistributable official PF1e rules content rather than a limited selection of books.
3. Static distribution. GitHub Pages hosts compiled files and data. No backend is required.
4. Immutable source material. Users cannot modify canonical rules, but can maintain local bookmarks and collections.
5. Fast reference workflow. Finding and opening an entry should require minimal interaction.
6. No external runtime dependencies. No CDN resources, API calls, telemetry, authentication, or remote fonts.

## 2. Functional requirements

| ID | Requirement | Priority |
|---|---|---|
| FR-01 | Browse rules by category | Required |
| FR-02 | Browse rules by publication | Required |
| FR-03 | Search titles and full text offline | Required |
| FR-04 | Filter entries using structured fields | Required |
| FR-05 | Render complete, formatted rules entries | Required |
| FR-06 | Navigate internal rules cross-references | Required |
| FR-07 | Save and remove bookmarks | Required |
| FR-08 | Organize bookmarks into collections | Required |
| FR-09 | Support navigation history and deep links | Required |
| FR-10 | Support light/dark themes | Required |
| FR-11 | Install and launch as an offline PWA | Required |
| FR-12 | Verify complete data installation | Required |
| FR-13 | Preserve local data across application updates | Required |
| FR-14 | Export/import user bookmarks | Later |
| FR-15 | Notes and annotations | Later |

### Content categories

The application should accommodate:

- Core rules, conditions, skills, combat, magic, and subsystems
- Classes, archetypes, class features, and prestige classes
- Races, alternate racial traits, and character options
- Feats, traits, and drawbacks
- Spells, spell lists, domains, and bloodlines
- Monsters, NPCs, templates, and related statistics
- Weapons, armor, adventuring gear, and magic items
- Other official rules-bearing material, organized by source

Not every publication is guaranteed to be covered by a redistributable source. Any unavailable or excluded material must appear in a corpus coverage report rather than silently disappearing.

### Explicit non-goals

Character building, character sheets, virtual tabletops, combat trackers, dice rolling, online accounts, synchronization, and community-submitted homebrew are outside the initial scope.

## 3. Technology stack

| Layer | Selected technology |
|---|---|
| Frontend | Rust + Leptos (client-side rendering) |
| WASM build | Trunk |
| Styling | CSS with design tokens |
| Serialization | Serde |
| Content processing | Native Rust command-line compiler |
| Search | Rust search library shared by native and WASM targets |
| Rules persistence | Browser Cache API |
| User persistence | IndexedDB |
| Service worker | Vanilla JavaScript |
| PWA manifest | Web App Manifest |
| Routing | Hash routing |
| Testing | Rust tests + Playwright |
| Continuous integration | GitHub Actions |
| Deployment | GitHub Pages |

No Rust server, SQL server, SSR framework, or production Node.js runtime.

JavaScript is limited to browser integration where it is simpler or required, including service workers, installation, and storage interoperability.

## 4. High-level architecture

The system consists of two independently built parts.

### A. Content compiler (native Rust)

Inputs:

- Publicly available PF1e source datasets
- Human-readable source records
- Publication metadata and license notices
- Optional curated corrections and mappings

Processing:

1. Parse each source format.
2. Normalize entries into a canonical schema.
3. Resolve duplicate entries and conflicting source records.
4. Generate stable identifiers.
5. Preserve source attribution and legal metadata.
6. Extract structured fields.
7. Resolve cross-references.
8. Generate searchable text and index structures.
9. Validate all produced records.
10. Output versioned content and indexes.

Outputs:

- Content manifest
- Categorized or partitioned rule records
- Search index
- Sourcebook index
- Cross-reference index
- Integrity hashes
- Coverage and validation reports
- Applicable licenses and legal notices

The content compiler runs on developer machines and in CI, never on the user's device.

### B. Client application (Rust/WASM)

Responsibilities:

- Render the user interface.
- Access locally cached rules data.
- Run full-text and field-aware searches.
- Filter and sort entries.
- Render rich rules content.
- Resolve internal navigation links.
- Maintain bookmarks and collections.
- Detect offline readiness.
- Coordinate user-initiated data updates when online.

All reference operations are local.

## 5. Canonical data model

The compiler outputs normalized, versioned records.

### RuleEntry

| Field | Type | Meaning |
|---|---|---|
| id | String | Stable canonical identifier |
| kind | EntryKind | Spell, feat, creature, item, etc. |
| title | String | Display name |
| source_ids | String[] | Original publications |
| section | String? | Rules hierarchy location |
| body | StructuredContent | Formatted rules body |
| fields | Typed metadata | Searchable mechanical properties |
| tags | String[] | Categories and classifications |
| links | EntryReference[] | Resolved internal references |
| provenance | Provenance[] | Origin dataset and source record |
| license_refs | String[] | Applicable licensing references |

### Type-specific metadata

Spell: school, subschool, descriptors, classes and spell levels, casting time, range, target, duration, saving throw, spell resistance.

Feat: feat category, prerequisites, benefits, normal text, special text.

Creature: type, subtype, size, CR, alignment, AC, hit points, saves, attacks, special abilities, and statistics.

Item: item category, price, weight, slot, aura, caster level, and other applicable attributes.

Class: hit die, skill ranks, class skills, progression tables, class features, and archetype relationships.

Complex entries must preserve tables, headings, formulas, lists, emphasis, and stat blocks.

The schema should not attempt to force every rules category into identical mechanical fields. The common structure handles navigation and indexing; typed extensions handle category-specific data.

### Identifier strategy

Canonical IDs must remain stable across library rebuilds. Book order, record position, and display title alone must not determine identity.

The compiler maintains explicit mappings for collisions, aliases, renamed records, and equivalent entries.

## 6. Content acquisition and licensing

### Candidate datasets

- PSRD-Data, from the original Masterwork Tools ecosystem
- Pf Data 1e structured JSON
- Other independently licensed PF1e datasets
- Official reference sources used for validation and gap analysis

Every imported source requires an audit of coverage, field quality, data rights, and upstream licensing.

### Licensing requirements

The content pipeline must:

- Identify Open Game Content and retained notices.
- Preserve applicable OGL attribution and license text.
- Track Product Identity and other excluded content.
- Account for Paizo's Community Use Policy where applicable to the free non-commercial tool.
- Respect licenses of third-party datasets, not just Paizo's licenses.
- Avoid redistributing unsupported artwork, setting text, or proprietary assets.
- Generate a readable attribution and legal-information page.

Human-readable source data remains available in the repository wherever its license requires this. Compiled binary indexes are derived artifacts, not substitutes for source attribution.

## 7. Search specification

Search is local, deterministic, and runs without network access.

### Required behavior

- Title and full-text search
- Case-insensitive matching
- Normalization of punctuation and whitespace
- Prefix matches
- Ranking with strong title weighting
- Category and sourcebook filtering
- Type-specific field filters
- Search result previews
- Highlighted matches

### Search index

The compiler generates an inverted index from normalized records.

The search engine is implemented as a shared Rust library that can compile for both native tests and browser WASM.

A ranking algorithm such as BM25 can be used, with additional boosts for exact titles, title prefixes, and aliases.

The index should be structured to permit selective loading if full-corpus memory use is excessive.

### Search evolution

The POC establishes correct indexing, matching, and ranking. Production optimization follows actual measurements of corpus size, WASM memory use, and mobile responsiveness.

Fuzzy matching and typo tolerance can be added after exact and prefix search performance is verified.

## 8. Offline installation and persistence

### Initial installation

1. The user opens the GitHub Pages application.
2. The application registers its service worker.
3. The installer displays the content manifest and required download size.
4. The complete rules library and search indexes are downloaded.
5. Downloaded artifacts are verified against the manifest.
6. The verified content version is activated.
7. The application reports that it is ready for offline use.

A partially installed library must never be reported as complete.

### Runtime behavior

Once the installation is complete:

- UI assets are loaded from the service-worker cache.
- Rules content is retrieved locally.
- Search executes locally.
- Bookmarks and preferences use IndexedDB.
- No app-initiated network requests occur during ordinary reference use.

### Storage separation

Cache API: application shell, compiled WASM, rules content, search indexes, and static resources.

IndexedDB: active library version, installation metadata, bookmarks, collections, preferences, and user state.

### Updates

Content updates are optional and explicitly initiated while online.

A new content version is downloaded into a separate cache namespace and verified before being activated. The previous working version remains available until the replacement is confirmed.

Bookmarks use stable record IDs and survive content changes.

### Browser storage limitations

The application should request persistent storage where supported and display storage diagnostics.

Browser storage can still be cleared by the user or affected by platform-specific policies. Therefore, even a successfully installed PWA cannot promise permanent offline retention.

## 9. User interface

### Main screens

| Screen | Responsibilities |
|---|---|
| First-run installer | Download, verify, activate offline library |
| Home | Search entry point, categories, recent entries |
| Search | Query, filter, ranked results |
| Browse | Navigate categories and sourcebooks |
| Entry detail | Display complete formatted rules |
| Bookmarks | Favorites and custom collections |
| Settings | Theme, storage status, library version, licenses |

### Navigation

The app uses hash-based routes so GitHub Pages can always serve the same application entry point.

Navigation history must work using browser back and forward controls.

Layouts must support narrow mobile screens and larger desktop/tablet screens without requiring separate applications.

Accessibility should cover keyboard navigation, labels, semantic headings, screen-reader compatibility, and readable contrast.

## 10. Non-functional requirements

| ID | Target |
|---|---|
| NFR-01 | All core functionality works in airplane mode after installation |
| NFR-02 | No external runtime resources or APIs |
| NFR-03 | Search should feel immediate on mid-range mobile hardware |
| NFR-04 | Long rules entries render without noticeable freezing |
| NFR-05 | Content builds are deterministic and reproducible |
| NFR-06 | Malformed or missing records fail validation visibly |
| NFR-07 | Interrupted downloads never corrupt the active library |
| NFR-08 | Bookmarks survive updates and normal application restarts |
| NFR-09 | GitHub Pages project-path deployment works correctly |
| NFR-10 | Published assets stay within hosting and browser storage limits |

Initial engineering performance targets, subject to measurements:

- Typical search response: under 100 ms after index loading
- Search interaction must not noticeably block UI rendering
- Core app startup: approximately 2 seconds or less on a representative mid-range phone after caching
- Complete offline installation must be verifiable, not inferred from download completion alone

---

# Proof of Concept Implementation Plan

## POC objective

Prove that a Rust/WASM application hosted on GitHub Pages can install a structured PF1e library, search it locally, render linked rules, preserve user bookmarks, and relaunch without network access.

The POC is an architectural validation, not a partial release of the final corpus.

## POC dataset

Use a representative subset of legally reusable PF1e records containing:

- Spells
- Feats
- Creatures
- Examples of nested rules sections
- Tables, stat blocks, prerequisites, and cross-references

Aim for at least 1,000 representative records if the selected source supports them. Include difficult formatting examples rather than selecting only simple entries.

## Phase 1: Repository and toolchain

Tasks:

1. Create a Rust workspace.
2. Initialize a Leptos client-side application with Trunk.
3. Add the native content-compiler crate.
4. Add a shared schema crate.
5. Add a shared search crate.
6. Configure formatting, linting, tests, and WASM builds.
7. Establish GitHub Actions CI.

Deliverable: A minimal Rust application serving successfully through Trunk, with both native and WASM builds passing CI.

Exit criterion: The app renders locally and its static release assets can be published to GitHub Pages.

## Phase 2: Data ingestion vertical slice

Tasks:

1. Select one legally usable input dataset.
2. Implement its source adapter.
3. Define canonical entry types.
4. Normalize representative records.
5. Preserve original source identifiers and attribution.
6. Generate stable entry IDs.
7. Build a validation and error-reporting mechanism.
8. Emit a versioned content manifest and records.

Deliverable: A deterministic compiler producing a small, structured PF1e content library.

Exit criterion: The same inputs produce equivalent outputs; all selected records validate; provenance and rejected records are reported.

## Phase 3: Search engine

Tasks:

1. Define text normalization and tokenization.
2. Build a native inverted-index generator.
3. Implement title and full-text search.
4. Add title weighting and prefix matching.
5. Add basic category filtering.
6. Compile the query engine for WASM.
7. Benchmark native and browser execution.

Deliverable: Searchable local data with a shared Rust implementation.

Exit criterion: Search works with the browser disconnected, returns correct results, and meets the initial responsiveness target on the POC dataset.

## Phase 4: Core reference interface

Tasks:

1. Implement home and category navigation.
2. Create a search screen with live results.
3. Build reusable rules-entry components.
4. Render formatted text, tables, and stat blocks.
5. Support internal cross-reference links.
6. Add sourcebook information.
7. Implement hash navigation and history.
8. Add a mobile-responsive layout.

Deliverable: A usable PF1e reference browser.

Exit criterion: Users can find a spell, open it, follow a reference, return using browser history, and browse related entries without network access.

## Phase 5: Offline installation

Tasks:

1. Add the web manifest and install metadata.
2. Add a versioned service worker.
3. Cache the static application.
4. Build a first-run library installer.
5. Implement content download and hash verification.
6. Store installation metadata.
7. Activate the verified library.
8. Handle interrupted downloads and incomplete content.
9. Request persistent storage where supported.

Deliverable: A fully installable and verified offline PWA.

Exit criterion: After installing the complete POC dataset, the app launches and operates with the browser network disabled, including after a fresh tab or installed-app restart.

## Phase 6: Bookmarks and local state

Tasks:

1. Define the IndexedDB schema.
2. Implement bookmark CRUD operations.
3. Add bookmark collections.
4. Persist theme and user preferences.
5. Test data persistence after application restarts.
6. Simulate a content update without deleting user records.

Deliverable: Persistent local user state.

Exit criterion: Bookmarks and collections survive offline restarts and compatible library updates.

## Phase 7: GitHub Pages deployment and validation

Tasks:

1. Configure Trunk's production public URL.
2. Configure the correct service-worker scope.
3. Build and publish through GitHub Actions.
4. Test a clean browser installation from GitHub Pages.
5. Test offline launch and navigation.
6. Test download interruption and recovery.
7. Measure WASM size, dataset size, search latency, and memory.
8. Test Android Chromium, desktop Chromium/Firefox, and iOS Safari where devices are available.

Deliverable: Publicly deployable POC with documented test results.

Exit criterion: All core user journeys pass on the deployed site, including disconnected use.

## 11. POC acceptance checklist

- [ ] Static GitHub Pages deployment works.
- [ ] Application is installable where browser support allows.
- [ ] Representative rules dataset installs completely.
- [ ] Download integrity is verified.
- [ ] Search operates entirely offline.
- [ ] Category and sourcebook browsing works.
- [ ] Rule entries preserve complex formatting.
- [ ] Cross-reference navigation works.
- [ ] Back/forward navigation works.
- [ ] Bookmarks persist across restarts.
- [ ] Interrupted installation can recover safely.
- [ ] Offline launch works after a new browser session.
- [ ] No application feature requires a runtime server.
- [ ] The compiler generates repeatable artifacts.
- [ ] Licensing information is preserved and accessible.

## 12. Post-POC implementation

After the POC is accepted, the primary work becomes corpus completeness and production hardening.

Content work: Ingest remaining source datasets, compare publication coverage, resolve conflicts and duplicates, verify licensing, repair malformed content, and generate a complete coverage report.

Performance work: Benchmark the full search index, reduce memory consumption where needed, move expensive operations off the UI thread if required, and optimize data partitioning and load times.

Product work: Complete all category-specific filters, bookmarks, publication navigation, accessibility, responsive layouts, and additional reference workflows.

Release work: Validate full offline installation across target browsers, ensure deterministic content releases, publish licensing notices, and establish a reliable manual update mechanism.

## 13. Principal technical risks

| Risk | Mitigation |
|---|---|
| Full dataset unavailable in one source | Multi-source ingestion and coverage audit |
| Inconsistent or duplicate entries | Canonical normalization and provenance tracking |
| Content licensing conflicts | Per-source licensing audit and exclusion reporting |
| Large search-index memory use | Benchmarks, compact postings, partitioned loading |
| Mobile storage limitations | Quota checks, resumable installation, persistence request |
| Browser-specific PWA behavior | Cross-browser offline acceptance testing |
| Rust/WASM browser API friction | Small JavaScript interoperability layer |
| Content update breaks local references | Stable IDs, schema versioning, migration tests |

## 14. Proposed repository structure

- `Cargo.toml`: Rust workspace
- `crates/schema/`: canonical rules types and validation
- `crates/content-compiler/`: native CLI and source adapters
- `crates/search/`: index generation and query engine
- `crates/web/`: Leptos application and UI
- `web/`: index HTML, service worker, manifest, styles, icons
- `sources/`: source manifests, import definitions, curated corrections
- `data/`: normalized human-readable content where redistribution is permitted
- `generated/`: build-generated artifacts, excluded from source control when appropriate
- `tests/`: fixtures, browser tests, and integration tests
- `.github/workflows/`: build, validation, and deployment
- `LICENSES/`: code, content, and upstream licensing notices

## 15. Initial implementation order

The recommended sequence is:

Schema → Content compiler → Search → Entry renderer → PWA installation → IndexedDB bookmarks → GitHub Pages deployment → Offline acceptance tests.

A small verified corpus should pass this entire pipeline before importing the full PF1e rules collection.

The POC is successful when it proves the offline architecture end to end, not when it merely displays a sample rule.
