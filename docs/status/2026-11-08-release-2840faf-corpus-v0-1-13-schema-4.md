---
title: "Release `2840faf` (corpus v0.1.13, schema 4)"
date: 2026-11-08
kind: release
---

# Release `2840faf` (corpus v0.1.13, schema 4), 2026-11-08

`release` was promoted from `1dacf59` to
`2840faf6800c9e03091396317d46d7a20683c176` with `scripts/promote-release.py --push`. The release
serves content `vde9dc21edbc50b07` (schema 4, 31,851 entries) with shell `03c8f6ec8bef277a` from
corpus pin `v0.1.13`. Schema 4 forces one library update for existing readers.

| Field | Value |
|---|---|
| Release commit | `2840faf6800c9e03091396317d46d7a20683c176` |
| Previous release | `1dacf59`, content `v2189dcc7f6c8b730` (schema 3), shell `9a77958fff99810f` |
| Content | `vde9dc21edbc50b07`, schema 4, 31,851 entries |
| Shell | `03c8f6ec8bef277a`, released 2026-11-08 |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.13`, commit `b86d92c` |

**Deployment.** Pages run
[42500599556](https://github.com/phynics/cast-iron/actions/runs/42500599556) succeeded (build and
deploy) for `headBranch: release` at `2840faf`. The live `content/manifest.json` reads
`content_version: vde9dc21edbc50b07`, `schema_version: 4`, `corpus_tag: v0.1.13`; the live
`sw.js` stamps `SHELL_DATE = "2026-11-08"`.

**CI.** Main full CI run
[42497922343](https://github.com/phynics/cast-iron/actions/runs/42497922343) succeeded at
`2840faf`, every job green, WebKit included.

**Local upgrade check, 2026-11-08,** Chromium and WebKit. From a profile on the previous release
(library `v2189dcc7f6c8b730`, schema 3) to this build (library `vde9dc21edbc50b07`, schema 4), on
the same origin and profile; 40 checks per browser:

| Check | Result |
|---|---|
| App update offered, applied; old library still readable under the new shell | PASS |
| Library update offered separately; old library serves entries during the download | PASS |
| Library activates; only the new content cache remains | PASS |
| Stars (feat, spell, creature), Recent, and two explicit book choices | Kept |
| `#/books/active` redirects to Manage books | PASS |
| Settings "This app": `v0.1.13 (vde9dc21e…)`, released date, content format 4 | PASS |
| Class options on Wizard, `Variant of Cure Light Wounds`, Mythic and Stamina gating | PASS |
| Offline reload: Home, entry, Quick search, results | PASS |
| Console errors, page errors, third-party requests | 0, 0, 0 |
| Spell density | No old setting to keep (added after the last release); a new choice persists |

**Live check, fresh profile, headless Chromium and WebKit,** against
https://phynics.github.io/cast-iron/ on 2026-11-08: the Download offer, library `v0.1.13
(vde9dc21e…)` with content format 4, Fireball online and after an offline reload, offline Quick
search and results (50 hits), 0 console errors, 0 third-party requests. PASS in both browsers.

**Published ids.** `cargo xtask published-ids` added 835 ids (`sources/published-ids.txt` now holds
41,909), the new add-on, class-option and template entries. No line was removed.

**Shipped since `1dacf59`** (42 first-parent merges, grouped):

- Attached entries (#246): [#260](https://github.com/phynics/cast-iron/pull/260) Schema 4, [#265](https://github.com/phynics/cast-iron/pull/265) mythic feats, [#282](https://github.com/phynics/cast-iron/pull/282) Stamina add-ons,
  [#277](https://github.com/phynics/cast-iron/pull/277) class options, [#270](https://github.com/phynics/cast-iron/pull/270) class-option alters, [#286](https://github.com/phynics/cast-iron/pull/286) spell variants and spell mythic
  sections.
- Books and settings: [#238](https://github.com/phynics/cast-iron/pull/238) In this book links, [#275](https://github.com/phynics/cast-iron/pull/275) closed groups build their rows on open,
  [#289](https://github.com/phynics/cast-iron/pull/289) Manage books, no book-page toggle, flat contents, [#291](https://github.com/phynics/cast-iron/pull/291) third-party marker and
  Paizo Everything without third-party books, [#261](https://github.com/phynics/cast-iron/pull/261) corpus tag and release date in Settings.
- Lists and spells: [#237](https://github.com/phynics/cast-iron/pull/237) creature type and CR menus, [#241](https://github.com/phynics/cast-iron/pull/241) CR scope wording, [#240](https://github.com/phynics/cast-iron/pull/240) spell
  row density, [#283](https://github.com/phynics/cast-iron/pull/283) level 0 as Cantrips, [#259](https://github.com/phynics/cast-iron/pull/259) discovery cache.
- Icons and polish: [#255](https://github.com/phynics/cast-iron/pull/255), [#267](https://github.com/phynics/cast-iron/pull/267), [#243](https://github.com/phynics/cast-iron/pull/243), [#268](https://github.com/phynics/cast-iron/pull/268), [#248](https://github.com/phynics/cast-iron/pull/248).
- Reliability: [#269](https://github.com/phynics/cast-iron/pull/269) WebKit fixes and recents shown after a visit, [#276](https://github.com/phynics/cast-iron/pull/276) serialised recents
  writes, [#244](https://github.com/phynics/cast-iron/pull/244), [#290](https://github.com/phynics/cast-iron/pull/290).
- Corpus pins: [#247](https://github.com/phynics/cast-iron/pull/247) v0.1.5, [#257](https://github.com/phynics/cast-iron/pull/257) v0.1.6, [#263](https://github.com/phynics/cast-iron/pull/263) v0.1.7, [#272](https://github.com/phynics/cast-iron/pull/272) v0.1.8, [#279](https://github.com/phynics/cast-iron/pull/279) v0.1.9,
  [#280](https://github.com/phynics/cast-iron/pull/280) v0.1.10, [#284](https://github.com/phynics/cast-iron/pull/284) v0.1.11, [#288](https://github.com/phynics/cast-iron/pull/288) v0.1.12, [#292](https://github.com/phynics/cast-iron/pull/292) v0.1.13.
- CI and corpus guard: [#239](https://github.com/phynics/cast-iron/pull/239) Playwright container, [#245](https://github.com/phynics/cast-iron/pull/245) weighted shards, [#274](https://github.com/phynics/cast-iron/pull/274) faster e2e
  setup, [#242](https://github.com/phynics/cast-iron/pull/242) upstream-token guard on typed fields.
- Docs: [#236](https://github.com/phynics/cast-iron/pull/236), [#253](https://github.com/phynics/cast-iron/pull/253), [#266](https://github.com/phynics/cast-iron/pull/266), [#293](https://github.com/phynics/cast-iron/pull/293).

**Not verified.** Physical devices and the accessibility check
([#12](https://github.com/phynics/cast-iron/issues/12)) are still not done. Firefox was not run
against the live site. No corpus marks a publication as third party yet, so the one-time
third-party notice (#189) has not run against real data.
