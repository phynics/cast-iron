---
title: "Release `2b56cc1` (corpus v0.1.40, schema 6)"
date: 2026-11-23
kind: release
---

# Release `2b56cc1` (corpus v0.1.40, schema 6), 2026-11-23

`release` was promoted from `9a6fb06` to `2b56cc1380422f2cff83ef3e9d3821fa0fead45e` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and search
index format 2, as in the previous release. The range is 35 first-parent merges: #569 (the record of the previous
release) to #636. It carries three corpus pins, v0.1.38 (#613), v0.1.39 (#625) and v0.1.40 (#636)
([v0.1.38](2026-11-22-pin-corpus-v0-1-38.md), [v0.1.39](2026-11-22-pin-corpus-v0-1-39.md),
[v0.1.40](2026-11-23-pin-corpus-v0-1-40.md)), and app changes. The app shell and the library both changed, so
the shell stamp and the content version are new.

| Field | Value |
|---|---|
| Release commit | `2b56cc1380422f2cff83ef3e9d3821fa0fead45e` |
| Previous release | `9a6fb061fb59c5e6c3ca8824169cc6df3d4f6569`, content `v28481793019cf75c` (schema 6, corpus v0.1.37) |
| Content | `v3f3d0b708758820d`, schema 6, index format 2, 31,742 entries, 443 publications |
| Library size | 217,014,990 bytes in 280 files, from the live manifest (about 217 MB, 207 MiB; the previous release was 224,578,783 bytes in 294 files, so 7,563,793 bytes smaller) |
| Shell | `114f05ccdf222d2a`, released 2026-11-23 (the previous release stamped `738b7855d23307b9`) |
| WASM | `cast-iron-web-beb5430dec0f06c3_bg.wasm`, 2,490,876 bytes (the live `content-length`, and the length of the bytes a browser fetched; the previous release shipped 2,783,498) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.40`, commit `ca316adfc791b7e090e76184a780496f60dcd861` |

**Local.** An upgrade check ran before the promotion with the new build served locally, in Chromium 156 and WebKit 27.2,
on an existing profile. It passed 65 of 65 checks in each browser, with no console error, no page error
and no request to another origin.

- *Steps.* Install the previous library (`v28481793019cf75c`) with the previous app; star 10 entries; leave 28 of 443
  books active; set a Continue Reading point. Serve the new build on the same origin. The app update is offered and
  accepted. The old library is read under the new shell. The library update is offered as a separate action; the
  old library stays usable while the new one downloads, and the new one is activated after the download
  completes (23 s in Chromium, 26 s in WebKit). The stars, the book choices and the Continue Reading point survive.
- *Pages read.* Goblin, Acolyte (the `Caster level` and `Concentration` rows), Longsword (the Weapon Group link to
  Heavy Blades), Bag of Holding (the Construction block), a trait and a class option with `Sources:` lines that
  give pages, Power Attack, Mythic (`Sources: Mythic Adventures p. 71`), Fireball, Fighter and the Core Rulebook
  book page render with no raw markup. An offline reload reads entries and searches.
- *Publication screen* on the new `dist/`: 0 issues, 31,742 entries, 443 sources.
- *Not covered:* the path in which the old shell had already run a library check before the app update. The new
  shell would then show the raw content version in the library row. That order was not exercised.

The pin entries record the local checks of their pin PRs, and the app PRs record their own. The record you are
reading is a docs-only change and needs `cargo xtask check` alone.

**CI.** Main full CI run
[45181702594](https://github.com/phynics/cast-iron/actions/runs/45181702594) at `2b56cc1` concluded `success`.
All 14 jobs passed: the four Chromium shards and every WebKit shard (informational).

**Deployment.** Pages run
[45209601850](https://github.com/phynics/cast-iron/actions/runs/45209601850) succeeded for
`headBranch: release` at `2b56cc1` (event `push`, 2026-11-23, 4 min 7 s; build and deploy jobs both `success`). It
was the newest Pages run for `release` and the only one for this push. `gh api repos/phynics/cast-iron/pages`
reports `build_type: workflow` and `https_enforced: true`, and the `github-pages` environment allows the one branch
`release`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-23, after the deployment, with a throwaway
Playwright script (Chromium 156 and WebKit 27.2, a fresh profile for each run; the script is not committed). The
live `content/manifest.json` reads `schema_version: 6`, `content_version: v3f3d0b708758820d`,
`entry_count: 31742`, `total_bytes: 217014990`, `corpus_tag: v0.1.40`, 280 files. The live `sw.js` stamps
`SHELL_VERSION = "114f05ccdf222d2a"` and `SHELL_DATE = "2026-11-23"`.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron"); a fresh profile offers **Download** | pass | pass |
| `content/manifest.json`: content `v3f3d0b708758820d`, schema 6, 31,742 entries, corpus tag `v0.1.40`, 217,014,990 bytes in 280 files | pass | pass |
| `sw.js` stamp `114f05ccdf222d2a`, 2026-11-23; shipped `*_bg.wasm` 2,490,876 bytes | pass | pass |
| Every script and link URL of the document is under `/cast-iron/` (7 assets); no same-origin request outside that path | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens; the active library record reads `v3f3d0b708758820d`, schema 6 | pass, 11 s (27 s in the first run) | pass, 18 s (19 s in the first run) |
| The service worker controls the page after the install | pass | pass |
| Goblin (`ref/monster/goblin`): heading, statistics, no raw markup | pass | pass |
| Acolyte (`ref/npc/acolyte`): a spell block with `Caster level` and `Concentration` rows, no raw markup | pass | pass |
| Longsword (`ref/eq-weapon/longsword`): the Weapon Group row links to `#/entry/ref/ability/weapon-groups?a=sec-heavy-blades`; following it opens Weapon Groups at the Heavy Blades section | pass | pass |
| Bag of Holding (`ref/magic-wondrous/bag-of-holding`): a Construction block, no raw markup | pass | pass |
| Reactionary (`ref/trait/reactionary`): `Sources: Ultimate Campaign p. 53; Advanced Player's Guide p. 328; Second Darkness Player's Guide p. 11` | pass | pass |
| Power Attack (`ref/feat/power-attack`): two Sources lines, `Sources: Core Rulebook p. 131` and `Sources: Mythic Adventures p. 71` | pass | pass |
| Fireball (`ref/spell/fireball`): opens, no raw markup | pass | pass |
| Offline: reload on Goblin with the network off shows the entry | pass | pass |
| Offline: Bag of Holding opens from the cache with its Construction block | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell (50 hits) | pass | pass |
| Page errors and console errors during the final run | none | none |
| Responses with a status of 400 or more | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper and the
previous release check do. The install used the Download button of a profile without a library. The checks read
the rendered page text, link targets and attributes; they did not compare layout or styling.

