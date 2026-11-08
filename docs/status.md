# Status

Updated 2026-11-07.

## Release `1dacf59` (corpus v0.1.4), 2026-11-07

`release` was promoted from `20e6a31` to
`1dacf5921ad0f309c7cc6d61ecbd1abf4f96b7e0` with `scripts/promote-release.py --push`. The release
serves content `v2189dcc7f6c8b730` (schema 3, 31,106 entries) with shell `9a77958fff99810f` from
corpus pin `v0.1.4`.

| Field | Value |
|---|---|
| Release commit | `1dacf5921ad0f309c7cc6d61ecbd1abf4f96b7e0` |
| Previous release | `20e6a31`, content `v377b3e08a8a0692a`, shell `6d54314fc71e9ae9` |
| Content | `v2189dcc7f6c8b730`, schema 3, 31,106 entries |
| Shell | `9a77958fff99810f` |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.4`, commit `cd8f9b3` |

**Deployment.** Pages run
[42368029974](https://github.com/phynics/cast-iron/actions/runs/42368029974) succeeded (build and
deploy) for `headBranch: release` at `1dacf59`. The live `content/manifest.json` reads
`content_version: v2189dcc7f6c8b730`, `schema_version: 3`, `entry_count: 31106`, from corpus
`v0.1.4`. A local `cargo xtask content` reproduces the same version and byte total.

**CI.** Main full CI run
[42366236069](https://github.com/phynics/cast-iron/actions/runs/42366236069) succeeded at
`1dacf59`. The run succeeded overall; one informational WebKit shard failed.

**Local upgrade check, 2026-11-07.** From an installed profile on the previous release (library
`v377b3e08a8a0692a`, shell `6d54314fc71e9ae9`) to this one (library `v2189dcc7f6c8b730`, shell
`9a77958fff99810f`), in Chromium:

| Check | Result |
|---|---|
| Stars | Kept |
| Inactive book (existing install not reset to Paizo Rulebooks) | Kept: `438 of 439 books active · Custom` |
| Old recent searches | Kept, unscoped |
| Offline reload | Passed |
| Console errors / third-party requests | 0 / 0 |
| Update manifest checks | One, same-origin |
| Fresh profile default | Paizo Rulebooks |
| `update`, `shell-update`, `canonical-cutover`, `active-books` specs | 22 of 22 per browser, Chromium and WebKit |

**Live check, fresh browser context, headless Chromium** against
https://phynics.github.io/cast-iron/ on 2026-11-07:

| Check | Result |
|---|---|
| App shell loads; every asset path is under `/cast-iron/` | PASS (HTTP 200; 301 paths) |
| Download installs the library: `v2189dcc7f6c8b730`, schema 3, 31,106 entries | PASS (30.6 s) |
| Books shows the fresh default `28 of 439 books active · Paizo Rulebooks` | PASS |
| `#/browse/kind/creature` shows 13 creature types, including "Magical Beast" | PASS |
| `ref/feat/power-attack` renders "Str 13, base attack bonus +1" | PASS |
| Offline reload still renders the entry | PASS |
| Every request goes to `phynics.github.io` | PASS (one host) |
| Console errors, page errors, failed requests | PASS: 0, 0, 0 |

**Published ids.** `cargo xtask published-ids` added 0 ids, and
`sources/published-ids.txt` keeps its 41,074 ids. `git diff sources/published-ids.txt | grep
'^-[^-]'` prints nothing. Corpus v0.1.4 changed creature-type spellings but added no permanent id.

**Shipped since `20e6a31`** (31 first-parent merges, grouped by PR):

