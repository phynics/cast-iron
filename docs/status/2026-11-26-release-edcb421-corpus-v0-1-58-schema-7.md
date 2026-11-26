---
title: "Release `edcb421` (corpus v0.1.58, schema 7)"
date: 2026-11-26
kind: release
---

# Release `edcb421` (corpus v0.1.58, schema 7), 2026-11-26

`release` was promoted from `70d528f` to `edcb42111282a22017d2c8a533978566270568f1` with
`scripts/promote-release.py --push`, a fast-forward push. **The library format does not change**: schema 7, search
index format 2. The range is 14 first-parent merges, #719 to #733. It carries five corpus pins, v0.1.53, v0.1.54,
v0.1.56, v0.1.57 and v0.1.58 ([v0.1.53](2026-11-25-pin-corpus-v0-1-53.md),
[v0.1.54](2026-11-25-pin-corpus-v0-1-54.md), [v0.1.56](2026-11-25-pin-corpus-v0-1-56.md),
[v0.1.57](2026-11-26-pin-corpus-v0-1-57.md) and [v0.1.58](2026-11-26-pin-corpus-v0-1-58.md); v0.1.52 and v0.1.55
are inside the pins that skip them), and a few app changes. The app shell and the library both changed, so the
shell stamp and the content version are new.

