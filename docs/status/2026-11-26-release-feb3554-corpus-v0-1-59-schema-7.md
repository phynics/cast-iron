---
title: "Release `feb3554` (corpus v0.1.59, schema 7)"
date: 2026-11-26
kind: release
---

# Release `feb3554` (corpus v0.1.59, schema 7), 2026-11-26

`release` was promoted from `edcb421` to `feb35542147da2fe3b65785663b8a70fff4ff3c8` with
`scripts/promote-release.py --push`, a fast-forward push. **The library format does not change**: schema 7, search
index format 2. The range is two first-parent merges: #734 (the record of the previous release) and #735 (the
corpus pin v0.1.59, [pin entry](2026-11-26-pin-corpus-v0-1-59.md)). **The app shell did not change**: the shell
stamp is the one of the previous release, so only the library is new and the content version changes.

| Field | Value |
|---|---|
| Release commit | `feb35542147da2fe3b65785663b8a70fff4ff3c8` |
| Previous release | `edcb42111282a22017d2c8a533978566270568f1`, content `ve89d8b4c31071512` (schema 7, corpus v0.1.58) ([entry](2026-11-26-release-edcb421-corpus-v0-1-58-schema-7.md)) |
| Content | `vaaeb1e4c7340254b`, schema 7, index format 2, 31,766 entries, 445 publications |
| Library size | 215,286,180 bytes in 276 files, from the live manifest (about 215 MB, 205 MiB; the previous release was 215,458,549 bytes in 277 files, so 172,369 bytes smaller and one file fewer) |
| Shell | `a5d3b8e5a478002d`, released 2026-11-26 (unchanged: the shell is byte-identical to the previous release) |
| WASM | `cast-iron-web-ece7ee2eead02ff1_bg.wasm`, 2,585,921 bytes (the live `content-length` with `Accept-Encoding: identity`, and the length of the decoded body a browser fetched; Pages sends it gzip-encoded at 1,073,028 bytes on the wire). It is the same file name and length as the previous release |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.59`, commit `1aa2f7c0eadcbbc20cb555f2576a62708ecf94c1` |

**Local.** The schema upgrade check of [releasing.md](../releasing.md) ran before the promotion, by a delegate, on
local builds of `edcb421` and `feb355421` served on one origin, from an existing profile, in Chromium 156.0.8078.4
and WebKit 27.2. Schema 7 does not change, so this is an upgrade of an existing profile within one schema, with an
interrupted library download and a retry.

- *The upgrade path and the interrupted download: 85 of 85 checks in each browser.* Chromium's first run passed 84
  of 85; the one failure was a mistake in the script (an undefined expected content value), and the second run
  passed 85 of 85. WebKit passed in one run.
- *Errors.* 0 page errors, 0 requests to another host, 0 responses with a status of 400 or more.
- *The shell.* The local shell was byte-identical on both sides, so the app offered no update, only the library
  update.
- *The interrupted download.* The cut came after about 100 of 276 requests. The app showed an error with a Retry
  button, and the old library stayed active. The retry fetched only the missing files (176 of 177) and took 51.5 s
  in Chromium and 57.9 s in WebKit. While it ran, Goblin and Fireball still read from the old library.
- *Saved state survives.* The stars, including the haunt Arcane Storm and Combat Scabbard, the 28 chosen books and
  Continue Reading were all intact after the update.
- *No id changed.* A catalog-by-id comparison of the two libraries found no id added, removed or moved.
- *A converted archetype.* `ref/arc-monk/invested-regent` shows 2 `.feature-head` elements and its printed
  vested-powers list as 50 `li` items, with no raw markdown. Before the update it had 0 feature heads.
- *Publication screen* on the new `dist/`: exit 0, 0 issues, 445 sources, 31,766 entries, 1,704 entries with
  Community Use names.

The pin entry records the local checks of its PR. The record you are reading is a docs-only change and needs
`cargo xtask check` alone.

**CI.** Main full CI run [45821132156](https://github.com/phynics/cast-iron/actions/runs/45821132156) at
`feb355421` (started with `workflow_dispatch` on `main`, 2026-11-26 12:35Z) concluded `success`. All 14 jobs passed:
Rust checks, Content compiler, Content determinism (NFR-05), Build the site, the four Chromium shards, every WebKit
shard (informational) and the two report jobs.

**Deployment.** Pages run [45829830477](https://github.com/phynics/cast-iron/actions/runs/45829830477) succeeded
for `headBranch: release` at `feb355421` (event `push`, 2026-11-26, created 13:30:39Z; the build job ran 13:30:42 to
13:33:35Z and the deploy job 13:33:41 to 13:33:53Z; both `success`). It was the newest Pages run for `release` and
the only one for this push. `gh api repos/phynics/cast-iron/pages` reports `build_type: workflow` and
`https_enforced: true`. `origin/release` is at `feb35542147da2fe3b65785663b8a70fff4ff3c8`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-26, after the deployment, with a throwaway
Playwright script (Chromium 156.0.8078.4 and WebKit 27.2, a fresh profile for each run; the script is not
committed). The live `content/manifest.json` reads `schema_version: 7`, `content_version: vaaeb1e4c7340254b`,
`entry_count: 31766`, `total_bytes: 215286180`, `corpus_tag: v0.1.59`, 276 files. The live `sw.js` stamps
`SHELL_VERSION = "a5d3b8e5a478002d"` and `SHELL_DATE = "2026-11-26"`.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron"); a fresh profile offers **Download** | pass | pass |
| `content/manifest.json`: content `vaaeb1e4c7340254b`, schema 7, 31,766 entries, corpus tag `v0.1.59`, 215,286,180 bytes in 276 files | pass | pass |
| `sw.js` stamp `a5d3b8e5a478002d`, 2026-11-26; shipped `*_bg.wasm` 2,585,921 bytes, unchanged | pass | pass |
| Every script and link URL of the document is under `/cast-iron/` (7 assets); no same-origin request outside that path | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens; the active library record reads `vaaeb1e4c7340254b`, schema 7 | pass, 17 s | pass, 25 s |
| The service worker controls the page after the install | pass | pass |
| Goblin (`ref/monster/goblin`): heading, a Defense section, no raw markup | pass | pass |
| Acolyte (`ref/npc/acolyte`): a spell block with `Caster level` and `Concentration` rows, no raw markup | pass | pass |
| Longsword (`ref/eq-weapon/longsword`): the Weapon Group row links to `ref/ability/weapon-groups?a=sec-heavy-blades`; following it opens Weapon Groups | pass | pass |
| Bag of Holding (`ref/magic-wondrous/bag-of-holding`): a Construction block, no raw markup | pass | pass |
| Reactionary (`ref/trait/reactionary`): `Sources:` lines with Ultimate Campaign p. 53 and Advanced Player's Guide p. 328 | pass | pass |
| Power Attack (`ref/feat/power-attack`): `Sources: Core Rulebook p. 131` | pass | pass |
| Power Attack, Mythic (`ref/carried/mythic-adventures/feats/power-attack/mythic`): `Sources: Mythic Adventures p. 71` | pass | pass |
| Fireball (`ref/spell/fireball`), Fighter (`ref/class/fighter`) and the Core Rulebook book page open, no raw markup | pass | pass |
| `#/entry/ref/eq-weapon/combat-scabbard` resolves to `ref/eq-misc/combat-scabbard` (address and title "Combat scabbard") | pass | pass |
| The Magical Beast creature filter (`creature_type:magical%20beast`) lists creatures ("274 of 3,662 creatures", 18 rows rendered) | pass | pass |
| Two guild-guide traits open without raw markup: `ref/trait/a-sure-thing-guild-guide-8-0` and `ref/trait/a-sure-thing-organized-play-4-3` | pass | pass |
| The Books page lists both guild guides, Guide to Pathfinder Society Organized Play, Version 4.3 and Pathfinder Society Roleplaying Guild Guide, Version 8.0 (445 book links; "443 of 445 books active · Paizo Everything") | pass | pass |
| **New:** Invested Regent (`ref/arc-monk/invested-regent`, Heroes of the High Court, converted to printed prose in v0.1.59) shows 2 `.feature-head` elements (Investiture, Vested Power) and its lists as 50 `li` items; no raw markup. The entry opens and shows its footer with the Open Game License line, the Section 15 copyright line and the Legal and attribution link | pass | pass |
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
URL of the document, and the same length as the `content-length` of an identity-encoded request.

