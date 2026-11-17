---
title: "Release `6365dba` (corpus v0.1.17, schema 4, index format 2)"
date: 2026-11-09
kind: release
---

# Release `6365dba` (corpus v0.1.17, schema 4, index format 2), 2026-11-09

`release` was promoted from `2840faf` to
`6365dba9e73cf340e7584a86f7b96bdcd8e08f2c` with `scripts/promote-release.py --push`, a fast-forward
push. The release serves content `v17a954e5c0f1d11c` (schema 4, 31,807 entries) with shell
`5464362442e5568f` from corpus pin `v0.1.17`. The search index format moved from 1 to 2 (book
sections in search, #354). Existing installs keep browsing, but they need the library update before
search works again; Settings offers it.

| Field | Value |
|---|---|
| Release commit | `6365dba9e73cf340e7584a86f7b96bdcd8e08f2c` |
| Previous release | `2840faf`, content `vde9dc21edbc50b07` (schema 4, corpus v0.1.13), shell `03c8f6ec8bef277a` |
| Content | `v17a954e5c0f1d11c`, schema 4, index format 2, 31,807 entries |
| Shell | `5464362442e5568f`, released 2026-11-09 |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.17`, commit `b91db5f` |

**Deployment.** Pages run
[42683286107](https://github.com/phynics/cast-iron/actions/runs/42683286107) succeeded for
`headBranch: release` at `6365dba`. The live `content/manifest.json` reads
`content_version: v17a954e5c0f1d11c`, `schema_version: 4`, `corpus_tag: v0.1.17`. The live `sw.js`
stamps `SHELL_VERSION = "5464362442e5568f"` and `SHELL_DATE = "2026-11-09"`.

**CI.** Main full CI run
[42680753166](https://github.com/phynics/cast-iron/actions/runs/42680753166) succeeded at
`6365dba`, with every job green, WebKit included. A GitHub Actions outage from about 18:05 to 18:44
UTC delayed the release. The pull-request runs failed before any job started, then passed when they
were rerun.

**Local upgrade check, 2026-11-09,** in Chromium and WebKit. It ran from a profile on the previous
release (library `vde9dc21edbc50b07`, index format 1) to this build, on the same origin and
profile. Before the swap, the profile starred Power Attack, Magic Missile and two ids that this
corpus drops (`ref/tech-misc/silverdisk`, `ref/rule/hierophant`), visited Fireball, and turned
Ultimate Magic off.

| Check | Chromium | WebKit |
|---|---|---|
| App update offered and applied; the old library stays active and an entry opens under the new shell | PASS | PASS |
| Library update offered separately; the old library stays active during the download, then the new one activates | PASS | PASS |
| Stars kept; the dropped ids open Battery and Hierophant through aliases and stay starred | PASS | PASS |
| Ultimate Magic stays off; The Elephant in the Room is off after the third-party marker | PASS | PASS |
| The one-time third-party notice shows on Home | PASS | PASS |
| Search, book-section results (`.section-hit-link`), Core Rulebook printed chapters | PASS | PASS |
| Offline: an entry opens, search works | PASS | PASS |
| Console errors, page errors, third-party requests | 0, 0, 0 | 0, 1, 0 |

The first Chromium run found the missing third-party notice: profiles from `2840faf` stored an
empty remembered set. #369 fixed it before promotion, and the run above is on the fixed build. The
WebKit page error is `Context is stopped`, logged twice in a row right after Settings → Reload
applies the app update. Every later check passed. It is recorded on
[#121](https://github.com/phynics/cast-iron/issues/121).

**Live check, fresh profile, headless Chromium and WebKit,** against
https://phynics.github.io/cast-iron/ on 2026-11-09. Both browsers showed the Download offer,
installed library `v0.1.17 (v17a954e5…)` with content format 4, and opened Fireball online and after
an offline reload. Offline Quick search suggested Fireball for "fireb", and offline search showed
50 hits. There were 0 console errors and 0 third-party requests. PASS in both browsers.

**Published ids.** `cargo xtask published-ids` added 110 ids, so `sources/published-ids.txt` now
holds 42,019. No line was removed.

**Shipped since `2840faf`** (43 first-parent merges, grouped):

- Books: [#327](https://github.com/phynics/cast-iron/pull/327) dotted leaders, [#346](https://github.com/phynics/cast-iron/pull/346) chapter pages and previous/next, [#352](https://github.com/phynics/cast-iron/pull/352) Continue reading,
  [#351](https://github.com/phynics/cast-iron/pull/351) Adventures grouped by series, [#354](https://github.com/phynics/cast-iron/pull/354) book sections in search, [#367](https://github.com/phynics/cast-iron/pull/367) Game Mastery Guide book id alias.
- Third-party material: [#318](https://github.com/phynics/cast-iron/pull/318) `changes` add-ons, [#319](https://github.com/phynics/cast-iron/pull/319) and [#341](https://github.com/phynics/cast-iron/pull/341) revisions, [#369](https://github.com/phynics/cast-iron/pull/369) notice for existing
  profiles.
- Search and Home: [#368](https://github.com/phynics/cast-iron/pull/368) Recently viewed in Quick search, with alias resolution, [#355](https://github.com/phynics/cast-iron/pull/355) Report a problem.
- Corpus format and reports: [#329](https://github.com/phynics/cast-iron/pull/329) series fields, [#366](https://github.com/phynics/cast-iron/pull/366) reachability report.
- Corpus pins: [#314](https://github.com/phynics/cast-iron/pull/314) v0.1.14, [#333](https://github.com/phynics/cast-iron/pull/333) v0.1.15, [#342](https://github.com/phynics/cast-iron/pull/342) v0.1.16, [#365](https://github.com/phynics/cast-iron/pull/365) v0.1.17. Together they bring printed
  chapters and nested placement for 16 books, about 150 duplicates resolved through aliases, the
  race hubs and class-feature headings.
- Book-order evidence and placement scripts: [#295](https://github.com/phynics/cast-iron/pull/295), [#312](https://github.com/phynics/cast-iron/pull/312), [#313](https://github.com/phynics/cast-iron/pull/313), [#315](https://github.com/phynics/cast-iron/pull/315), [#316](https://github.com/phynics/cast-iron/pull/316), [#321](https://github.com/phynics/cast-iron/pull/321)–[#326](https://github.com/phynics/cast-iron/pull/326),
  [#328](https://github.com/phynics/cast-iron/pull/328), [#331](https://github.com/phynics/cast-iron/pull/331), [#334](https://github.com/phynics/cast-iron/pull/334)–[#336](https://github.com/phynics/cast-iron/pull/336), [#338](https://github.com/phynics/cast-iron/pull/338), [#339](https://github.com/phynics/cast-iron/pull/339), [#343](https://github.com/phynics/cast-iron/pull/343), [#344](https://github.com/phynics/cast-iron/pull/344), [#350](https://github.com/phynics/cast-iron/pull/350), [#353](https://github.com/phynics/cast-iron/pull/353), [#370](https://github.com/phynics/cast-iron/pull/370).
- Process: [#294](https://github.com/phynics/cast-iron/pull/294) previous release record, [#349](https://github.com/phynics/cast-iron/pull/349) test scope and the parallel determinism job.

**Not verified.** Physical devices and the accessibility check
([#12](https://github.com/phynics/cast-iron/issues/12)) are still not done. Firefox was not run
against the live site. The Game Mastery Guide book-id migration (#367) is shipped but inactive,
because v0.1.17 still has both book trees. It runs when a library without the retired tree arrives.