| Field | Value |
|---|---|
| Release commit | `edcb42111282a22017d2c8a533978566270568f1` |
| Previous release | `70d528fc78fb1e73cde3b63ba0b8de78202c4587`, content `vad43d1e1b6e40e61` (schema 7, corpus v0.1.51) ([entry](2026-11-25-release-70d528f-corpus-v0-1-51-schema-7.md)) |
| Content | `ve89d8b4c31071512`, schema 7, index format 2, 31,766 entries, 445 publications |
| Library size | 215,458,549 bytes in 277 files, from the live manifest (about 215 MB, 205 MiB; the previous release was 215,677,047 bytes in 277 files, so 218,498 bytes smaller and the same number of files) |
| Shell | `a5d3b8e5a478002d`, released 2026-11-26 (the previous release stamped `2d4770843b89f710`) |
| WASM | `cast-iron-web-ece7ee2eead02ff1_bg.wasm`, 2,585,921 bytes (the live `content-length` with `Accept-Encoding: identity`, and the length of the decoded body a browser fetched; Pages sends it gzip-encoded at 1,073,028 bytes on the wire). The previous release shipped a file of the same length, 2,585,921 bytes, under a different name (`c3ea433637d8faf0`); the only code change in `crates/web`, `crates/schema`, `crates/search` and `web/` in the range is in `views/search.rs` (#724) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.58`, commit `10f42689700375f4e5da3a69821cc9be5aa462e0` |

**Local.** The schema upgrade check of [releasing.md](../releasing.md) ran before the promotion, by a delegate, on
local builds of `70d528f` and `edcb42111` served on one origin, in Chromium 156.0.8078.4 and WebKit 27.2. Schema 7
does not change, so this is an upgrade of an existing profile within one schema, with an interrupted library
download and a retry.

- *The upgrade path and the interrupted download: 84 of 84 checks in each browser.* Chromium's first run passed 83
  of 84; the one failure was a case-sensitive title regex in the script, and the second run passed 84 of 84.
  WebKit passed in one run.
- *Errors.* 0 page errors, 0 requests to another host, 0 responses with a status of 400 or more. The 9 aborted
  requests in each browser were the deliberate cut.
- *The interrupted download.* The cut came at 101 of 277 requests. The retry fetched 177 files, and the retry with
  the activation took 50 s in Chromium and 55 s in WebKit.
- *Haunt star.* `ref/haunt/arcane-storm`, starred before the update, stays a `haunt` throughout.
- *Combat Scabbard.* A star on `ref/eq-weapon/combat-scabbard`, a real entry in v0.1.51, is rewritten to
  `ref/eq-misc/combat-scabbard` with the same `added_at`, title and kind when the new library is activated. It
  shows as one Starred row that links to the canonical entry. The old address opens the canonical entry.
- *Creature type.* The Magical Beast facet (`creature_type:magical%20beast`) lists 183 of 2,575 creatures under the
  28-book profile of the check. An old route `creature_type:magicalBeast` lists 0, which is the transition behaviour
  [#724](https://github.com/phynics/cast-iron/pull/724) accepted: a saved or shared link with the old spelling
  needs the type picked again from the menu.
- *Books.* 445 books are listed (443 before). Both new guild guides are active: the stored choice is a set of
  disabled ids, so a book that did not exist before is active. The 28 chosen books are unchanged.
- *Publication screen* on the new `dist/`: exit 0, 0 issues, 445 sources, 31,766 entries, 1,704 entries with
  Community Use names.
- *Shell stamps.* The local build of `70d528f` stamped `bb3dd0ae36b7335d`, not the deployed `2d4770843b89f710`. As
  at the last two releases, a local build and the deployed build differ, so the check did not start from the
  deployed shell.

The pin entries record the local checks of their pin PRs, and the app PRs record their own. The record you are
reading is a docs-only change and needs `cargo xtask check` alone.

**CI.** Main full CI run [45759332700](https://github.com/phynics/cast-iron/actions/runs/45759332700) at
`edcb42111` (started with `workflow_dispatch` on `main`) concluded `success`. All 14 jobs passed: Rust checks,
Content compiler, Content determinism (NFR-05), Build the site, the four Chromium shards, every WebKit shard
(informational) and the two report jobs.

**Deployment.** Pages run [45773332985](https://github.com/phynics/cast-iron/actions/runs/45773332985) succeeded
for `headBranch: release` at `edcb42111` (event `push`, 2026-11-26, 3 min 59 s from its creation at 07:10:17Z; the
build job ran 07:10:20 to 07:13:57Z and the deploy job 07:14:03 to 07:14:15Z; both `success`). It was the newest
Pages run for `release` and the only one for this push. `gh api repos/phynics/cast-iron/pages` reports
`build_type: workflow` and `https_enforced: true`, and the `github-pages` environment allows the one branch
`release`. `origin/release` is at `edcb42111282a22017d2c8a533978566270568f1`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-26, after the deployment, with a throwaway
Playwright script (Chromium 156.0.8078.4 and WebKit 27.2, a fresh profile for each run; the script is not
committed). The live `content/manifest.json` reads `schema_version: 7`, `content_version: ve89d8b4c31071512`,
`entry_count: 31766`, `total_bytes: 215458549`, `corpus_tag: v0.1.58`, 277 files. The live `sw.js` stamps
`SHELL_VERSION = "a5d3b8e5a478002d"` and `SHELL_DATE = "2026-11-26"`.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron"); a fresh profile offers **Download** | pass | pass |
| `content/manifest.json`: content `ve89d8b4c31071512`, schema 7, 31,766 entries, corpus tag `v0.1.58`, 215,458,549 bytes in 277 files | pass | pass |
| `sw.js` stamp `a5d3b8e5a478002d`, 2026-11-26; shipped `*_bg.wasm` 2,585,921 bytes | pass | pass |
| Every script and link URL of the document is under `/cast-iron/` (7 assets); no same-origin request outside that path | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens; the active library record reads `ve89d8b4c31071512`, schema 7 | pass, 25 s | pass, 17 s |
| The service worker controls the page after the install | pass | pass |
| Goblin (`ref/monster/goblin`): heading, a Defense section, no raw markup | pass | pass |
| Acolyte (`ref/npc/acolyte`): a spell block with `Caster level` and `Concentration` rows, no raw markup | pass | pass |
| Longsword (`ref/eq-weapon/longsword`): the Weapon Group row links to `ref/ability/weapon-groups?a=sec-heavy-blades`; following it opens Weapon Groups | pass | pass |
| Bag of Holding (`ref/magic-wondrous/bag-of-holding`): a Construction block, no raw markup | pass | pass |
| Reactionary (`ref/trait/reactionary`): `Sources:` lines with Ultimate Campaign p. 53 and Advanced Player's Guide p. 328 | pass | pass |
| Power Attack (`ref/feat/power-attack`): `Sources: Core Rulebook p. 131` | pass | pass |
| Power Attack, Mythic (`ref/carried/mythic-adventures/feats/power-attack/mythic`): `Sources: Mythic Adventures p. 71` | pass | pass |
| Fireball (`ref/spell/fireball`), Fighter (`ref/class/fighter`) and the Core Rulebook book page open, no raw markup | pass | pass |
| **New:** `#/entry/ref/eq-weapon/combat-scabbard` resolves to `ref/eq-misc/combat-scabbard` (address and title "Combat scabbard") | pass | pass |
| **New:** the Magical Beast creature filter (`creature_type:magical%20beast`) lists creatures ("274 of 3,662 creatures", 18 rows rendered) | pass | pass |
| **New:** two guild-guide traits open without raw markup: `ref/trait/a-sure-thing-guild-guide-8-0` and `ref/trait/a-sure-thing-organized-play-4-3` | pass | pass |
| **New:** the Books page lists both guild guides, Guide to Pathfinder Society Organized Play, Version 4.3 and Pathfinder Society Roleplaying Guild Guide, Version 8.0 (445 book links; "443 of 445 books active · Paizo Everything") | pass | pass |
| **New:** Aerochemist (`ref/arc-alchemist/aerochemist`, converted to printed prose in v0.1.58) shows `.feature-head` elements (3) | pass | pass |
| Haunts: `#/browse/kind/haunt` heading "Haunts", "45 haunts", 45 rows | pass | pass |
| Haunts: `ref/haunt/arcane-storm` opens, no raw markup, the metadata line reads `Haunt · Horror Adventures` | pass | pass |
| Haunts: Home's "More categories" links Haunts (count 45) to `#/browse/kind/haunt` | pass | pass |
| Offline: reload on Goblin with the network off shows the entry | pass | pass |
| Offline: Bag of Holding opens from the cache with its Construction block | pass | pass |
| Offline: the haunt list shows 45 rows | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell | pass | pass |
| Page errors and console errors during the final run | none | none |
| Responses with a status of 400 or more | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, as the test helper and the previous release check
do. The install used the Download button of a profile without a library. That profile shows "443 of 445 books
active" under the Paizo Everything preset, so the creature count above is for those books and differs from the
count of the local check. The checks read the rendered page text, link targets and attributes; they did not compare
layout or styling. The WASM size is the decoded body length of the response the browser received for the `*_bg.wasm`
URL of the document.

- **Two runs in Chromium, honestly.** The first Chromium run passed 31 of 32 checks. The one failure was the
  Books page check: **a mistake in my script**, which looked for the guide titles in groups that the page builds
  closed. The page was not at fault. After the script opened every group, the second run passed 32 of 32. WebKit
  ran once and passed 32 of 32. Neither run logged a console error, a page error, a response with a status of 400 or
  more or a request to another host, and no transient 503 from Pages occurred.

**Not run.**

- No check on the real iPhone and no Home Screen app. The release was checked with Chromium and WebKit in a browser
  only. [#12](https://github.com/phynics/cast-iron/issues/12) was closed by the owner on 2026-11-25 without a
  real-device check.
- The upgrade check did not start from the deployed old shell (its stamp differs from the local build's, see
  Local), and it did not exercise the daily automatic update check: only Settings, Reload. It ran on local builds
  before the promotion, not on the live site, which no longer serves the old shell. No iOS Simulator session and no
  real device.
- The live check read the pages named above. It did not open the other pages that these pins changed (the 348
  archetypes of the #540 batches 3 to 5 and the pilot beyond Aerochemist, the 91 guild-guide traits beyond
  one, the 428 entries whose creature type spelling changed beyond one filter), or check styling at a phone width, in
  dark mode or in forced-colors mode, or with large text.
- No daily update check, Settings page or Find a book run on the live site.
- `cargo xtask published-ids` was not run for the release itself (the command needs a built library, and this
  record is docs only). The published-id guard of `cargo xtask content` passed at each pin, and the pin entries
  record that the 95 new ids are in `sources/published-ids.txt` and that the one removed entry resolves through
  `aliases.json`.

**Known open issues.**

- [#540](https://github.com/phynics/cast-iron/issues/540): archetype, class-option and base-class features as
  printed prose. The release now carries the archetypes of the Advanced Player's Guide, Ultimate Magic, Ultimate
  Combat, the Advanced Class Guide, the Advanced Race Guide, Occult Adventures, Ultimate Intrigue, Ultimate
  Wilderness, the Adventurer's Guide, Horror Adventures, Planar Adventures and Blood of the Ancients. Later batches
  are in the corpus or in progress and are not published.
- [#725](https://github.com/phynics/cast-iron/issues/725) (`post-v1`): Continue Reading. A tab on the old library
  can overwrite a valid point after another tab activates a replacement.
- [#726](https://github.com/phynics/cast-iron/issues/726) (`post-v1`): the glue checks of the corpus residue
  guards accept unrelated text.

## Release notes

Changes since the release at `70d528f` (corpus v0.1.51, 2026-11-25). The release pins corpus v0.1.58.

### Upgrading

**No schema change.** The app update comes first; the app offers a reload. The library update is a separate action
from Settings, and the app does not download it by itself. The old library stays in use, and fully readable, until
the new one has passed its hash checks, and an interrupted download leaves the old library active; Retry fetches
only the files that are missing. The new library is about 215 MB (205 MiB), 218 KB smaller than before. **Stars,
active books and Continue Reading survive** (checked before the release in Chromium and WebKit; see Local).

- **A star on the old weapon Combat Scabbard moves to the equipment entry.** Print has one Combat Scabbard, so the
  weapon entry became an alias of `ref/eq-misc/combat-scabbard`. The star keeps its date and title and shows as one
  Starred row. The old address still opens the entry.
- **Saved or shared links with the old creature type spelling list nothing.** The corpus now writes `magical beast`
  and `monstrous humanoid`; a link that holds `creature_type:magicalBeast` lists no creature. Pick the type again
  from the menu.
- **Two new books are active.** The book choice is stored as the books that are off, so the two new guild guides
  are on until you switch them off.

### Changes to the app

- **Share slots for the two Pathfinder Society guild guides** (#730). Each book of a library needs a stable slot in
  share links before the content compiler accepts it; the two guides take the slots 443 and 444.
- **Lower-case typed fields** (#724). The creature type and spell subschool filters accept the new lower-case
  spelling from the corpus beside the old one, and the menus keep their capitalised labels ("Magical Beast").

### Changes to the library

Entries: **31,672 to 31,766 (94 more)**. v0.1.56 removes one entry: the weapon `ref/eq-weapon/combat-scabbard` is
an alias of the equipment entry, so no id is lost (31,672 to 31,671). v0.1.57 adds 95 entries (31,671 to 31,766).
Publications go from 443 to 445. The pins v0.1.53, v0.1.54 and v0.1.58 add or delete no entry.

- **The Pathfinder Society guild guides** (v0.1.57). Two publications, Guide to Pathfinder Society Organized Play,
  Version 4.3 and Pathfinder Society Roleplaying Guild Guide, Version 8.0, and 95 entries: 91 traits (the faction
  traits of both guides and the five exemplar traits of Chronicle of Legends), three feats (Inscribe Rune,
  Osiriontologist, Overseer of the Two Treasuries) and one spell (Baphomet's Blessing). Both guides have their
  Section 15 notices in the corpus.
- **Archetypes as printed prose** ([#540](https://github.com/phynics/cast-iron/issues/540)). Batch 3 states every
  archetype of Occult Adventures (43) and Ultimate Intrigue (74), batch 4 every archetype of Ultimate Wilderness
  (126) and the Adventurer's Guide (38), batch 5 every archetype of Horror Adventures (37) and Planar Adventures
  (12), and the multi-model pilot (v0.1.58) the 18 archetypes of Blood of the Ancients. In v0.1.53, 18 archetypes that waited for the trade reader (12 oracle
  archetypes among them) and Daring Infiltrator and Ironskin Monk are converted, so every archetype of the Advanced
  Class Guide and the Advanced Race Guide is printed prose. These archetypes show feature heads with `Level N` and search aliases, and
  their trades follow print. Duplicate trades that differ only in case are merged and the witch hex trades take the
  printed words.
- **Spell fields and levels** (v0.1.56). Major Image is level 3 with the duration `concentration+ 3 rounds`, and
  Secret Vault has 60 days. Durations and components are fixed in seven spells and class levels in 13; 106 class
  levels are added and nine unprinted ones removed. A class level is held only when print states it or a printed
  rule derives it.
- **One spelling for typed fields** (v0.1.56, phynics/cast-iron-data#287). `creature_type` (428 entries), lower-case `size` (325),
  written-out alignments (309), `hit_die` without a trailing period (120), lower-case spell subschools (1,074
  values), `glamer` out of descriptors (82) and bracketed descriptors moved out of `school` (14).
- **Combat Scabbard** is one entry, `ref/eq-misc/combat-scabbard` (v0.1.56). The weapon id is an alias.
- **Polytheistic Blessing** gains the eleven pantheons of Faiths and Philosophies p. 21 (v0.1.58).
- **Print fixes.** 349 citation pages, ten Effect lines (nine diseases and Void Death, now 1d3 Dex and 1d3 Con),
  Brambleblight as two stat sets and the folded stat rows of nine Adventure Path creatures (v0.1.53); 19 titles
  follow print, for example Unshakable Zeal, Phantasmal Asphyxiation, Conciliator, Usurping Spell and Poisonous
  Performer, with ids that keep the old spelling (v0.1.54); 40 entries keep a printed page citation and six gain
  their printed table title (v0.1.58).

### For contributors

- **The printed-trade check is an error** (#728, #731). A printed trade sentence of a listed feature needs a stored
  trade; v0.1.57 is the first tag with none left, so the check turned from a warning into an error with the pin.
  The trade reader follows "as indicated below" as a cross-reference (#723) and expands each suspended-hyphen list
  against its own tail (#727). The data side holds a root file, `trade-sentence-exceptions.toml`, with one row.
- **An uncited book membership is a corpus error**, with an allow-list file (#715, [#461](https://github.com/phynics/cast-iron/issues/461)).
- **Specs.** The spec for the external-reference line waits before it measures the edges in WebKit (#721), and the Quick search spec reads
  `aria-controls` after the lists settle (#732). `feature-heads.spec.ts` picks its control archetype from the
  compiled library, because #540 converts the one it used (v0.1.56).
- **Docs.** The record of the previous release is on `main` (#720) and the pin entries cover the other changes.

### Known limits

- No real-device check.
- The #540 conversion of the remaining class options, base-class features and archetypes is not complete; entries
  not yet converted keep their earlier form.
- 19 class-option powers are exceptions that the check does not hold to the canonical form
  (`power-exceptions.toml`).
- Saved links with the old `creature_type:magicalBeast` spelling list nothing (see Upgrading).
- The upgrade check did not start from the deployed old shell, and it did not run the daily automatic update check.