- Books and Manage books: [#204](https://github.com/phynics/cast-iron/pull/204) a book row opens
  the book page, [#215](https://github.com/phynics/cast-iron/pull/215) Manage books page with
  presets and Undo, [#214](https://github.com/phynics/cast-iron/pull/214) publication filter opens
  Adventures alone and keeps a forced tier's choice,
  [#234](https://github.com/phynics/cast-iron/pull/234) one count line, oldest first, h2 groups,
  book page without breadcrumb, [#228](https://github.com/phynics/cast-iron/pull/228) test fixes
  after Active books.
- Filter menus and lenses: [#213](https://github.com/phynics/cast-iron/pull/213) menus for items,
  traits, archetypes and spells, classes under group headings,
  [#210](https://github.com/phynics/cast-iron/pull/210) feats filter menu, in-place type filter,
  chain marker and scoped recents, [#216](https://github.com/phynics/cast-iron/pull/216) drop
  empty facet values, [#218](https://github.com/phynics/cast-iron/pull/218) keep publication
  names out of article contents.
- Search and quick search: [#202](https://github.com/phynics/cast-iron/pull/202) list-page
  template and scoped Quick search, [#221](https://github.com/phynics/cast-iron/pull/221) name the
  active source and say when matches are hidden,
  [#229](https://github.com/phynics/cast-iron/pull/229) quick search suggestions use the preferred
  active source, [#231](https://github.com/phynics/cast-iron/pull/231) tie-break hidden match
  books by tier, then source id.
- Entry navigation and rendering: [#222](https://github.com/phynics/cast-iron/pull/222) creature
  breadcrumbs on one line, [#227](https://github.com/phynics/cast-iron/pull/227) stop the
  backlinks loading flash, [#219](https://github.com/phynics/cast-iron/pull/219) pluralize counted
  labels through one helper.
- Updates and settings: [#209](https://github.com/phynics/cast-iron/pull/209) surface library
  updates with a same-origin, opt-out automatic check and Updates first in Settings,
  [#207](https://github.com/phynics/cast-iron/pull/207) header update action opens Settings →
  Updates.
- Corpus and CI: [#208](https://github.com/phynics/cast-iron/pull/208) pin corpus v0.1.3 and check
  CR lines for drift, [#220](https://github.com/phynics/cast-iron/pull/220) pin corpus v0.1.4 and
  require one of the 13 PF1e creature types, [#233](https://github.com/phynics/cast-iron/pull/233)
  fold creature-type spellings before comparing,
  [#235](https://github.com/phynics/cast-iron/pull/235) fail `corpus-check` on leftover upstream
  tokens, [#212](https://github.com/phynics/cast-iron/pull/212) bound every CI job with
  `timeout-minutes`, [#225](https://github.com/phynics/cast-iron/pull/225) derive class-group test
  expectations from the built data, [#226](https://github.com/phynics/cast-iron/pull/226) raise
  the keyboard-walk test ceiling.
- Focus, copy and docs: [#203](https://github.com/phynics/cast-iron/pull/203) record the
  `20e6a31` release, [#223](https://github.com/phynics/cast-iron/pull/223) record the carried
  spell-list counts, [#224](https://github.com/phynics/cast-iron/pull/224) record the filter-menu
  focus deviation, [#230](https://github.com/phynics/cast-iron/pull/230) record why the search row
  and entry header differ on preferred sources,
  [#232](https://github.com/phynics/cast-iron/pull/232) move focus to the results status line,
  [#217](https://github.com/phynics/cast-iron/pull/217) callout labels and dark-surface contrast.

**Not verified.** Physical devices and the accessibility check
([#12](https://github.com/phynics/cast-iron/issues/12)) are still not done. Firefox and WebKit
were not run against the live site. `release` points at `1dacf59`; `main` is ahead only by
documentation commits such as this record.

## Corpus pinned to v0.1.4, 2026-11-07

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.4`, commit
`cd8f9b3656bba069f3a1a4728649d1a585056442`, tree hash
`77ef95c353e25d342d55e5a7405554f58beabf2d558aa7b718a36ab48035827e`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). The pin normalises every creature's `creature_type` to one of the 13 PF1e
spellings, so the Creatures lens shows 13 chips with readable two-word labels, and carries
the markup, spell-link and raw-link fixes in corpus v0.1.4
([#211](https://github.com/phynics/cast-iron/issues/211)).

Local verification, 2026-11-07 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v2189dcc7f6c8b730`, 31,106 entries (was `v57b3e42733317ed3`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books, 1 licence; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `3f17489e52a0c2ac`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/browse-groups.spec.ts tests/search.spec.ts` | 49 passed |
| Creature lens on the built site | 13 chips, including “Magical Beast” and “Monstrous Humanoid”; no `magicalBeast`, `monstrousHumanoid`, or `Intelligence` |

## Corpus pinned to v0.1.3, 2026-11-06

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.3`, commit
`46e7cf538d3d2b8bc49850b112ba1acb446c3d18`, tree hash
`7d625e7b4a8ebe07b51046010f777109dacf75a8c03801a399e91cfe5e43f7fe`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). This pin supplies the facet data behind the class, item and creature lenses
([#195](https://github.com/phynics/cast-iron/issues/195)), and the CR drift check now compares
a creature's `cr` field with its `**Challenge rating:**` line
([#198](https://github.com/phynics/cast-iron/issues/198)).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v57b3e42733317ed3`, 31,106 entries (was `v377b3e08a8a0692a`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books, 1 licence; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `50ff6058afafd3d9`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/browse-groups.spec.ts tests/search.spec.ts` | 35 passed |

The lenses read from the search index in the built site:

| Lens | Values | Entries |
|---|---|---|
| Class, `class_type` (prestige skipped) | 7: Alternate 3, Base 10, Core 11, Hybrid 10, NPC 5, Occult 6, Unchained 4 | 49 |
| Item, `item_category` | 57, from Adventuring Gear 49 and Weapons 541 to Wondrous Items 2,614 and Miscellaneous Equipment 1,747 | 8,091 |
| Creature, `creature_type` | 26 chips for 13 types, because the corpus spells some types in two cases (`aberration` 182 and `Aberration` 2); `Intelligence` appears as a type | 3,241 |
| Creature, `cr` | 35, from `1/8` 14 through `1` 199 and `5` 256 to `30` 13 | 3,718 |

## Corpus pinned to v0.1.2, 2026-11-06

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.2`, commit
`7739e7add3f2a6ccd1c5232377bd910d38278992`, tree hash
`44f014cb44d2ab033d9069cbfd4c30bad28bfc3e99dae6cd4f3e1e63e16de29f`
(`cargo xtask corpus-tree-hash` on the extracted archive; `cargo xtask fetch-sources`
verified it). Drift between typed fields and body text is now an error rather than a
warning, so `corpus-check` and `cargo xtask content` fail on any disagreement
([#180](https://github.com/phynics/cast-iron/issues/180)).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `generated/content` | Content version `v377b3e08a8a0692a`, 31,106 entries (was `vc01cc1c20a652c37`, 31,106 entries) |
| `cargo xtask corpus-check .cache/sources/cast-iron-data` | Exit 0: 31,106 entries, 439 publications, 449 books; 0 drift disagreements |
| `cargo xtask check` | Passed |
| `cargo xtask build-site --public-url /cast-iron/` | Passed; shell `6d54314fc71e9ae9`, 11 files |
| `npm --prefix tests/e2e test -- --project=chromium tests/journeys.spec.ts tests/relationships.spec.ts` | 9 passed |

## Release `20e6a31` (corpus v0.1.2), 2026-11-06

`release` was promoted from `ccbfb7c` to
`20e6a31af342cb5f7899cdb7e039022f5bae5de8` with `scripts/promote-release.py --push`. The release
serves content `v377b3e08a8a0692a` (schema 3, 31,106 entries) with shell
`6d54314fc71e9ae9`. At that time `release` and `origin/main` both pointed at `20e6a31`, and
nothing had merged after it.

| Field | Value |
|---|---|
| Release commit | `20e6a31af342cb5f7899cdb7e039022f5bae5de8` |
| Previous release | `ccbfb7c`, content `vc01cc1c20a652c37`, shell `501943363e34b0fb` |
| Content | `v377b3e08a8a0692a`, schema 3, 31,106 entries |
| Shell | `6d54314fc71e9ae9` |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.2`, commit `7739e7a` |
| Included changes | [#185](https://github.com/phynics/cast-iron/pull/185) Home density, [#186](https://github.com/phynics/cast-iron/pull/186) two-step release record, [#190](https://github.com/phynics/cast-iron/pull/190) Books tab speed, [#181](https://github.com/phynics/cast-iron/pull/181) Quick search, [#200](https://github.com/phynics/cast-iron/pull/200) status drift fixes, [#187](https://github.com/phynics/cast-iron/pull/187) feat field drift check, [#201](https://github.com/phynics/cast-iron/pull/201) corpus v0.1.2 pin |

**Deployment.** Pages run
[42288829788](https://github.com/phynics/cast-iron/actions/runs/42288829788) succeeded (build and
deploy) for `headBranch: release` at `20e6a31`. The live `content/manifest.json` reads
`content_version: v377b3e08a8a0692a`, `schema_version: 3`, `entry_count: 31106`. A local
`cargo xtask content` reproduces the same version.

**CI.** Main full CI run
[42281880993](https://github.com/phynics/cast-iron/actions/runs/42281880993) succeeded at
`20e6a31`.

**Local upgrade check, 2026-11-06.** From an installed profile on the previous release (library
`vc01cc1c20a652c37`, shell `501943363e34b0fb`) to this one (library `v377b3e08a8a0692a`, shell
`6d54314fc71e9ae9`), in Chromium and WebKit:

| Check | Result |
|---|---|
| App update and library update | Both offered and applied |
| Stars | Survived |
| Active-book choice | Survived |
| Point-Blank Shot requirements | No longer names Precise Shot |
| Offline reload | Passed |
| Console errors / third-party requests | 0 / 0 |
| `update.spec`, `shell-update.spec`, `canonical-cutover.spec` | 6 of 6 passed |

**Live check, fresh profile, headless Chromium** against
https://phynics.github.io/cast-iron/ on 2026-11-06:

| Check | Result |
|---|---|
| App shell loads; every asset path is under `/cast-iron/` | PASS |
| Download installs the library: `v377b3e08a8a0692a`, schema 3, 31,106 entries | PASS (29.9 s) |
| `ref/feat/power-attack` renders "Str 13, base attack bonus +1" | PASS |
| `ref/feat/point-blank-shot` has no Requirements section or prerequisite chain naming Precise Shot | PASS |
| Offline reload still renders the entry | PASS |
| Every request goes to `phynics.github.io` | PASS |
| Console errors, page errors, failed requests | PASS: 0, 0, 0 |

One earlier attempt logged a single transient 503 from Pages for a static asset and read the
Power Attack body before it finished rendering; the rerun above was clean.

**Published ids.** `cargo xtask published-ids` added 0 ids; `sources/published-ids.txt` stays at
41,074 ids (9,968 PSRD ids and 31,106 `ref/` ids). The v0.1.2 corpus introduced no new permanent
ids.

**Not verified.** Physical devices and the accessibility check
([#12](https://github.com/phynics/cast-iron/issues/12)) are still not done. Firefox and WebKit
were not run against the live site.

## Two-step release, 2026-11-06

Pages now serves the canonical corpus library. Both steps followed `docs/releasing.md`, and the
owner approved them in [#127](https://github.com/phynics/cast-iron/issues/127).

| Step | `release` | App commit | Library | Pages run |
|---|---|---|---|---|
| Before | `d48eaa3` | `d48eaa3` | PSRD (old shell) | — |
| 1. Alias-capable shell, PSRD library | `ae032ba` (one-time fast-forward push) | tag `psrd-last-buildable` plus `--psrd` | `vcdd730994d863e50`, 9,990 entries | [42200932251](https://github.com/phynics/cast-iron/actions/runs/42200932251): build and deploy succeeded |
| 2. Canonical corpus library | `ccbfb7c` (`promote-release.py --push`) | `main` `ccbfb7c` (#177, #179, #184) | `vc01cc1c20a652c37` | [42206465391](https://github.com/phynics/cast-iron/actions/runs/42206465391): build and deploy succeeded |

**Superseded.** These changes shipped in the `20e6a31` release above and then in the `1dacf59`
release. `release` points at `1dacf59`.

**CI**
- Prep commit `ae032ba`: [42190040210](https://github.com/phynics/cast-iron/actions/runs/42190040210)
  succeeded on attempt 2, including the PSRD determinism steps. On attempt 1, WebKit shard 3/3
  (informational) hung for over an hour in `setup-e2e` before any test ran, so that job was rerun.
- `main` `ccbfb7c`: full run [42201946869](https://github.com/phynics/cast-iron/actions/runs/42201946869)
  succeeded, with every Chromium and WebKit shard green.

**Local**
- Step 1, in a worktree of the tag:
  - `check-library-upgrade.py` from PSRD to canonical: 9,968 old ids; 0 missing, 0 unresolvable,
    0 alias problems, 0 dangling references.
  - `build-site --psrd` succeeded; `check-publication.py` exit 0; 30 Chromium tests passed (with the
    four documented exclusions).
- Step 2, at `ccbfb7c`:
  - `cargo xtask content` and `build-site` gave `vc01cc1c20a652c37`; `check-publication.py` exit 0.
  - Chromium e2e for canonical-cutover, offline, install, update, shell-update, search and
    reference: 40 passed.
  - A scripted walk of the app served under `/cast-iron/` passed: install, search, entry, offline
    reload, search and legal page while offline, a legacy PSRD id redirecting to `ref/`, spell
    lists, a feat entry, and Books.
  - `cargo xtask published-ids` added 0 ids.

**Live** (headless Chromium against https://phynics.github.io/cast-iron/)
- Fresh profile, after each step:
  - The library installed, an entry opened, and an offline reload worked; search and the legal page
    worked while offline.
  - Every request went to `phynics.github.io`, with 0 console errors and 0 failed requests.
  - After step 2, the legacy id `core-rulebook/spells/fireball` opens `ref/spell/fireball`.
- Existing profile, upgraded from step 1 to step 2:
  - The profile was installed on the step 1 release, with Fireball starred.
  - Settings offered the app update, and Reload applied it. The library update then downloaded.
  - Afterwards, only `cast-iron-content-vc01cc1c20a652c37` remains. The star survived, and its
    bookmark moved to `ref/spell/fireball`. Offline reading works, with 0 page errors.

**Not verified:** physical devices and the accessibility check (#12), Firefox, and WebKit against
the live site.

## Plain-text corpus, step 6: the upstream import is removed, 2026-11-06

Step 6 of [#127](https://github.com/phynics/cast-iron/issues/127). The content compiler,
`cargo xtask`, `sources/`, `scripts/`, CI, and the e2e suite no longer contain the PSRD-Data and
Pf Data adapters, the `--psrd`, `--canonical`, and `--from-upstream` builds, or the upstream
download. The last commit that builds them is the tag `psrd-last-buildable`
(`9800663202f4cb9bcd4fad1af43c5ebf884b4a09`). Decision:
[the upstream import is removed](decisions.md).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `cargo xtask fetch-sources` | Downloads only the pinned corpus (tree hash `d103cc70...` verified) |
| `generated/content` before and after the removal | Byte-identical (`diff -r`): content version `v6aa587a686a3caf0`, 31,106 entries, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Two corpus builds from scratch | Byte-identical over all of `generated/` (content and reports): `7a6ec44b0e26030711cbbe41851f825e517927a828c6f5323dbf6fc4b797dc4a`. The reports changed shape (no deduplication counts, survey, or supplemental sections), so this hash differs from the step 3 entry |
| `dist/content` after `cargo xtask build-site` | Same tree hash as `generated/content` (`2c6005d6...`) |
| `python3 scripts/check-publication.py` | Exit 0. The registry comparison is replaced by a check that the sixteen core books are present |
| `npm --prefix tests/e2e test -- reference search install books legal --project=chromium` | 45 passed (a sample, not the full suite) |
| `cargo xtask check` | Passed: formatting, clippy, all workspace tests (including the new corpus build integration test), and the WASM build |
| Build from the tag | `cargo xtask fetch-sources && cargo xtask content --psrd` in a checkout of `psrd-last-buildable` gives the PSRD library `vcdd730994d863e50`, unchanged from the step 3 entry |
| `check-library-upgrade.py` from the tag against this branch's `generated/content` | Exit 0: 9,968 old ids, 0 missing, 0 unresolvable, 0 alias problems, 0 dangling references, 0 class-category errors |

Removed: 125 files and about 36,600 lines of Rust, 3,900 lines of Python, and the upstream inputs
and fixtures under `sources/`, `reports/`, and the compiler's test fixtures. Added: about 500 lines of
Rust (the corpus build integration test, the relocated facet, markup, and shard tests, and the
rewritten report writer).

Limitations: this is local evidence. CI has not run the changed content job, and nothing is
deployed. The PSRD release step needs a release commit whose Pages workflow builds with `--psrd`;
`docs/releasing.md` gives the exact preparation and verification, and records the one open
owner decision (how the prep commit reaches `release`). The tier table for Books categories is
checked against the five categories of the pinned corpus; a corpus bump with a new category fails
the `book_tier` test until the table lists it.

### Step 6, review round 1, 2026-11-06

Added after review: the published-id guard (`sources/published-ids.txt`, 41,074 ids: 9,968 PSRD
ids equal to the snapshot at the tag, and the 31,106 `ref/` ids of `v6aa587a686a3caf0`), the
`corpus-check` licensing gate, a CI-enforced `book_tier` corpus test, and an executable PSRD
release procedure in `docs/releasing.md`. Local verification, no CI or deployment claim:

| Check | Result |
|---|---|
| `generated/content` tree hash after the guard | `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3`, `diff -r` against the baseline empty; two builds from scratch identical (`7a6ec44b...dc4a` over all of `generated/`) |
| Guard on the real corpus | `cargo xtask content` passes; with an extra unresolvable id in a copy of the file the compiler exits 1 before writing any output and names the id |
| `cargo xtask corpus-check` on corpus `v0.1.0` with the licensing gate | Exit 0: 439 publications, all with a copyright; 431 covered by a `section_15` notice, 8 by `notice_pending` (Bestiary 6, Faiths of Purity, Goblins of Golarion, Horror Realms, Inner Sea Taverns, Kingmaker Player's Guide, Magic of Thassilon, Serpent's Skull Player's Guide); no exceptions |
| `book_tier` cached-corpus test | Fails with `CI=true` and no cache; prints a skip message locally without one |
| `cargo xtask check`, `check-publication.py`, `cargo xtask build-site` | Passed; `dist/content` tree hash `2c6005d6...` |
| `npm --prefix tests/e2e test -- reference search install books legal --project=chromium` | 45 passed (a sample) |
| PSRD procedure in a worktree of `psrd-last-buildable` | Followed as written: `content --psrd` (`vcdd730994d863e50`), the tag's upgrade check against this branch's `generated/content` by relative path exit 0, the one-line `pages.yml` edit on a prep branch, `build-site --psrd`, `check-publication.py` exit 0, and 30 Chromium tests pass with the four documented exclusions. The full suite does not load against the PSRD library (`entry-header.spec.ts`); six tests of the first sample fail on missing categories. `git merge -s ours` of the prep commit into `main` leaves the tree identical and makes the prep commit and `release` ancestors |

## Plain-text corpus, step 3: the build reads the corpus, 2026-11-06

`cargo xtask content` now compiles the canonical library from the pinned corpus
(`phynics/cast-iron-data` tag `v0.1.0`, commit `0e7c3abe620b80293e0a6382e2ab7a4b3372f8e4`,
pin in `sources/corpus.toml`). The pin has since moved to `v0.1.2`; the measurements in
this section are from `v0.1.0`. `--from-upstream` keeps the previous build for comparison.
Decision: [the pinned corpus is the source of truth](decisions.md).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| Pinned corpus | Tree hash `d103cc70e91293ebdd7026d0ac01013dfa3edfba6395ca8b4bbfb7d27a6df373`, 31,136 files, verified by `fetch-sources`; a hand-edited cache fails with the expected and actual hash |
| `generated/content` from the corpus vs `--from-upstream` | Byte-identical: `v6aa587a686a3caf0`, 31,106 entries, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Two corpus builds from scratch | Byte-identical over all of `generated/` (content and reports): `dfba02a16b4fa38f807933032b91f2d89e2998a0a7a63915672805c7f62107d8` |
| `dist/content` after `cargo xtask build-site` | Same tree hash as `generated/content` (`2c6005d6...`) |
| Reports | `validation.json` and `coverage.md` differ from the upstream build (one corpus note; no upstream survey or `link-pass.json`); `coverage.json` is in the content tree and identical |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0 (`--psrd` unchanged: `vcdd730994d863e50`, 9,990 entries) |
| `check-publication.py` | Exit 0 |
| `cargo xtask data-report` | Exit 0; tracked `reports/` files unchanged |
| `npm --prefix tests/e2e test -- reference search install --project=chromium` | 35 passed (a sample, not the full suite) |
| `cargo xtask check` | Passed |

## Plain-text corpus, step 1, 2026-11-06

`crates/corpus` writes and reads the format in `docs/corpus-format.md`.
`cargo xtask corpus-export` turns the compiled canonical library into text;
`corpus-check` validates it; `corpus-roundtrip` compares it with the compiled
library. The build is not switched, so this adds tooling and no library change.

Local verification on 2026-11-06 after the review changes (`v6aa587a686a3caf0`,
31,106 entries, 149,047 body links):

| Check | Result |
|---|---|
| `cargo xtask content` twice | Byte-identical: `v6aa587a686a3caf0`, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Change against the previous library (`vd14efc1479965461`) | 4 entries: adjacent text nodes merged in 3, one explicit heading anchor (`sec-ghast` in the Ghoul entry); `coverage.json`, `books.json`, `relations.json` byte-identical |
| `cargo xtask corpus-export` twice | Byte-identical, tree hash `4564c25541e4f9d7facb7a1f9707a20ae15127dfd1d211aa84ba404f4ce176e9` |
| Export size and count | 31,113 files, 82.61 MB (sum of file bytes) |
| `cargo xtask corpus-fmt --check` | 0 of 31,113 files unformatted |
| `cargo xtask corpus-check` | 0 errors; 80 drift warnings (creature 1, feat 79) |
| `cargo xtask corpus-roundtrip` | 0 mismatches, exact comparison; coverage rebuilt from the corpus equals `coverage.json` |
| PSRD `--psrd` | Unchanged: `vcdd730994d863e50`, 9,990 entries |
| `check-publication.py generated/content` | Exit 0 |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0: 0 missing, 0 unresolvable, 0 dangling |
| `cargo xtask data-report` | Exit 0; tracked `reports/` files unchanged |
| `cargo xtask check` | Passed (format, Python tests, native and WASM clippy, workspace tests, WASM build) |

Tree hash: SHA-256 over the sorted `file-sha256  relative-path` lines.

Earlier status, 2026-11-05. The repository is public. The latest observed successful
[main CI](https://github.com/phynics/cast-iron/actions/runs/41691686691) and
[Pages deployment](https://github.com/phynics/cast-iron/actions/runs/41691686716)
are for `d48eaa3c8dce901b80a777600ae09238647c41cd`. The release migration has
not replaced that deployment or repeated its live browser acceptance.
Earlier published iPhone Simulator observations remain in
[the journey report](research/iphone-journeys.md).

## Canonical library is the default build, 2026-11-05

`main` now builds the canonical Pf Data library by default. `cargo xtask content`
writes `generated/content/`, and `cargo xtask build-site` packages that directory
into `dist/`. PSRD-Data remains the carry-over input and the previous published
library: `cargo xtask content --psrd` writes `generated/psrd/`, and
`cargo xtask build-site --psrd` packages it. `--canonical` is still accepted as the
explicit spelling of the default.

**This is a default switch on `main`, not a release.** The `release` branch and the
live GitHub Pages site still publish the PSRD library (`vdb43853600b8ed8b`, 9,968
entries). Promotion needs the owner's approval and follows `docs/releasing.md`:
publish and verify the alias-capable shell on the PSRD release first, then promote
the commit that makes canonical the default (design section 6.5). Merging this
switch publishes nothing.

Local verification on 2026-11-05 against the pinned PSRD-Data and Pf Data commits:

| Check | Result |
|---|---|
| Canonical default, two runs | Byte-identical: `vd7596846128e349b`, 29,515 entries, 206,972,160 bytes; content tree hash `eb8d2e60a947ca9b70c14b0e3bff9f251705a38f5026c0b4d0dc8b6a10d6846c` |
| PSRD `--psrd`, two runs | Byte-identical: `vdb43853600b8ed8b`, 9,968 entries; content tree hash `4425ab68f1a668e926be7017457be6d4397cb597f41e7be68b82aaf5ddd46b07` |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0: 9,968 old ids, 0 missing, 0 unresolvable, 0 alias problems, 0 unexpected title changes, 0 wrong class categories, 0 dangling references; 5,710 direct, 1,898 shared, 115 derived, and 2,245 carried |
| `check-publication.py` on `dist/content` | Exit 0: 29,515 entries, 397 publications, 0 issues |
| Full browser suite | 110/110 passed: chromium 54, webkit 54, mobile-chromium 2; the `Licensing` case passed in both desktop projects |
| `cargo xtask check` | Passed (format, Python tests, native and WASM clippy, workspace tests, WASM build) |
| `cargo xtask data-report` | Passed; no change to the tracked `reports/` files |

Schema version 3 and search format 1 are unchanged. The default library's legal page
lists both dataset pins and all 951 Section 15 notices plus the reviewed
notice-pending list. This is local evidence only;
no CI, deployment, or live verification is claimed here.

### Notice coverage, 2026-11-05

Three Section 15 lines from the owner's copies of the books and eight reviewed
notice-pending publications are now pinned. The 453 in-scope publications split
into 434 matched, 8 notice-pending, and 11 comic labels without a notice. The
canonical library has 31,106 entries at `vf32215920f3724aa`; the legal screen
rejects 21 records, every one naming only a comic. Local verification:
`cargo xtask check` passes, two canonical compilations are byte-identical,
`cargo xtask data-report` reproduces the committed reports,
`check-publication.py generated/content` and
`check-library-upgrade.py generated/psrd generated/content` both exit 0, and the
`Licensing` browser test passes on chromium and webkit. No CI, deployment, or
live check is claimed.

### Search facet-chip latency, 2026-11-05

The canonical search view renders 479 chips: one per publication, kind, and facet value.
Chips waited on three repeated full-index passes. Every search rebuilt the discovery
snapshot, scanned all 29,515 documents once per facet field for the value lists, and
counted each chip with a cloned `Filters` plus lowercase allocation for every document
and facet pair. `SearchIndex` now builds per-value document lists once when it decodes the
index. `filter_counts_eligible` evaluates each query-matched document against the active
filter groups once and answers every chip from its failure mask, and Search caches facet
values and the discovery snapshot per selection revision. The lists use metadata that
search format 1 already carries, so nothing changes in `search.bin`.

`tests/e2e/measure-library.mjs` now records these times. Local run, fresh browser profile,
distribution server on localhost, canonical content `vd7596846128e349b`:

| Measure (ms) | Chromium before | Chromium after | WebKit before | WebKit after |
|---|---:|---:|---:|---:|
| Open Search: first chip visible | 225 | 153 | 236 | 174 |
| Open Search: every chip count settled | 235 | 164 | 249 | 188 |
| Select Spells: School and Spell level chips | 178 | 145 | 201 | 146 |
| Toggle one school: counts settled | 146 | 97 | 135 | 88 |
| Search engine p50 / p95 | 53.9 / 207.3 | 51.5 / 195.6 | 51.0 / 211.0 | 47.0 / 188.0 |

The "before" numbers already fit under a second on this machine, so the 60-second waits
were a cutover safety margin, not a reproduced local stall. They are gone from
`search.spec.ts`, which now uses the suite's normal 15-second expectation timeout; no
assertion was weakened. The remaining cost is DOM work for the 479 chips, not counting.

## Development and release migration

[GitHub Issues](https://github.com/phynics/cast-iron/issues) is the tracking
source. [#10](https://github.com/phynics/cast-iron/issues/10) tracks this migration.
`release` was created at the published `d48eaa3` commit without changing main or
existing history. Readback confirms Pages uses Actions and the `github-pages`
environment permits only branch `release`. Its old `source.branch: main` API
field is legacy metadata, not the Actions publication selector.

The guarded workflow and promotion helper are prepared in
[PR #16](https://github.com/phynics/cast-iron/pull/16) against main. Main publication is already blocked by the environment policy. The workflow
must merge and be explicitly promoted before the first release-branch deployment.
Local workflow validation passes with actionlint 1.7.12. Eight promotion safety
checks pass, and a real preview against `d48eaa3` confirms the successful main
CI and unchanged release baseline. A real non-main promotion attempt was
rejected by the ancestry check before any push.
[Wrong-ref dispatch 41725677628](https://github.com/phynics/cast-iron/actions/runs/41725677628)
ran on this feature branch and skipped both build and deploy jobs. This verifies
the workflow guard on GitHub without publishing. Local `cargo xtask check`
passes, including formatting, native and WASM clippy, workspace tests, and the
WASM build. No new distribution or browser suite was run for this workflow/docs
change. Final-head GitHub CI is separate and remains visible on the PR.

No release-branch deployment or live migration verification is claimed. See
[release operations](releasing.md) for exact completion and recovery steps.

### Canonical id alias map (Phase D)

[PR #49](https://github.com/phynics/cast-iron/pull/49) on `data/pf-derived-rules`
added the Phase D alias map; the proposal pass below extends it.
`sources/canonical/id-map.toml` gives every one of the 9,968 frozen legacy ids one
disposition. The current map has 9,968 mapped (5,735 direct, 115 derived, 2,216
carried, 1,902 shared) and none retired; the pre-proposal state was 94 mapped
(1 direct, 93 derived), none shared, and 9,874 retired. `cargo xtask data-alias`
rebuilds the canonical catalog from the pinned Pf Data corpus and the PSRD
snapshot, runs totality, injectivity, target, direction, hash, and semantic-fit
gates, and writes `generated/content/<version>/aliases.json`. The carried rows
come from the all-unmatched product decision: each re-emits the PSRD entry at
`ref/carried/<legacy-id>`, and the gate adds only the carried entries the run
actually produced.

`cargo xtask data-alias-propose` proposes the dispositions through the same
`title_fit` function the gate uses. It requires equal kind, a source book in the
record's publication set, and equal normalized titles or a documented comma
inversion, family/category parenthetical, or item measure/package parenthetical. It
does not match on a slug. Ties resolve only by native-over-derived, a racial-trait
parent race named in the legacy path, or a weapon/armor special-ability path; other
ties stay retired with an `ambiguous: <candidates>` note. The 94 already published
rows are unchanged, and four proposals that would merge onto them stay retired.

Carried rows by kind: rules 512, items 418, creatures 327, racial traits 275,
afflictions 214, spells 179, feats 165, traps 58, races 36, skills 26, archetypes
4, traits 2. The carry-over reports 434 possible duplicates with candidate ids
for later review; none of them changes a mapping. It rewrites 7,238 references
(6,201 body, 438 parent, 439 relation, 160 feat prerequisite) and flattens none.

Local verification on 2026-11-05: `cargo xtask check` passes, two
`cargo xtask data-alias` runs produced a byte-identical alias file
(`5dd41777f712bfd14fa4a084d8b6b77f7b65b38874c838cfb2a592322c4bfe9c`, content
`va174cce225939e2e`), and two `cargo xtask data-report` runs produced the same
`reports/migration-coverage.json`. This is local evidence only; the PR's GitHub
checks are separate and are not claimed here.

Phase 5 makes the map part of the library. `cargo xtask data-alias` adds
`aliases.json` to `manifest.json` and recomputes the content version deterministically.
`Library::resolve` reads the map eagerly and resolves one step (catalog, alias, then
retirement). On the first load of a library that lists a map, an idempotent IndexedDB
transaction rewrites bookmark and recent ids to their canonical targets, merges
many:1 rows, and records `meta["id_migration"].aliases_sha256`; retired and unknown
rows keep their snapshots. Schema version 3 and search format 1 are unchanged.

Local verification on 2026-11-04: `cargo test -p cast-iron-web` (104 lib tests),
`cargo test -p cast-iron-content-compiler`, and `cargo xtask check` pass.
`cargo xtask data-alias` produced a manifest-listed alias file
(`ve730cbbe80014fec`, 94 mapped and 9,874 retired rows). `check-library-upgrade.py`
compared the pre-alias and post-alias library: 9,968 ids, zero missing, zero title
changes, zero incorrect class categories, zero dangling references. The new
`canonical-cutover.spec.ts` fixture test passes in Chromium and WebKit. This is local
evidence only; the production cutover is not live and the deployed library still uses
PSRD ids.

Review follow-up on [PR #51](https://github.com/phynics/cast-iron/pull/51) fixes the
findings above against the same design: the merged-kind tie-break uses the serialized
slug, the star contract resolves stored rows through the alias map (pressed, add, unstar,
and collection membership), the bookmarks and Home lists collapse one saved item to one
row, a refused conversion shows the Settings notice, an unreadable migration marker reads
as absent, catalog membership trusts the sorted catalog, a retired entry route shows its
reason category, and `AliasIndex::validate` enforces the one-step and one-disposition
rules. Local verification on 2026-11-04: `cargo test -p cast-iron-web` passes with 113
lib tests, `cargo test -p cast-iron-content-compiler` passes with 162 lib and 20
integration tests, and `cargo xtask check` passes (format, native and WASM clippy,
workspace tests, WASM build). No `dist/` build or Playwright rerun was made for this
follow-up, and no production cutover is claimed.


## Outstanding work and UI history

- [#8](https://github.com/phynics/cast-iron/issues/8) tracks installation controls;
  [#9](https://github.com/phynics/cast-iron/issues/9) tracks the update indicator.
- [#11](https://github.com/phynics/cast-iron/issues/11) owns ambiguous navigation
  mapping and missing-book/source investigation following draft
  [PR #7](https://github.com/phynics/cast-iron/pull/7). That PR already implements
  portable stars, publication choices, and its coverage audit.
- [#12](https://github.com/phynics/cast-iron/issues/12) owns physical-device and
  accessibility acceptance, including true offline standalone restart.
- [#13](https://github.com/phynics/cast-iron/issues/13) owns printed-order evidence.
- [#14](https://github.com/phynics/cast-iron/issues/14) owns remaining standalone
  corpus families and relationship coverage.
- [#15](https://github.com/phynics/cast-iron/issues/15) owns remaining reference
  rendering and discovery limitations.

[UI implementation history](ui-tasks.md) retains the completed local work and
original acceptance context. It is not a second backlog. The supplied Gemini
review remains recorded in [the review follow-up](research/gemini-ui-review.md).
The measurements below are historical evidence for their named builds.

## Acceptance checklist (spec section 11)

"Local" means the Playwright suite in `tests/e2e` passed against a real `dist/` build served at `/cast-iron/` by `tests/e2e/server.mjs`. Last local full run: 55 passed in 2.3 minutes (chromium 27, webkit 27, mobile-chromium 1), including relationship navigation, spell lists, feat prerequisites, source search, book history, and mobile layout. `cargo xtask check` also passed.

| Item | State | Evidence |
|---|---|---|
| Static GitHub Pages deployment works | Live pass | [Deployment run 41576342801](https://github.com/phynics/cast-iron/actions/runs/41576342801) succeeded for `a6636f3`. Chromium installed the library from [the public site](https://phynics.github.io/cast-iron/), verified the contact and OGL notice, reloaded offline, and found Fireball offline. No page errors or third-party requests occurred. |
| Application is installable where browser support allows | Local pass | `Application is installable where browser support allows` |
| Representative rules dataset installs completely | Local pass | `Representative rules dataset installs completely` |
| Download integrity is verified | Local pass | `Download integrity is verified` (corrupted shard rejected; retry fetches only that file) |
| Search operates entirely offline | Local pass | `Search operates entirely offline` (p95 recorded per run; a 250 ms smoke bound is asserted, not a latency gate). On the canonical library 2026-11-05: p95 7.4 ms Chromium, 8.0 ms WebKit over 20 queries; [#84](https://github.com/phynics/cast-iron/issues/84). |
| Category and sourcebook browsing works | Local pass | `Category and sourcebook browsing works` checks every publication; `Expanded books and separate class categories work offline` checks 16 books and the distinct categories. |
| Rule entries preserve complex formatting | Local pass | three `Rule entries preserve complex formatting: …` tests |
| Cross-reference navigation works | Local pass | `Cross-reference navigation works`, plus the deep-link test |
| Back/forward navigation works | Local pass | `Back/forward navigation works` |
| Stars persist across restarts | Local pass | `Home keeps legacy collection members as unique stars across offline restart` and `More than six starred entries stay independently openable and removable` |
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

## Published navigation and common journeys

Primary destinations are Home, Browse, Books, and Settings. Search is a bar, and Home shows up to six saved entries plus recent reading. Quick scopes expose Rules text, Spells, and Feats. Rule headings feed the existing search alias field: `grapple` with Rules text selected surfaces Special Attacks first in the checked build. A native regression requires that entry in the first three results.

Content version `v171f0348e5717ee0` has 9,967 entries, schema 3, 61,956,172 bytes raw, and a 20,329,611-byte search index. Search format remains 1. The upgrade check preserves all 9,967 prior IDs and titles with no kind changes or dangling derived references. Publication checks and `cargo xtask check` pass. The full built-site suite passes 55 tests. Desktop and mobile screenshots in both themes were inspected; the screenshot run reported no horizontal overflow or console errors.

Composed offline journeys cover quick lookup, creating a collection and reopening a bookmark from Home, racial traits, class options and archetypes, class spell levels, traits, equipment, rules, and reading onward from book contents. A new app-shell test verifies that the worker waits for Reload, preserves the installed library and Home bookmarks, and works after an offline reload. The library-update test verifies replacement activation separately.

### Published iPhone Simulator checks

On iPhone 18 Pro, iOS 27.2, the primary agent personally applied the waiting app-shell update and replaced library `vdb8e844bfc068446` with `v171f0348e5717ee0` in Safari. The existing Races bookmark survived both updates. A named collection held that entry. Native Add to Home Screen installation and standalone launch succeeded.

The standalone app required its own library download and initially had no Safari bookmarks. After installing there, sampled journeys passed through Dwarves and racial traits, Fighter and Archer, Magic Missile and the Wizard level 1 list, combat traits, armor, Core Rulebook contents, and Generating a Character. Rules text placed Special Attacks first for `grapple`, but opened the beginning of the long article. Next in book opened Defensive Training from Darkvision. Closing and reopening the standalone app preserved a Special Attacks bookmark and recent reading. Sampled light and dark screens fit the viewport.

True offline simulator networking was not available through the driver. The relaunch check ran with networking available. A full subsystem read with backtracking, feat chains, and a complete character build remain open manual checks. See [iPhone journey observations](research/iphone-journeys.md) for the earlier baseline, follow-up, driver failures, and recovered actions. Automated offline acceptance is separate evidence.

Navigation CI and deployment succeeded in runs 41595871949 and 41595872013. The earlier schema-3 reference UI deployed successfully in run 41590678502. The 16-book schema-2 deployment succeeded in run 41579539956. POC PR CI passed on GitHub in runs 41569305309 and 41569305238. The first Pages attempt failed before Pages was enabled. Deployment run 41576342801 then succeeded and the public site passed the live smoke check.

## Acceptance limits, tracked in [#12](https://github.com/phynics/cast-iron/issues/12)

- Firefox: the Playwright project remains available for debugging, but CI no longer installs or runs it. Playwright's Firefox does not start on the development machine (macOS 27). In [CI run 41697800843](https://github.com/phynics/cast-iron/actions/runs/41697800843), all 37 Firefox tests failed and the step took 57 minutes 55 seconds. The installer remained at its initial storage check. Local verification: `cargo xtask check` passed, and a parsed-workflow check confirmed Firefox is absent from install and test commands while Chromium, mobile Chromium, and informational WebKit remain enabled. CI verification of the updated workflow is pending.
- Physical devices: Android Chromium and iOS Safari, including offline standalone relaunch. Simulator standalone relaunch passed with networking available.
- Persistent storage: headless Chromium reports "not granted". On real browsers the result depends on browser heuristics.

## Known limits and issue ownership

- Historical baseline, addressed by the reference simplification below: the iPhone Simulator follow-up found separate Safari and Home Screen library and bookmark stores. Installation guidance does not explain that transition. Long-rule search results also open the article start rather than the matched heading. See the simulator report for scope and other UX findings.
- Licensing review is scoped to the 16 selected books. The publication screen covers all 9,967 entries and found no issue, but does not classify every sentence or establish blanket rights to new sources. See `docs/research/licensing-audit.md`.
- Paizo Community Use Policy: on 2026-11-03 the official policy, FAQ, reinstatement post, and licensing overview were fetched and compared with the built Legal page. The notice matches the policy text, and the page shows a contact link. No registration is required. This does not clear every sentence of the dataset, and Paizo can withdraw the permission at its discretion. Details are in the audit.
- [#13](https://github.com/phynics/cast-iron/issues/13): Book-order fidelity is incomplete. Core Rulebook main chapters are verified; other outlines, nested placement, and appendix grouping need publisher evidence and mapping work. Zero unplaced entries proves coverage, not printed order.
- [#14](https://github.com/phynics/cast-iron/issues/14): Advanced Race Guide standard racial traits remain embedded in race bodies rather than standalone records. Alternate-trait replacement links are not modeled.
- [#14](https://github.com/phynics/cast-iron/issues/14): Eight spell-list names lack a class entry, as listed above. The lists cover installed spells; they do not establish complete coverage of those classes or their books. Feat chains contain extracted named links, including alternatives, rather than complete eligibility logic.
- [#43](https://github.com/phynics/cast-iron/issues/43), [#116](https://github.com/phynics/cast-iron/issues/116): The "Referenced by" panel shipped in #106; the remaining gaps are tracked here. Typed relations have inverse lookups in the entry screens, and the panel covers body links, but old libraries and the listed follow-ups are still open.
- [#15](https://github.com/phynics/cast-iron/issues/15): Category lens counts and source counts are global. Search facet chips carry counts conditional on the other active filters; those counts are not used by the category lenses.
- [#15](https://github.com/phynics/cast-iron/issues/15): In spell and creature `spells` stat lines, upstream lists become lines with breaks, so list structure is lost.
- [#15](https://github.com/phynics/cast-iron/issues/15): Heading ids are not prefixed, so a content anchor could collide with an app element id such as `main`.
- Firefox, and a single flaky run of `Back/forward navigation works` on Chromium, which could not be reproduced (see `docs/decisions.md`).

## Published reference simplification

The local implementation separates Browse categories from Books, removes the duplicate mode picker and Rules category, and keeps rules accessible through Books, search, saved links, and starred entries. Home lists every star with an independent Unstar control. Collection and bookmark-manager UI is retired while legacy stored stars remain readable. Old manager URLs lead to Home, and the former Rules-category URL leads to Books.

Books has persistent Active controls. Disabling filters ordinary discovery, counts, search candidates, relationships, and spell lists, while preserving the download and canonical reading access. Corrupt book preferences offer Retry or Reset active books without discarding the library or stars. Direct disabled-book entries show a notice. Book placement determines visibility; search publication filters still describe attribution sources.

Primary rule text precedes related navigation. Large archetype lists start collapsed. Rule search links to matching typed headings, article contents supports repeated jumps to the current section, and exact named sections rank ahead of longer item-title prefixes. Prestige requirements link unambiguous named feats and skills. Source badges remain links, and source-grouped References avoids duplicate racial-trait and archetype targets. Installer copy explains the optional Home Screen installation before the library download and the observed possibility of separate Safari data.

The installed corpus remains 9,967 entries at `v171f0348e5717ee0`, schema 3 and search format 1. No source content was imported. The AoN inventory contains 23 source headings, 50 topics, and 233 subsections. Strict within-book title mapping yields 95 unique section matches, 16 ambiguous matches, and 172 unmapped rows. These are mapping evidence, not printed-order verification or complete AoN coverage.

Local verification: `cargo xtask check` passes, including 21 search and 94 web unit tests. The real `dist/` build succeeds and its content manifest matches `generated/`. Six targeted Chromium checks pass for the integration fixes and recovery paths. The full browser run passed 66 cases and found one WebKit fixture failure when setup navigated away during its initial manifest request. Setup now waits for the Download offer; that WebKit case passed on rerun. All 67 distinct cases are verified across Chromium, WebKit, and mobile Chromium. The complete suite was not repeated after the fixture-only change.

Two bounded visual batches covered 390 x 844 and 1280 x 900 in both themes. The confirmation captures reported zero horizontal overflow, no undersized alphabet or spell-level controls, and no console errors. Separate settled search captures confirm exactly one selected scope and rule-only results. The search-only capture waits for both the selected scope and rule-only result kinds, avoiding the stale-state screenshot that appeared during capture setup. Selected-scope foreground/background contrast measured 6.02:1 in light and 6.54:1 in dark. Representative PNGs were opened and inspected. The rerunnable capture command is in the verify skill.

The first browser pass caught blocked inline article-contents indentation and stale test assumptions about star ordering, labels, and open disclosures. Those were corrected. A later affected test exposed repeated section links failing to scroll when the URL already matched; explicit scrolling fixes that case. The corrected six-test run passed. The final full run passed the CSP and repeated-heading cases in both engines. An independent bounded integration review found no concrete P1 or P2 issue.

This simplification merged in [PR #6](https://github.com/phynics/cast-iron/pull/6) at `d48eaa3`. Its successful main CI and Pages deployment are linked at the top of this page. This migration did not repeat live browser acceptance for that artifact. Physical iPhone offline relaunch, VoiceOver, and a full physical-device character-building walkthrough remain unverified in [#12](https://github.com/phynics/cast-iron/issues/12). Earlier published simulator observations and automated offline checks remain separate evidence.

## Local follow-up: portable stars and publication choices

Note: manual star export/import described below was later removed in PR #28;
stars stay local to the browser profile (see `docs/decisions.md`). The
publication-choice grouping remains shipped behavior.

Home now offers manual star export/import through a bounded, versioned JSON archive. Import validates the complete file before writing and merges by permanent entry ID in one transaction. Existing stars win and repeated imports add no duplicates. This transfers files between independent browser profiles; it does not synchronize Safari and Home Screen storage automatically.

Search and category lists group repeated titles within an entry kind after active-book filtering. Publication choices retain separate IDs, source labels, rule content, and star state. Search limits groups rather than truncating publication choices, and reports the underlying entry count. Prestige requirements retain multiple named matches as explicit source choices. Two reviewed Combat navigation labels use consistent connector casing without changing canonical titles or rule bodies.

The AoN audit now includes nested typed headings, inline formatting, entry IDs, and anchors. Of 283 topic/subsection rows, 90 have one exact target, 49 have ambiguous targets, 39 lack a matching section in an installed source, and 105 belong to 14 source headings outside the installed corpus. These are mapping dispositions, not imported content. The complete missing-source list and regeneration command are in [mapping resolution](research/aon-mapping-resolution.md). The corpus remains 9,967 entries in 16 books at `v171f0348e5717ee0`, schema 3 and search format 1.

Native verification passes `cargo xtask check`, including 99 web tests. Six extractor fixture tests pass, and two mapping regenerations compare byte-identically. The final distribution build has shell `1135a12922754f39`; its content manifest matches `generated/`.

The primary agent also used a manual browser walkthrough to open Core Rulebook > Equipment > Armor, read the rendered article, jump to Masterwork Armor, follow Armor Spikes, use Back and Forward, and step to Banded Mail and back. No console errors appeared. Automated offline acceptance visits all 24 Armor article contents stops and follows a typed cross-link with reading history. The article is the sampled scope; this does not claim every linked equipment entry was read.

A 52-capture batch covers phone and desktop widths in both themes, including open publication choices and star transfer. It reports no horizontal overflow, undersized alphabet/spell-level controls, or console errors. Representative PNGs were opened and inspected. The screenshots were kept outside the repository and are not committed.

Physical-iPhone VoiceOver and genuine offline standalone restart remain unverified. Only an iPhone 18 Pro iOS 27.2 simulator is connected. The device driver's settings support no VoiceOver control, and its Wi-Fi/airplane switches only override the simulator status bar. The verify skill contains the remaining physical-device steps. PR #6 is merged; this follow-up is not deployed, and its CI result is not yet verified.

The final browser matrix passed 73 of 75 cases. The two WebKit failures were test setup issues: the keyboard check expected Tab to focus a button, while this WebKit environment requires Option-Tab; and the transfer test navigated twice before installation, aborting the initial requests with `pageerror: Load failed`. The keyboard failure reported `Expected: focused; Received: inactive`. The corrected two-test WebKit rerun passes. All 75 distinct cases are therefore verified across Chromium, WebKit, and the mobile Chromium layout project. The entire matrix was not repeated after these test-only corrections. No browser console-error allowance was added.

Earlier exploratory failures caught stale assertions that counted nested source-choice titles, attempted to open a closed race choice before incremental list rendering, and assumed Special Attacks supplied typed cross-links. Tests now wait for actual rendered choices, follow the disclosure, count top-level result groups, and use the linked Armor article for the combined reading journey. The corrected Chromium cases pass. The final native/build verification and the 52-capture batch use the final application code.

### Independent review follow-up

An independent review of head `2982678` reported no P1/P2 findings and one P3 archive-size bug, tracked in [#17](https://github.com/phynics/cast-iron/issues/17). A compact 1,991,340-byte archive could import but exceed the export limit after pretty-printing. Export now uses compact JSON, and import checks canonical serialized size, including numeric normalization, before any write. Two native boundary tests cover the reproduced case and a compact integer-date input whose normalized encoding exceeds the limit. `cargo xtask check` passes with 101 web tests. The rebuilt distribution has shell `9104b7a769594cbf`; the unchanged content manifest matches generated content. Targeted Chromium and WebKit star-transfer acceptance both pass. This follow-up is locally verified; no new deployed result is claimed.

## Local book target and source audit

Issue [#11](https://github.com/phynics/cast-iron/issues/11) tracks this work. The 283 AoN directory rows now contain 90 exact-title targets, 60 source-validated passage targets, two source-validated chapter contexts, 26 rows without a verified installed target, and 105 rows under 14 absent source headings. All 49 prior title ambiguities have target evidence; 12 prior unmatched labels refer to installed passages. These are target resolutions, not new library text. The two chapter cases preserve explicit opening-passage limitations.

The pinned Pf Data audit contains source-tagged records for all 14 absent headings. PCGen has matching source manifests for nine. A source-bounded passage probe found candidate records for 12 of the 14 remaining opening passages missing from the installed snapshot; it did not find the Bestiary or GameMastery Guide introductions. Exact source pins, count methods, JSON pointers, hashes, and directive inventories are committed as research evidence. The corpus is now 9,968 entries in 16 books. No new source is shipped by this audit, and no deployment is claimed.

Verification for this evidence change: `cargo xtask check` passes, including 23 Python audit/mapping fixtures and 95 web unit tests. Replayed passage/chapter evidence, applied mappings, and pinned source inventories compare byte-identically. Compiler inputs and client behavior are unchanged by the research files. Remaining corpus work stays open in [#11](https://github.com/phynics/cast-iron/issues/11), the parallel migration [#18](https://github.com/phynics/cast-iron/issues/18), and the bounded source adapter [#19](https://github.com/phynics/cast-iron/issues/19). A new corpus can invalidate the evidence hashes; regenerate against its actual entries/book index rather than weakening those checks.

## Reviewed supplemental source import

Local branch `data/18-pf1e-corpus-migration` adds a native adapter for the pinned
`jasontankapps/pathfinder-data-1-e` snapshot and imports one reviewed record:
`pf-data/rule/magic`, "Magic and Spellcasting". It is the missing Core Rulebook
page 206 opening passage and is placed first in the existing Core Rulebook Magic
article. Its permanent ID uses the native protocol namespace; all earlier IDs and
titles are unchanged. Every other primary-source record stays in coverage as
`record shape ... is not approved`, not in the library.

Content version `vff56b0e6310710c1` has 9,968 entries (was 9,967), schema 3 and
search format 1. Raw content is 62,123,015 bytes (was 61,956,172); the search
index is 20,330,948 bytes (was 20,329,611). Resolved links are 1,310, including
the six reviewed links from the new passage. The legal page lists the new dataset
pin and appends the pinned source's complete Section 15 notices. Supplemental
coverage rows carry a `pathfinder-data-1-e:` prefix, so they cannot be read as
PSRD-Data paths.

Local verification: `cargo xtask check` passes. Two `cargo xtask content` runs
produce the same tree hash. `scripts/check-library-upgrade.py dist/content
generated/content` reports no missing IDs, no changed titles or kinds, and no
dangling references. The offline Chromium check `Reviewed primary-source passage
appears under the Core Rulebook Magic article` passes against the real `dist/`
build. `cargo xtask data-report` is reproducible and its tracked reports are
committed; the full-corpus experiment remains blocked and is not shipped.
No CI, deployment, or full browser-matrix result is claimed for this content
change. Apollyon Clasp and the other unreviewed comic and blog material remain
excluded. Licensing decisions are tracked in
[#34](https://github.com/phynics/cast-iron/issues/34). Two PF2 publications are
registered in the pinned source (`Monster Core`, `Bestiary 3 (2nd Edition)`);
eight tagged records, the `main.json#ogc` ORC notice, and the untagged PF2
*Player Core* description in `gnome_flick_mace` are explicitly out of scope and
cannot enter the PF1e path. Comics are ignored permanently; blog records are
deferred to [#36](https://github.com/phynics/cast-iron/issues/36). The PSRD-Data
dataset-license risk is overridden for now and tracked in #34.

## Canonical page-ordered book index, 2026-11-05

[#18](https://github.com/phynics/cast-iron/issues/18) now extracts page attribution
from the screened Pf Data `‹SOURCE Publication/page›` lines into the canonical
converter and builds a page-ordered `books.json` for the `ref/` library. The full
pinned corpus (pin `08ea71f24375908d0190c822fd1fd080797667a9`, 27,853 converted
entries) yields:

| Measure | Full corpus | Representative `data-report` |
|---|---:|---:|
| Books | 454 | 49 |
| Entries | 27,853 | 42 |
| Placed by page | 24,129 | 37 |
| Unpaged | 3,724 | 5 |
| In more than one book | 4,872 | 9 |
| Without publication attribution | 0 | 0 |
| Placements | 35,700 | 82 |
| Paged placements | 29,738 | 47 |
| Unpaged placements | 5,962 | 35 |

The Core Rulebook is ordered by page: `Casting Spells` at page 206 follows
`How Combat Works` at page 178 in the flattened book. Two `build_canonical` runs
over one full conversion are byte-identical, and `cargo xtask data-report`
reproduces the tracked representative reports. The canonical index has no curated
chapters yet; `book-order.toml` matches PSRD urls, so its chapter list remains a
later refinement. At that time the default `cargo xtask content` produced
`vdb43853600b8ed8b` with 9,968 entries, byte-identical to the baseline production
build; that library is now `cargo xtask content --psrd`. Schema 3 and search format
1 are unchanged. This is local evidence; the full migration remained blocked and
was not shipped at that time.

## Full Pf Data corpus root-cause findings, 2026-11-04

[#18](https://github.com/phynics/cast-iron/issues/18) now has a full-corpus run
that reports root causes instead of reference cascades. Local 2026-11-04 results
against pin `08ea71f24375908d0190c822fd1fd080797667a9`:

| Measure | Before | After |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 24,610 | 27,853 |
| Findings | 49,035 | 2,174 |
| High findings | 47,605 | 1,831 |
| Medium findings | 1,430 | 343 |
| Mandatory fixtures | 1/6 | 6/6 |

The largest previous causes were 42,611 unresolved references, of which 39,207
pointed at records that were selected but had failed conversion. Those links are
now `reference-to-failed-record` coverage under the target's own conversion
finding. The remaining high findings are distinct conversion causes: 161 records
without name or publication metadata, 152 `::trait` metadata records, 147
`::prereq` records, and smaller families such as `::elephant`, `::room`,
`::haunt`, `::trap`, and `::drug`. The deliberate `::aff block has no name`
rejection stays counted as 332 medium findings.

Representative mode still reports 0 findings and 6/6 fixtures, and the tracked
`reports/migration-test.*` and `reports/migration-coverage.json` are unchanged.
Two `cargo xtask data-import` runs produced byte-identical reports and generated
full-mode output. `cargo xtask content` produced the same `vff56b0e6310710c1`
content version and a byte-identical tree to `main`, so production PSRD content
is unchanged. Schema 3 and search format 1 are unchanged. This is local evidence;
the full migration is still blocked and not shipped.


## Canonical library compile (P7), 2026-11-05

`cargo xtask content --canonical` now builds the complete canonical library into
`generated/canonical/` through the same `pipeline::package` tail as the PSRD
build. The measured local build on 2026-11-05, against PSRD pin
`76ea4784604145ba3dea40eb56f83bb904759467` and Pf Data pin
`08ea71f24375908d0190c822fd1fd080797667a9`, is:

| Measure | Value |
|---|---:|
| Content version | `vf49db728890fffb9` |
| Schema / search format | 3 / 1 |
| Entries | 28,777 |
| Total library bytes | 197,486,346 |
| `search.bin` | 62,295,124 |
| Shards | 235 |
| Largest shard | `entries/item-016.json`, 524,282 bytes |
| Books | 436 |
| Publications (with entries) | 456 (394) |
| Relation edges | 2,142 (`part` 342, `archetype` 332, `prerequisite` 1,468) |
| Book placements / unpaged | 22,997 / 5,780 |
| Alias rows / retired | 9,794 / 174 |
| Section 15 notices | 660 |
| Dataset pins | `psrd-data`, `pathfinder-data-1-e` |

Entries by kind: 8,006 items, 7,370 rules, 3,368 feats, 3,360 creatures,
3,099 spells, 1,631 traits, 942 archetypes, 342 afflictions, 342 racial traits,
111 prestige classes, 103 races, 58 traps, 26 skills, and 19 classes. The
library carries 2,216 PSRD carry-over entries.

The legal screen rejects 1,292 records whose publications have no Section 15
notice in either the PSRD or Pf Data list; 5,075 references to those records are
flattened or dropped and counted under `legal-reference-settlement`. The alias
file retires 174 rows whose planned canonical target the converter did not emit
and none for rights. The PI screen records 1,301 omissions across the in-scope
inventory. Coverage rows also carry 17,863 converter/out-of-scope/PI/carry-over
rows and 3,400 rejected records.

Two consecutive builds produced byte-identical trees, including `manifest.json`,
`aliases.json`, and every shard. `python3 scripts/check-publication.py
generated/canonical` passes with 0 issues. `cargo xtask content` (default) still
produces `vdb43853600b8ed8b` with 9,968 entries, byte-identical to
`origin/main`. `cargo xtask check` passes.

`python3 scripts/check-library-upgrade.py generated/content generated/canonical`
exits nonzero because the script is not alias-aware and expects one global book
placement: it reports all 9,968 PSRD ids as missing (the canonical library uses
`ref/` ids and ships the map in `aliases.json`), and 17,042 `books.json`
bookkeeping entries as "placed twice" or "neither placed nor reported". The
latter is the script's single global `placed` set: a canonical entry may appear
in several books, and an entry without a page is both listed under Unpaged and
recorded in `book_order.unplaced`. Schema validation reports no dangling link,
parent, relation, or book target. The script was not weakened.

This is a local build, not a deployment. PSRD was still the production default at
that time; the later default switch is recorded at the top of this page. P8
acceptance on the later facet-carrying build (`v523b94bd49b15699`) is recorded
under [Canonical library acceptance (P8)](#canonical-library-acceptance-p8-2026-11-05).

## Canonical library acceptance (P8), 2026-11-05

At the time of P8, `cargo xtask build-site --canonical` packaged `generated/canonical/`
into `dist/` through the same Trunk build, and the default still packaged PSRD-Data
from `generated/content/`. The default now packages the canonical library from
`generated/content/`, and the PSRD build is `--psrd` into `generated/psrd/` (top
section). The Playwright suite reads its expectations from the compiled corpus
selected by `CAST_IRON_CONTENT` (`generated/content` by default). Both libraries were
checked against the same full suite, one `dist/` at a time. The command block below
records the P8 commands; the paths moved with the default switch.

| Project | Canonical | PSRD |
|---|---:|---:|
| chromium | 54/54 | 54/54 |
| webkit | 54/54 | 54/54 |
| mobile-chromium | 2/2 | 2/2 |
| Total | 110/110 | 110/110 |

Commands, run from the repository root:

```sh
cargo xtask content --canonical
cargo xtask build-site --canonical --public-url /cast-iron/
CAST_IRON_CONTENT=generated/canonical npm --prefix tests/e2e test -- \
  --project=chromium --project=webkit --project=mobile-chromium

cargo xtask content
cargo xtask build-site --public-url /cast-iron/
npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium
```

The canonical library in these runs is content `vd7596846128e349b`: 29,515 entries,
206,972,160 bytes, 65,725,435 bytes of search index. The PSRD build is
`vdb43853600b8ed8b`: 9,968 entries and 62,123,015 bytes. Two consecutive
`cargo xtask content --canonical` runs produced the same tree hash
5ac59bee1dcec9f72596ba1233446d952dbff8f73b2ae1be73ee3bb26ab830cb. `cargo xtask check` passes.

Measurements on that build, from `tests/e2e/measure-library.mjs` with a fresh profile and
the distribution server on localhost:

| Measure | Chromium | WebKit |
|---|---:|---:|
| Install, Download click to reader (s) | 1.3 | 2.3 |
| Storage usage / quota | 209.1 MB / 10.9 GB | 208.9 MB / 20.6 GB |
| First entry render (ms) | 32 | 36 |
| Search engine p50 / p95 (ms) | 53.5 / 200.0 | 52.0 / 211.0 |
| Search input to results p50 / p95 (ms) | 178 / 324 | 174 / 342 |
| JS heap after search | 5 MB | not available |
| Console errors | 0 | 0 |

The storage figures are `navigator.storage.estimate().usage`: the library plus the app
shell, about 1.0–1.9 % of the reported quota. No quota error occurred. The heap figure is
the Chromium JS heap only; WebAssembly linear memory is a separate allocation, so it is a
floor rather than the whole process. WebKit exposes no equivalent counter.

Acceptance surfaced two canonical-only gaps and two app defects:

- Search facets were empty in the canonical index, so the school, level, CR, and similar
  filters never appeared. `pipeline::build_canonical` now derives them from `TypedFields`
  and the typed archetype edge. Race groups remain absent because the pin stores them only
  in upstream HTML.
- Prestige-class requirements are prose in the canonical records, not typed fields, so the
  prestige pages have no named requirement links there. The requirement-link acceptance
  uses a feat, whose canonical records do carry typed prerequisite edges. Prestige
  category and publication navigation are still asserted.
- Search blocked for about 48 seconds before showing results offline. Every one of the 397
  publication chips rebuilt the whole discovery set for its own active-book total. A memo
  now computes all source totals in one pass per selection revision, and the chip counts
  read the search's `FilterCounts`; the first offline search now reaches results in about
  one second.
- Publication titles longer than a phone screen widened `.entry-list` rows and the page
  itself. `.row-source` and the row link now shrink and wrap, so `#/browse/kind/spell` has
  no horizontal page scroll at 390 px. Light and dark 390 px screenshots of that screen
  show the wrapped row inside the viewport.
- About 35 canonical spells carry upstream reference markup in their `school` attribute,
  such as `‹spelldef/abjuration› &#91&see text&#93&`. Their stat rows and the new school
  facet show that markup. Rendering attribute references is converter work outside this
  acceptance; it is recorded here rather than hidden, and the entries stay readable and
  searchable.

The sixteen PSRD books carry their curated chapters in this build; every other publication
is page-ordered. The canonical library also publishes both `core-rulebook` and a duplicate
`prpg-core-rulebook` book. Tests derive the book tree from `books.json` and attribute an
entry to its first placement, which is what the reader does. Folding the duplicate
publication remains later work.

Two harness flakes on a library this size were fixed while making the suite pass: WebKit's
header-button tab walk uses Option-Tab as the other keyboard tests do, and the mobile
install-button size check rounds device-pixel layout values before comparing with 44 px.

This is local verification. Nothing here is deployed by that build, and PSRD was still
the production default at that time.

## Search scaling on the canonical library, 2026-11-05

Refs [#84](https://github.com/phynics/cast-iron/issues/84). The measurements below were
taken on content `vd7596846128e349b`: 29,515 entries and `search.bin` 65,725,435 bytes,
format 1 (unchanged). Merging current main first moved the canonical build to
`vbf0523074eb3de80` (29,880 entries, `search.bin` 67,372,004 bytes); the same engine path
measures p50 3.55 ms and p95 9.52 ms natively there, and the browser numbers below were
repeated on that build. The search view asks for every match because it groups duplicate
titles and reports the true total, so the engine materialized a highlighted title and a
snippet for each match. Each snippet tokenized up to 6 KB of body text, which made query
time scale with matches rather than with the rendered page.

Native measurement uses `cargo run --release -p cast-iron-search --example
bench_library --features profile -- generated/content/vd7596846128e349b/search.bin web`.
It loads the compiled index and runs the 50 queries from
`tests/e2e/measure-library.mjs`, three times each. Per-phase p50/p95 in milliseconds:

| Phase | Before (all hits materialized) | After (summary rank + 50-row page) |
|---|---:|---:|
| Tokenize | 0.00 / 0.00 | 0.00 / 0.00 |
| Posting lookup | 0.02 / 0.08 | 0.02 / 0.08 |
| Scoring | 1.26 / 3.42 | 1.45 / 4.18 |
| Sort | 0.04 / 0.19 | 0.05 / 0.27 |
| Result materialization | 0.32 / 1.92 | 0.00 / 0.00 |
| Snippet/title building | 50.91 / 199.66 | 1.21 / 3.38 |
| Total | 52.21 / 205.86 | 3.11 / 7.52 |

These figures come from a quiet host. "Before" is the bench's `full` mode, which builds a
snippet for every match as the old path did, with the new scoring code. The unmodified
engine at `404c530`, timed over the same 50 queries with no other work running, measures
p50 53.7 ms and p95 187.3 ms (max 264.5 ms). Snippet building was about 95% of the query.
The largest result sets are 4,054 matches at p95 and 7,665 at the maximum.

Browser acceptance on the same `dist/`, run locally:

| Measure | Chromium | WebKit |
|---|---:|---:|
| `Search operates entirely offline` p95 (20 queries) | 6.4 ms (max 9.8) | 13 ms (max 13) |
| `measure-library.mjs` engine p50 / p95 (50 queries) | 4.0 / 10.0 ms | 4.0 / 11.0 ms |
| `measure-library.mjs` input-to-results p50 / p95 | 131 / 185 ms | 127 / 162 ms |

Ranking is unchanged. `crates/search/tests/canonical_ranking.rs` compares the top 50 ids
for all 50 queries against a golden captured from the `origin/main` engine at `404c530`
on content `vbf0523074eb3de80`, and passes. A synthetic test compares bounded limits with
the full ranking and checks that summary materialization matches full hits.
`INDEX_FORMAT_VERSION` and the serialized layout are unchanged.

Local verification: `cargo xtask check` passes, and the full Playwright suite passes
114/114 (chromium 56, webkit 56, mobile-chromium 2) against the canonical `dist/` in
14.0 minutes. Nothing here is deployed, and the 250 ms smoke bound is unchanged.

After merging main again, the canonical build moved to `v3767562a585b5ce4`, so the pinned
golden skips. The baseline engine at `404c530` and the current engine were run over the
same `search.bin` and the same 50 queries: all 2,550 lines (top 50 ids and scores) are
identical. On that build `measure-library.mjs` reports engine p50 / p95 of 4 / 14.8 ms in
Chromium and 4 / 10 ms in WebKit, and the three search specs pass in both browsers. This is
local verification.

## Pf Data class conversion, 2026-11-05

Local results against the same pin after the class-conversion fixes:

| Measure | 2026-11-04 | 2026-11-05 |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 27,853 | 28,496 |
| Findings | 2,174 | 1,579 |
| High findings | 1,831 | 1,244 |
| Medium findings | 343 | 335 |
| Mandatory fixtures | 6/6 | 6/6 |

Every class and prestige-class description record now converts, including the
core classes and the derived class-archetype rules that previously skipped when
their host was missing. The remaining causes are non-class: 161 records without
name or publication metadata, 152 `::trait` metadata records, and the
`::room`, `::haunt`, `::trap`, `::drug`, and `::spelllist` families. The
`::prereq` and `::elephant` families are resolved. Representative mode still
reports 0 findings. Two full runs produced byte-identical reports and generated
output, and `cargo xtask content` remains byte-identical to `origin/main`.
Schema 3, search format 1, and production content are unchanged. This is local
evidence; the full migration remains blocked and unshipped.

## Canonical fallback carry-over and upgrade check (P8), 2026-11-05

The canonical build now carries a legacy entry when its planned canonical target
was not emitted, instead of retiring the alias row. On the pinned corpus 29 rows
take that fallback (elementals, ettercap, night hag, several traits and
archetypes, Magic, hazards, mythic feats/spells, special materials, spellbooks),
so `aliases.json` has 9,968 rows and zero retirements. The fallback count is one
coverage row, `alias-fallback-carry-over`; the carry-over report moves from
2,216 to 2,245 entries and 434 to 454 possible-duplicate review leads.

| Measure | Value |
|---|---:|
| Content version | `v3c4d6a1ad9e6838b` |
| Schema / search format | 3 / 1 |
| Entries | 29,417 |
| Total library bytes | 206,052,099 |
| `search.bin` | 64,746,739 |
| Shards / largest shard | 250 / `entries/creature-018.json`, 524,284 bytes |
| Books / publications (with entries) | 436 / 456 (394) |
| Relation edges | 4,170 (`part` 342, `archetype` 1,200, `prerequisite` 2,628) |
| Book placements / unpaged | 23,546 / 5,871 |
| Alias rows / retired | 9,968 / 0 |
| PSRD carry-over entries | 2,245 (29 from the fallback) |

Entries by kind: 8,011 items, 7,583 rules, 3,444 feats, 3,370 creatures,
3,099 spells, 1,639 traits, 1,240 archetypes, 342 afflictions, 342 racial
traits, 111 prestige classes, 103 races, 58 traps, 49 classes, and 26 skills.

`python3 scripts/check-library-upgrade.py generated/content generated/canonical`
now exits 0. It resolves all 9,968 PSRD ids in one step: 5,710 direct, 1,898
shared, 115 derived, and 2,245 carried; no id is missing, unresolvable, or
retired, no title differs under the alias fit gate, no class category is wrong,
and no relation, list, or book target dangles. The 11,937 repeated book
placements are the canonical multi-publication rule and are reported as a count.
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/scripts -v`
covers the alias and legacy paths.

This is a local build, not a deployment. PSRD was still the production default at
that time, and P8 acceptance (install size, quota, latency, offline navigation,
bookmark migration, browsers) had not been run for that library.
## Full Pf Data corpus second pass, 2026-11-05

A second local pass over pin `08ea71f24375908d0190c822fd1fd080797667a9` removed
the largest remaining conversion causes on the canonical main:

| Measure | Before | After |
|---|---:|---:|
| Selected records | 29,253 | 29,253 |
| Converted records | 28,496 | 29,024 |
| Findings | 1,579 | 858 |
| High findings | 1,244 | 523 |
| Medium findings | 335 | 335 |
| Excluded records | 1,489 | 961 |
| Mandatory fixtures | 6/6 | 6/6 |

The deliberate `::aff block has no name` rejection remains 332 medium findings.
The remaining high findings are smaller families, led by the inline `@list`
token, unresolved ability anchors such as `#/fetch`, and a few unsupported
containers. Two full `cargo xtask data-import` runs produced byte-identical
`full.json`, `full-search.bin`, `inventory.json`, `books.json`,
`book-coverage.json`, `migration-full.json`, and `migration-full.md`.
`cargo xtask data-report` reports 0 findings and 6/6 fixtures. Two canonical
`cargo xtask content` runs produced a byte-identical `generated/content` tree,
and `python3 scripts/check-library-upgrade.py generated/psrd generated/content`
exited 0 with an empty alias-problem and dangling-reference set.

The default canonical library moves from 29,515 entries at
`vd7596846128e349b` to 30,000 entries at `v46b91fe4b0c71e28`, a net gain of 485
entries. The PSRD carry-over library built with `cargo xtask content --psrd`
remains 9,968 entries at `vdb43853600b8ed8b`. Schema 3 and search format 1 are
unchanged. This is local evidence; the full migration is still blocked and not
shipped. See [decisions](decisions.md#full-pf-data-corpus-second-root-cause-pass-2026-11-05).

## Canonical cross-reference coverage and spell-list structure, 2026-11-05

The canonical library at `generated/content` is content version
`v8b314f5f8dcf7565`: 31,106 entries with schema 3 and search format 1. The
compiler reports 0 out-of-selection links and 0 broken links. The compiled tree
has 141,081 body link nodes in 22,297 entries (71.7%).

`scripts/measure-reference-links.py` is the rerunnable measurement for #20 and
#22-#26. It reads the compiled content tree, so it can be rerun against
`generated/psrd` with `--content generated/psrd`. Local numbers for the
canonical build:

| Category | Links to targets | Entries with such a link | Bare exact italic/bold spans | Pf | Carried | Self |
|---|---:|---:|---:|---:|---:|---:|
| Spells (spell bodies only, #20) | 1,583 | 668 | 398 | 236 | 162 | 44 |
| Feats (#24) | 18,307 | 8,944 | 262 | 235 | 27 | 25 |
| Special abilities (#25) | 5,878 | 3,862 | 935 | 663 | 272 | 19 |
| Conditions (#26) | 4,447 | 2,623 | 59 | 47 | 12 | 0 |

The largest spell residue is *protection from evil* (51), *magic circle* (41),
and *magic circle against evil* (21). The largest special-ability residue is
*spell resistance* (284), *spell-like abilities* (169), and *invisibility* (93).
These are exact-title hits, not confirmed references: *consumption* and
*renown* are mass-combat or class terms that share a feat title. Cloudkill
(`ref/spell/cloudkill`, a Pf Data entry) names *fog cloud* twice in plain text,
not italic, so an italic-only pass cannot satisfy that old acceptance example.

#42 is fixed in the compiler. All 3,135 spells now group their level row by
spell level (3,022 Pf and 113 carried); before the change all 3,022 Pf spells
and 113 carried spells were one flat string. 346 Pf creatures carry a
class/level spell list rendered with readable labels; 1,086 carry
frequency-only lists. The carried library adds 114 class/level spell lists
and 2 frequency-only lists. Raw `l0`..`l9`, `Cl:`, and `Con:` labels in
creature stat blocks fell from 1,432 Pf entries to 0.

Local verification: `cargo xtask check` passes; two `cargo xtask content` runs
produced the same file-hash manifest (`39cdced315f96d2de9ac12234c69f15c46b03dfc3cc1566a2908e0e2db31fb70`);
`check-publication.py generated/content` and
`check-library-upgrade.py generated/psrd generated/content` exit 0; the
Chromium `reference.spec.ts` suite passes (14 tests), including the new spell and
creature spell-list checks. This is local evidence only; no deployment is
claimed.
