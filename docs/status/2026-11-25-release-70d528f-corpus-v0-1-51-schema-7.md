---
title: "Release `70d528f` (corpus v0.1.51, schema 7)"
date: 2026-11-25
kind: release
---

# Release `70d528f` (corpus v0.1.51, schema 7), 2026-11-25

`release` was promoted from `2b56cc1` to `70d528fc78fb1e73cde3b63ba0b8de78202c4587` with
`scripts/promote-release.py --push`, a fast-forward push. **The library format changes: schema 7** (the haunt
kind, [decision](../decisions/2026-11-23-haunts-are-a-kind-schema-7.md)); the search index format stays 2. The
range is 59 first-parent merges, #638 to #717. It carries ten corpus pins, v0.1.41 to v0.1.47, v0.1.50 and
v0.1.51 ([v0.1.41](2026-11-23-pin-corpus-v0-1-41.md), [v0.1.42](2026-11-24-pin-corpus-v0-1-42.md),
[v0.1.43](2026-11-24-pin-corpus-v0-1-43.md), [v0.1.44](2026-11-24-pin-corpus-v0-1-44.md),
[v0.1.45](2026-11-24-pin-corpus-v0-1-45.md), [v0.1.46](2026-11-24-pin-corpus-v0-1-46.md),
[v0.1.47](2026-11-24-pin-corpus-v0-1-47.md), [v0.1.50](2026-11-25-pin-corpus-v0-1-50.md) (which holds v0.1.48
and v0.1.49) and [v0.1.51](2026-11-25-pin-corpus-v0-1-51.md)), and app changes. The app shell and the library both
changed, so the shell stamp and the content version are new. This is the first release that serves the haunt kind.

| Field | Value |
|---|---|
| Release commit | `70d528fc78fb1e73cde3b63ba0b8de78202c4587` |
| Previous release | `2b56cc1380422f2cff83ef3e9d3821fa0fead45e`, content `v3f3d0b708758820d` (schema 6, corpus v0.1.40) ([entry](2026-11-23-release-2b56cc1-corpus-v0-1-40-schema-6.md)) |
| Content | `vad43d1e1b6e40e61`, schema 7, index format 2, 31,672 entries, 443 publications |
| Library size | 215,677,047 bytes in 277 files, from the live manifest (about 216 MB, 206 MiB; the previous release was 217,014,990 bytes in 280 files, so 1,337,943 bytes smaller and three files fewer) |
| Shell | `2d4770843b89f710`, released 2026-11-25 (the previous release stamped `114f05ccdf222d2a`) |
| WASM | `cast-iron-web-c3ea433637d8faf0_bg.wasm`, 2,585,921 bytes (the live `content-length` with `Accept-Encoding: identity`, and the length of the decoded body a browser fetched; Pages sends it gzip-encoded at 1,073,029 bytes on the wire; the previous release shipped 2,490,876, so 95,045 bytes larger) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.51`, commit `689deb0e0a290ea3c2eb952f636e2f218b71046f` |

**Local.** The schema upgrade check of [releasing.md](../releasing.md) ran before the promotion, by delegates, on
local builds of `2b56cc1` and `70d528fc7` served on one origin, in Chromium 156.0.8078.4 and WebKit 27.2. It
covers the schema 6 to schema 7 change, so it is the evidence that an installed profile survives it.

- *Run A, the upgrade path: 73 of 73 checks in each browser.* The old library was installed with the old app, then
  11 stars (one of them the haunt `ref/haunt/arcane-storm`, starred while it was of kind `rule`), 28 of 443 books
  active and a Continue Reading point were set. The new shell was served and the app update accepted. The old
  schema 6 library was read under the new shell (the haunt still reads as Rule). The library update was a separate
  action, and the old library stayed usable during the download. After activation the haunt star's stored kind
  snapshot is rewritten from `rule` to `haunt` with its id, title and `added_at` unchanged, and Home's Starred row
  reads "Haunt". `#/browse/kind/haunt` lists 45 with every book active and 36 under the 28-book profile. Stars,
  active books and Continue Reading survive. The usual pages read with no raw markup: Goblin, Acolyte, Longsword,
  Bag of Holding, a trait, Power Attack, Power Attack, Mythic, Fireball, Fighter and the Core Rulebook page. An
  offline reload and an offline search work. No console error, page error, 4xx or 5xx response, or request to
  another origin. WebKit logged nine "WASM preloaded but not used" warnings, as at the previous release.