- **Three Chromium runs, honestly.** The first Chromium run passed 29 of 30 checks. The one failure was the Books
  page check: **a mistake in my script**, which looked for the guide titles in groups that the page builds closed
  (the same mistake as at the previous release). The same run's Invested Regent check passed with a loose condition
  (it counted `li` elements of the whole page, 118, and looked for the Community Use wording, which an Open Game
  License entry does not carry); a probe showed 50 `li` items in the entry body and the Open Game License footer, and
  I tightened both. I then ran the script again by mistake before the fix was in place (a failed edit command
  re-ran the old script), and it gave the same 29 of 30. After the fix the third Chromium run passed 30 of 30.
  WebKit ran once, on the fixed script, and passed 30 of 30. No run logged a console error, a page error, a response
  with a status of 400 or more or a request to another host, and no transient 503 from Pages occurred. The count of
  30 includes the four rows for errors, bad responses and foreign hosts.

**Not run.**

- No check on the real iPhone and no Home Screen app. The release was checked with Chromium and WebKit in a browser
  only. [#12](https://github.com/phynics/cast-iron/issues/12) was closed by the owner on 2026-11-25 without a
  real-device check.
- The upgrade check did not run the deployed old shell (the shell is byte-identical in this range, so no app
  update was offered and none was tested), and it did not exercise the daily automatic update check: only Settings,
  Reload. It ran on local builds before the promotion, not on the live site. No iOS Simulator session and no real
  device.
- The live check read the pages named above. It did not open the other pages that this pin changed (the other 156
  archetypes of the ten books and the five earlier entries that gained a level), or check styling at a phone width,
  in dark mode or in forced-colors mode, or with large text.
- No daily update check, Settings page or Find a book run on the live site.
- `cargo xtask published-ids` was not run for the release itself (the command needs a built library, and this
  record is docs only). The published-id guard of `cargo xtask content` passed at the pin, which adds, deletes and
  moves no entry.

**Known open issues.**

- [#540](https://github.com/phynics/cast-iron/issues/540): archetype, class-option and base-class features as
  printed prose. The release now also carries the archetypes of Blood of the Beast, Heroes of the High Court,
  Martial Arts Handbook, Inner Sea Magic, Inner Sea Intrigue, Inner Sea Combat, Wilderness Origins, Legacy of the
  First World, People of the Wastes and Healer's Handbook. Later batches are in the corpus or in progress and are
  not published.
- [#725](https://github.com/phynics/cast-iron/issues/725) (`post-v1`): Continue Reading. A tab on the old library
  can overwrite a valid point after another tab activates a replacement.
- [#726](https://github.com/phynics/cast-iron/issues/726) (`post-v1`): the glue checks of the corpus residue
  guards accept unrelated text.

## Release notes

Changes since the release at `edcb421` (corpus v0.1.58, 2026-11-26). The release pins corpus v0.1.59.

### Upgrading

**No schema change and no app update.** The shell is unchanged, so the app does not offer a reload. Install the
library update from Settings; the app does not download it by itself. The old library stays in use, and fully
readable, until the new one has passed its hash checks, and an interrupted download leaves the old library active;
Retry fetches only the files that are missing. The new library is about 215 MB (205 MiB), 172 KB smaller than
before. **Stars, active books and Continue Reading survive** (checked before the release in Chromium and WebKit;
see Local). No entry id is added, removed or moved.

### Changes to the app

None.

### Changes to the library

Entries: **31,766, as before** (203 entry files changed, none added, deleted or moved). Publications: 445. Four
data PRs, all part of [#540](https://github.com/phynics/cast-iron/issues/540), under owner rulings 45 to 50 on that
issue ([pin entry](2026-11-26-pin-corpus-v0-1-59.md)).

- **Archetypes as printed prose** (phynics/cast-iron-data#338, #339 and #340). 157 archetypes of ten books: Blood of the Beast, Heroes
  of the High Court and Martial Arts Handbook (41), Inner Sea Magic, Inner Sea Intrigue and Inner Sea Combat (53),
  and Wilderness Origins, Legacy of the First World, People of the Wastes and Healer's Handbook (63). They show
  feature heads with levels and search aliases, and their trades follow print. Some converted entries now hold
  printed bullet lists and tables (ruling 45). The Infiltrator (Advanced Class Guide) regains its flattened Voice
  Mimicry table.
- **Blood of the Ancients** (phynics/cast-iron-data#337) under ruling 26; two entries keep a citation.
- **Earlier entries** (phynics/cast-iron-data#339). One feature each of Halcyon Druid, Road-Keeper, Storm Druid,
  Rageshaper and Oozemorph now has `level = 1` (five features in all).

### For contributors

- The release adds no app, schema or spec change. The record of the previous release is on `main` (#734).

### Known limits

- No real-device check.
- The #540 conversion of the remaining class options, base-class features and archetypes is not complete; entries
  not yet converted keep their earlier form.
- 19 class-option powers are exceptions that the check does not hold to the canonical form
  (`power-exceptions.toml`).
- Saved links with the old `creature_type:magicalBeast` spelling list nothing (since the release at `edcb421`).
- The upgrade check did not run the deployed old shell or the daily automatic update check.
