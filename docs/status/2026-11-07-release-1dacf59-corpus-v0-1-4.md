---
title: "Release `1dacf59` (corpus v0.1.4)"
date: 2026-11-07
kind: release
---

# Release `1dacf59` (corpus v0.1.4), 2026-11-07

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