- *Chromium's first run of A passed 72 of 73.* The one failure was a mistake in the script, which expected 45
  haunts under a profile with three haunt books off. The second run passed 73 of 73.
- *Run B, the interrupted download: 79 of 79 checks in each browser.* The server cut the library update after about
  101 of 277 files (about 27%). The UI showed a `role=alert` error that names the failed file ("`...creature-029.json`:
  network error (TypeError: Failed to fetch) (after 3 attempts)", "Load failed" in WebKit) with a Retry button.
  `active_library` stayed `v3f3d0b708758820d`, schema 6. The partial cache held no stored manifest and was not
  counted as installed. The old library, the haunt read as Rule and the stars were intact, and an offline reload
  worked. After the network came back, Retry in the UI fetched only the missing files (176 in one browser, 177 in
  the other) and activated `vad43d1e1b6e40e61`, with the haunt snapshot `haunt`. 0 page errors and 0 third-party
  requests; the nine aborted requests and their console errors were the deliberate cut.
- *Publication screen* on the new `dist/`: exit 0, 0 issues, 31,672 entries, 443 sources, 16 kinds, 1,690 entries
  with Community Use names.
- *Two caveats.* The local build of `2b56cc1` stamped shell `dac1dfa4ed7f4b41`, not the deployed `114f05ccdf222d2a`;
  the cause is likely a build-path difference and was not checked. And the path in which the old shell had already
  run a library check before the app update was not exercised.

The pin entries record the local checks of their pin PRs, and the app PRs record their own. The record you are
reading is a docs-only change and needs `cargo xtask check` alone.

**CI.** Main full CI run [45550179604](https://github.com/phynics/cast-iron/actions/runs/45550179604) at `70d528f`
(started with `workflow_dispatch` on `main`) concluded `success`. All 14 jobs passed: Rust checks, Content compiler,
Content determinism (NFR-05), Build the site, the four Chromium shards, every WebKit shard (informational) and the
two report jobs.

**Deployment.** Pages run [45580167145](https://github.com/phynics/cast-iron/actions/runs/45580167145) succeeded for
`headBranch: release` at `70d528fc7` (event `push`, 2026-11-25, 5 min 12 s from its creation at 08:52:46Z; the build
job ran 08:52:50 to 08:56:34Z and the deploy job 08:56:39 to 08:57:58Z; both `success`). It was the newest Pages run
for `release` and the only one for this push. `gh api repos/phynics/cast-iron/pages` reports `build_type: workflow`
and `https_enforced: true`, and the `github-pages` environment allows the one branch `release`. `origin/release` is
at `70d528fc78fb1e73cde3b63ba0b8de78202c4587`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-25, after the deployment, with a throwaway
Playwright script (Chromium 156.0.8078.4 and WebKit 27.2, a fresh profile for each run; the script is not
committed). The live `content/manifest.json` reads `schema_version: 7`, `content_version: vad43d1e1b6e40e61`,
`entry_count: 31672`, `total_bytes: 215677047`, `corpus_tag: v0.1.51`, 277 files. The live `sw.js` stamps
`SHELL_VERSION = "2d4770843b89f710"` and `SHELL_DATE = "2026-11-25"`.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron"); a fresh profile offers **Download** | pass | pass |
| `content/manifest.json`: content `vad43d1e1b6e40e61`, schema 7, 31,672 entries, corpus tag `v0.1.51`, 215,677,047 bytes in 277 files | pass | pass |
| `sw.js` stamp `2d4770843b89f710`, 2026-11-25; shipped `*_bg.wasm` 2,585,921 bytes | pass | pass |
| Every script and link URL of the document is under `/cast-iron/` (7 assets); no same-origin request outside that path | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens; the active library record reads `vad43d1e1b6e40e61`, schema 7 | pass, 18 s (30 s in the first run) | pass, 14 s (25 s in the first run) |
| The service worker controls the page after the install | pass | pass |
| Goblin (`ref/monster/goblin`): heading, a Defense section, no raw markup | pass | pass |
| Acolyte (`ref/npc/acolyte`): a spell block with `Caster level` and `Concentration` rows, no raw markup | pass | pass |
| Longsword (`ref/eq-weapon/longsword`): the Weapon Group row links to `ref/ability/weapon-groups?a=sec-heavy-blades`; following it opens Weapon Groups | pass | pass |
| Bag of Holding (`ref/magic-wondrous/bag-of-holding`): a Construction block, no raw markup | pass | pass |
| Reactionary (`ref/trait/reactionary`): `Sources:` lines with Ultimate Campaign p. 53 and Advanced Player's Guide p. 328 | pass | pass |
| Power Attack (`ref/feat/power-attack`): `Sources: Core Rulebook p. 131` | pass | pass |
| Power Attack, Mythic (`ref/carried/mythic-adventures/feats/power-attack/mythic`): `Sources: Mythic Adventures p. 71` | pass | pass |
| Fireball (`ref/spell/fireball`), Fighter (`ref/class/fighter`) and the Core Rulebook book page open, no raw markup | pass | pass |
| **Haunts:** `#/browse/kind/haunt` heading "Haunts", "45 haunts", 45 rows | pass | pass |
| **Haunts:** `ref/haunt/arcane-storm` opens, no raw markup, the metadata line reads `Haunt · Horror Adventures` | pass | pass |
| **Haunts:** Home's "More categories" links Haunts (count 45) to `#/browse/kind/haunt` | pass | pass |
| Offline: reload on Goblin with the network off shows the entry | pass | pass |
| Offline: Bag of Holding opens from the cache with its Construction block | pass | pass |
| Offline: the haunt list shows 45 rows | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell (50 hits) | pass | pass |
| Page errors and console errors during the final run | none | none |
| Responses with a status of 400 or more | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper and the
previous release check do. The install used the Download button of a profile without a library. The checks read
the rendered page text, link targets and attributes; they did not compare layout or styling. The WASM size is
the decoded body length of the response the browser received for the `*_bg.wasm` URL of the document.

- **Two runs, honestly.** The first run passed 26 of 29 checks in Chromium and 28 of 29 in WebKit. In both browsers
  the failing check was Power Attack, Mythic: **a mistake in my script**, which guessed the id `ref/feat/power-attack-mythic`
  instead of the carried id above (the app's own specs use the carried id). In Chromium alone, the install also
  took a **503 response from the host** for `content/vad43d1e1b6e40e61/entries/creature-045.json`; that response was
  logged as a console error and a response of 400 or more, so those two rows failed. The page was not at fault, but
  I did not read the installer's retry itself: the install completed and every later check passed. The second
  run, after fixing the id, passed 29 of 29 in each browser, with no console error, no page error, no response
  with a status of 400 or more and no third-party request. The 503 did not recur.

**Not run.**

- No check on the real iPhone ([#12](https://github.com/phynics/cast-iron/issues/12), complete physical-device and
  accessibility acceptance) and no Home Screen app. The release was checked with Chromium and WebKit in a browser only.
- The path in which the old shell had already run a library check before the app update was not exercised (see
  Local). The upgrade check ran on local builds before the promotion, not on the live site, which no longer serves
  the old shell. The local old build's shell stamp differed from the deployed one (see Local).
- The live check read the pages named above. It did not open the other pages whose statistics, citations or
  feature heads moved (the 365 archetypes converted to printed prose, 45 haunts beyond Arcane Storm, stat sets), or
  check styling at a phone width, in dark mode or in forced-colors mode, with large text, or the new breadcrumbs,
  tab highlight and Other printings line.
- No daily update check, Settings page or Find a book run on the live site.
- `cargo xtask published-ids` was not run for the release itself (the command needs a built library, and this
  record is docs only). The published-id guard of `cargo xtask content` passed at each pin, and the pin entries
  record that all 70 retired ids are in `sources/published-ids.txt` and resolve through `aliases.json`.
- No iOS Simulator session for this release.

**Known open issues.**

- [#12](https://github.com/phynics/cast-iron/issues/12): physical-device and accessibility acceptance. The app side
  of the audit (parts A to F and the large-text finding) is in this release; the real-device check is the owner's.
- [#540](https://github.com/phynics/cast-iron/issues/540): archetype, class-option and base-class features as
  printed prose. The release carries the Advanced Player's Guide (81), Ultimate Magic and Ultimate Combat (126) and
  Advanced Class Guide and Advanced Race Guide (158) archetypes at the pins above, with the corpus-check reader
  changes up to #716. Later batches are on `main` or in the corpus (the v0.1.53 pin finishes the Advanced Class Guide
  and Advanced Race Guide) and are not published.
- [#461](https://github.com/phynics/cast-iron/issues/461): book memberships a book does not print. Slices S5 to S12
  are in this release; the warning for memberships without a page citation is 1 at v0.1.51 (Gamemastery, in the
  GameMastery Guide). [#715](https://github.com/phynics/cast-iron/pull/715) (an uncited membership becomes a
  corpus error, with an allow-list) merged to `main` after this release, so the release does not carry it; the
  v0.1.53 pin on `main` brings the allow-list file.

## Release notes

Changes since the release at `2b56cc1` (corpus v0.1.40, 2026-11-23). The release pins corpus v0.1.51.

### Upgrading

**The library format changes to schema 7, so the library must be installed again from Settings.** The app update
comes first; the app offers a reload. The library update is a separate action and the app does not download it by
itself. The old library stays in use, and fully readable, until the new one has passed its hash checks, and an
interrupted download leaves the old library active; Retry fetches only the files that are missing. The new library
is about 216 MB (206 MiB), 1.3 MB smaller than before. **Stars, active books and Continue Reading survive** both the
app update and the library update (checked before the release in Chromium and WebKit; see Local). **A starred haunt
becomes a Haunt**: haunts were rules in schema 6, and the stored kind of a haunt star is rewritten when the new
library is activated; its title and date stay. Until then, the old library shows the haunt as a Rule.

### Changes to the app

- **Haunts are a kind of their own** (schema 7, #662). Home lists Haunts under More categories, the Kind filter has
  a Haunts row, a book's In this book names its haunts, and a haunt page reads `Haunt` in its metadata line. The
  45 haunts keep their ids.
- **Monster stat sets.** A monster that prints several statistics lists each set on its entry page, and the creature
  filters match any set's challenge rating (#643). Air, Aether, Earth, Fire, Ice, Lightning, Magma, Mud and Water
  Elemental have six sets each in this library.
- **Entry pages and navigation.** An *Other printings* line links the other books that print a rule (#659). Feature
  heads show `Level N` and class features have search aliases (#681). Entry-link rows name their book (#663). The
  masthead tab that holds the route is highlighted (#658) and breadcrumbs list the pages between the tab and the
  entry (#664). A citation whose page is not a number prints without `p.` (#667).
- **Feat chains.** The page shows every descendant (#653), folds long chains and opens a chain from the address
  (#660), and has the list-page head with a search scoped to feats (#674).
- **Search and lists.** Search prefers the latest printing of a rule kept in several books (#639); the Kind filter
  is a multi-select menu (#655); a book-section hit ranks below an entry with a close score (#694). List folds
  order their printings like search (#654), and a modifier title (Greater X, Improved X) sits after X (#709). The
  spell row density *Wide* is named *Expanded* (#697).
- **Accessibility** ([#12](https://github.com/phynics/cast-iron/issues/12)). Keyboard focus clears the fixed bars
  and the targets are 44 px (#699). The installer moves focus sensibly and announces progress less often (#700).
  Search has a keyboard path and Quick search has ARIA roles (#702). The star keeps a fixed name, control edges
  have 3:1 contrast, and the storage request gives feedback (#703). Entry headings never skip a level (#707). At a
  large root text size the tab bar and the stat values fit, and the app follows iOS Dynamic Type (#711). The star
  keeps keyboard focus while it saves (#638). The colophon's two hit areas are apart (#705).
- **Settings and install.** Storage sits below Install on Settings, so a late storage answer cannot move Install
  (#675). A first install never announces an app update (#685).

### Changes to the library

Entries: **31,742 to 31,672 (70 fewer)**. Every one of the 70 is a duplicate that resolves through `aliases.toml`
to a canonical entry, so no id is lost. They left in three pins:

- **7 in v0.1.42** (basis `shared`): the carried Rabies and Ghoul Fever, the carried Shared Wrath and Tireless
  Pursuers, the `eq-reagent` Dragon's Gut and Odor Stalk, and the Ultimate Equipment Madu. By kind: affliction 468 to 466,
  item 8,100 to 8,097, spell 3,088 to 3,086.
- **2 in v0.1.43**: the carried Advanced Player's Guide traits `divine-courtesan` and `favored-son-daughter`
  (trait 1,937 to 1,935).
- **61 in v0.1.44**: 57 carried spells with inverted titles (`Restoration, Greater`, `Reduce Person, Mass`, ...)
  that duplicated their Pathfinder Data twins (`Greater Restoration`, `Mass Reduce Person`, ...), and 4 carried
  madnesses of the GameMastery Guide. Spell 3,086 to 3,029, affliction 466 to 462.

7 + 2 + 61 = 70. The pins v0.1.45 to v0.1.51 add or delete no entry. The published-id guard passed at each pin, and
the pin entries record that the retired ids are in `sources/published-ids.txt`. The 45 haunts moved from `rule` to
`haunt` without leaving the count. The library keeps 443 publications and 0 broken links.

- **Haunts** (v0.1.43). 45 entries, 26 of them in Horror Adventures. Two further changes of that pin: 35 disease
  Effect lines follow print, and five NPCs with mounts have two stat sets.
- **Stat blocks and sets** (v0.1.41). Tech, miscellaneous equipment, ioun stones, wondrous items, spellbooks,
  weapons, artifacts and siege engines state their statistics in stat blocks, with new `Arcane spell failure`,
  `Category`, `Price` and `Weight` rows. Twelve creatures have two or more stat sets. The seven planes
  (`Heaven`, `Nirvana`, `Elysium`, `Axis`, `The Maelstrom`, `Hell`, `Abaddon`) have their printed titles.
- **Citations** (v0.1.41 to v0.1.43). More raw `**Sources:**` lines become `[[sources]]` rows; 355 rows gain a page
  in v0.1.42 and about 200 more in v0.1.43. Fourteen Faiths and Philosophies traits cite `inside cover`, which the
  Sources line prints as `, inside cover`.
- **Archetypes as printed prose** ([#540](https://github.com/phynics/cast-iron/issues/540), corpus format 4:
  v0.1.45 to v0.1.50). 365 archetypes become headed printed prose with `features` and trades: 81 of the Advanced
  Player's Guide (v0.1.46), 126 of Ultimate Magic and Ultimate Combat (v0.1.48) and 158 of the Advanced Class
  Guide and Advanced Race Guide (v0.1.49). Their feature heads show `Level N` and carry search aliases; trade
  names follow print. v0.1.50 removes five `features` rows (`armor-proficiency` from Beast Rider, Emissary and Gun
  Tank, `class-skills` from Beast Master and Trapper), so those five archetypes lose the feature head and the
  search alias.
- **Book memberships** ([#461](https://github.com/phynics/cast-iron/issues/461), v0.1.44 to v0.1.51). Class hubs,
  class mirrors, race hubs, standard racial traits, Figurines of Wondrous Power and Adventure Path items lose
  books that do not print them, so the page counts of many books drop (the Advanced Player's Guide from 1,567 to
  1,495 entry placements at v0.1.44, Horror Adventures from 628 to 567). 14 navigation hubs gain 137 entry links
  (v0.1.47), and Basics and Optional Combat Changes move into the Core Rulebook and Ultimate Combat. The warning for memberships without a page citation falls
  from 43 to 1 in v0.1.51.
- **Print fixes.** 16 titles follow print and five spells are corrected (v0.1.44); Sound Striker, Crypt Breaker, five
  Advanced Class Guide errata rows and others (v0.1.47, v0.1.50); the Serpentfolk Venom Fortitude DC is 15 (v0.1.42).

### For contributors

- **Corpus format 4** (#677) and the reader for archetype features and trade sentences, with a run of corpus-check
  rulings for #540: tolerance for the reader (#704), granted-thing subjects, prose and aside trades (#710), a sentence
  that starts after a parenthesis (#714), second-clause verbs and mid-phrase asides (#716), scaffold headings (#690)
  and the accepted printed slot value `arm or wrist` (#652). The drift check ends the normal part by publication (#641, #644).
- **Search golden.** The canonical ranking golden is repaired and each corpus pin refreshes it (#689); the
  comparison runs in the content job in CI (#696).
- **Flakes and specs.** The install button and WebKit update button races are fixed (#665, #685, #688), a Quick search
  level check compares centres (#708) and four specs wait for Quick search to close (#717).
- **Docs.** Decisions for haunts, breadcrumbs and tabs, other printings, the latest printing, monster stat sets,
  corpus format 4, feature heads, modifier-title order, the star name and control edges, and the trade rulings are in
  `docs/decisions/`; the v1 scope and licensing decisions of 2026-11-23 are recorded (#649), with the iOS Simulator
  acceptance of the previous release (#657), the integrated-browser measurements (#693) and the cross-check of
  the data sources (#669, #687). The previous release record is on `main` (#640).

### Known limits

- No real-device check ([#12](https://github.com/phynics/cast-iron/issues/12)).
- The #540 conversion of the remaining class options, base-class features and archetypes is not complete; entries
  not yet converted keep their earlier form.
- 19 class-option powers are exceptions that the check does not hold to the canonical form
  (`power-exceptions.toml`).
- The upgrade check did not cover the order in which the old shell had run a library check before the app update.
  The new shell would then show the raw content version in the library row.
