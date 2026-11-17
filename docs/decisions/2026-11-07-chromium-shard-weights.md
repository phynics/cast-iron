---
title: "Chromium shard weights"
date: 2026-11-07
status: current
superseded_by: []
issues: [97]
prs: []
area: process
---

# Chromium shard weights, 2026-11-07

Refs [#97](https://github.com/phynics/cast-iron/issues/97).

- **Equal weights left shard 3 about a third above the average.** In run 42377506682 the
  Chromium test steps took 3m26s, 4m21s and 6m10s. Shard 3 carried `update.spec.ts` (whose
  content-update test takes about 51 s), the mobile project and the slower half of
  `search.spec.ts`. Playwright shards by test group and ignores duration.
- **Weights move cheap tests to shard 1.** Playwright 1.64 reads
  `PWTEST_SHARD_WEIGHTS` as colon-separated weights, one per shard. The Chromium job sets
  `6:4:3`, which puts 96, 47 and 41 tests in shards 1, 2 and 3 instead of 67, 63 and 54.
  The measured durations are in the pull request that added the weights. WebKit keeps equal
  weights: it is informational and its shards have a different mix.
