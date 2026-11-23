---
title: "Chromium CI runs in four shards with weights 3:3:1:2"
date: 2026-11-23
status: current
superseded_by: []
issues: [633]
prs: []
area: process
---

# Chromium CI runs in four shards with weights 3:3:1:2, 2026-11-23

Refs [#633](https://github.com/phynics/cast-iron/issues/633). This entry replaces
`2026-11-07-chromium-shard-weights` and the Chromium shard count of
`2026-11-05-ci-browser-shards-and-early-cache-saves`.

- **Problem.** The required Chromium jobs ran three shards with `PWTEST_SHARD_WEIGHTS: "6:4:3"` under
  `timeout-minutes: 20`. On the green full run 45139208462 (`cf59ff602`) the jobs took 12.1, 15.0 and
  8.5 minutes. A slow runner adds about 40 %, which puts shard 2 at the limit; a cancelled shard makes
  the whole run `cancelled`.
- **Measurement.** The per-test durations of the three shard logs of that run add up to 32.0 minutes
  over 587 tests that run and 6 that are skipped. Playwright reported 11.3, 14.2 and 7.6 minutes of
  test time, about 4 % more than the sums; a job adds about 0.85 minutes of setup. The tests fall into
  four blocks, in the order in which Playwright shards them:

  | Block | Tests | Minutes | Seconds per test |
  |---|---|---|---|
  | `chromium`, specs on the shared installed library (`installedTest`) | 400 | 16.50 | 2.5 |
  | `chromium`, specs that install the library in each test (`test` of `fixtures.ts`) | 49 | 8.28 | 10.1 |
  | `chromium`, `teardown-window.spec.ts` (no browser) | 8 | 0.00 | 0.0 |
  | `mobile-chromium` | 130 | 7.23 | 3.3 |

  The longest spec groups are `search.spec.ts` (50 tests, 132 s), `update.spec.ts` (7 tests, 117 s),
  `install-button.spec.ts` (10 tests, 89 s) and, in the mobile project, the install-per-test part of
  `recovery-copy.spec.ts` (10 tests, 88 s).
- **How Playwright 1.64.1 places tests.** It builds test groups per project in configuration order;
  inside a project it orders them by worker fixture set, in the order of first use, and then by file.
  A file that uses both `installedTest` and `test` is two groups. Each shard gets
  `floor(weight × tests / sum of weights)` tests, and the remainder goes one test each to the first
  shards. A group goes whole to the shard that holds the position of its first test; the count leaves
  out tests that are skipped statically. A shard runs its own groups in the same rule's order, so a
  shard log does not show the global order. A simulation of this rule gives the 277, 186 and 130 tests
  that the three shards of run 45139208462 ran.
- **Four shards, weights `3:3:1:2`.** The sizes are 196, 196, 65 and 130 tests of 587. Shard 1 runs
  the installed-library specs from `accessibility-reading` to `class-sections`, shard 2 the ones from
  `continue-reading` to `stamina`, shard 3 the last three of them, the install-per-test block and
  `teardown-window`, shard 4 the mobile project.

  | Weights | Shard 1 | Shard 2 | Shard 3 | Shard 4 | Longest |
  |---|---|---|---|---|---|
  | `6:4:3` (three shards, measured) | 10.94 | 13.84 | 7.23 | | 13.84 |
  | `1:1:1:1` | 5.48 | 6.37 | 10.78 | 9.37 | 10.78 |
  | `3:3:1:2` (chosen) | 8.33 | 7.91 | 8.54 | 7.23 | 8.54 |
  | `8:9:2:6` | 8.33 | 8.16 | 8.28 | 7.23 | 8.33 |

  The values are sums of test durations in minutes. Expect about 4 % more as Playwright's reported
  time and about 0.85 minutes more as job time: 9.5, 9.1, 9.7 and 8.4 minutes for `3:3:1:2`. No weights
  up to 12 give a longest shard below 8.33 minutes. `8:9:2:6` reaches it, but its third boundary lies
  exactly at the first test of the install-per-test block, so one test fewer before it moves 54 s into
  shard 2. With `3:3:1:2` the first boundary lies inside `class-sections.spec.ts`, 10 tests after its
  first test and 19 before the next group, and the second lies between groups of 9 s or less.
- **What follows the shard count.** The job name is `Chromium shard N/4 (required)`.
  `scripts/merge-pr.py` lists the four names in `REQUIRED_JOBS`, so it refuses a full run that has
  three shards. The report job merges every `blob-report-chromium-*` artifact and needs no change. The
  ruleset of `main` requires only `Rust checks`. `timeout-minutes` stays 20; it only cuts a hang.
- **When to measure again.** The weights are shares of the test count. Measure the shard logs again
  when a block's share of the tests changes much, for example when install-per-test specs move to
  `installedTest` or the mobile project gains specs.