- **Early runs: mistakes in my script, none in the page.** The first Chromium run failed three checks and the
  second runs of both browsers failed one. The script looked for a link named "Heavy Blades"; the Weapon Group row
  prints the group as "Blades, Heavy" and links it to the Heavy Blades section, as the corpus intends. It also
  read the second offline entry and the offline search results before the page had drawn them (the search results
  were counted before the region reported its result). The checks now use the printed label and wait for the
  rendered state. The final runs, one per browser, passed 35 of 35 checks; none of the runs logged a console error,
  a page error, a response with a status of 400 or more or a third-party request.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)) and no Home Screen app.
  The release was checked with Chromium and WebKit in a browser only.
- The path in which the old shell had already run a library check before the app update was not exercised (see
  Local). The upgrade check ran on a local build before the promotion, not on the live site, which no longer
  serves the old shell.
- The live check read the pages named above. It did not open the other pages whose statistics moved into stat
  blocks (2,828 monsters, 440 NPCs and 7,550 items), compare class-option powers with the books, or check styling
  at a phone width, in dark mode or in forced-colors mode.
- The live check did not drive the Settings page ([#632](https://github.com/phynics/cast-iron/pull/632)), the Find a
  book flow ([#629](https://github.com/phynics/cast-iron/pull/629)) or the daily update check.
- `cargo xtask published-ids` was not run for the release itself. It ran at the v0.1.38 pin (25 ids that the
  library already held), and the published-id guard of `cargo xtask content` passed at each pin. The 83 duplicate
  entries that leave the library in v0.1.39 resolve through `aliases.toml`.

**Known open issues.** [#423](https://github.com/phynics/cast-iron/issues/423): the iPhone measurement. The pin
entries list the remaining corpus warnings (9 in v0.1.40).

## Release notes

Changes since the release at `9a6fb06` (corpus v0.1.37, 2026-11-21). The release pins corpus v0.1.40.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 217 MB (207 MiB), 7.6 MB
smaller than before. The app shell changed (`114f05ccdf222d2a`), so the app offers a reload. Install the library
from **Settings**; the app does not download it by itself and keeps the old library in use until the new one has
passed its hash checks. Stars, active books and Continue Reading survive the app update and the library update
(checked before the release in Chromium and WebKit; see Local).

### Changes to the app

- **A smaller app.** The WASM is 2,490,876 bytes, down from 2,783,498: one non-generic stable sort (#596), BTree
  values built by insert and effects through one helper (#617), tagged enums read tag first with no schema
  change (#622), and views that never update in place are replaced instead of patched (#624).
- **Entry pages.** The head names the type and the parent of feats, traits, racial traits and archetypes (#609).
  Link lists fold and the relationship sections cap their length (#606). List rows name their parent, and the
  fold joins only printings of one rule (#611).
- **Search and lists.** Facets for the class-option group and the racial-trait race (#605), a Type filter menu
  for afflictions (#626), and large facet groups as multi-select filter menus (#631). The spell list keeps its name
  filter in the route and its density as one text toggle (#600); the name-filter race is fixed (#620). Races are a
  grouped list and the size facet's case is normalised (#598).
- **Class pages.** An archetype list with a compatibility filter (#599).
- **Books.** Find a book on Books and Manage books (#629), plus fixes to the book page, the reading footer and the
  chapter page (#602).
- **Settings and recovery.** Settings has seven fixed sections, and the update panel says each thing once (#632).
  The storage failure screen follows the recovery pattern (#630), and the recovery copy is revised (#601).
- **Masthead and legal page.** The Offline mark and the header action are not pills (#612). The legal page has a
  contents list, closed licences and folded maintainer tables (#614).

### Changes to the library

- **Citation rows** (corpus v0.1.40). Traits, racial traits, class options, afflictions, mythic feats, traps and
  skills carry `[[sources]]` rows with pages, so an entry page prints, for example, `Sources: Mythic Adventures p. 71`
  on Power Attack, Mythic.
- **Stat blocks** (corpus v0.1.40). Monsters, NPCs and items state their statistics in `::: statblock` blocks under
  `Defense`, `Offense`, `Statistics`, `Ecology` and `Construction` headings.
- **Weapon group anchors** (corpus v0.1.40). The `Weapon Group` row of a weapon links to its section of Weapon
  Groups.
- **Closed vocabularies** (corpus v0.1.38). Feat types, creature size, affliction types and item slots are closed
  lists, and the compiler checks them as errors (9 `slot` values stay open by owner decision). The Draconic
  bloodline's Linnorm table is rebuilt from the linnorm breath weapons.
- **Title echoes** (corpus v0.1.38) of creatures, traps, templates, feats, mythic feats and traits no longer repeat
  in the body.
- **Duplicates** (corpus v0.1.39). 83 duplicate entries leave the library and resolve through `aliases.toml`. The
  library keeps 31,742 entries, 443 publications and 0 broken links.

### For contributors

- The Chromium e2e job runs in four shards with weights 3:3:1:2 (#635), and specs that do not test installation use
  the installed fixture (#634). The feat prerequisite drift check compares the first printed statement (#621), and
  the stat-block drift check and the footnotes spec read statistics from stat-block rows (#628).
- **Docs.** `DESIGN.md` is restructured by layer (#597). New decisions are in `docs/decisions/` for the closed
  vocabularies, list rows that name their parent, the tag-first reader, replaced views, find-a-book route state,
  the Settings sections and the four-shard run. The previous release record is on `main` (#569).

### Known limits

- 19 class-option powers are exceptions that the check does not hold to the canonical form.
- 299 of 5,943 trades are unresolved. 1,501 book memberships in 384 entries have no `[[sources]]` row (corpus
  warnings of v0.1.40).
- The Gare and Sea rows of the Draconic bloodline's Linnorm table could not be checked against print.
