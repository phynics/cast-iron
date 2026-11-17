---
title: "CI browser shards and early cache saves"
date: 2026-11-05
status: current
superseded_by: []
issues: [87]
prs: []
area: process
---

# CI browser shards and early cache saves, 2026-11-05

Refs [#87](https://github.com/phynics/cast-iron/issues/87). The first split (above) did not shorten the run: each browser suite stayed serial on one runner (`workers: 1`), and the WebKit browser cache was never saved.

- **Caches save before the tests run.** `actions/cache` saves in its post step only when the whole job succeeded, so a failing informational WebKit job never filled its cache. The Playwright browser, the Trunk binary and the pinned datasets now use `actions/cache/restore`, then `actions/cache/save` on a miss right after the download. Trunk's tools are saved by the `site` job directly after the build, because only that step downloads them. `Swatinem/rust-cache` runs with `cache-on-failure: true`. Not split: the npm cache (`actions/setup-node`), which the Chromium and report jobs save on success.
- **Each browser runs as three shards.** A matrix job runs `playwright test --shard=i/3`. Every shard has its own runner, test server and browser state, so the single worker assumption holds inside a shard. `workers`, `fullyParallel`, the tests and their thresholds are unchanged. Playwright assigns whole spec files by test count. From the per-file durations of run 42010548184, three Chromium shards (with `mobile-chromium` in the same invocation) take about 270, 295 and 195 s; four shards would only lower the longest to about 270 s, because `reference.spec.ts` alone takes about 150 s. So N is 3 for both browsers. Raise N only if one file split lowers the longest shard.
- **Every Chromium shard is required; every WebKit shard is informational.** Each WebKit shard has job-level `continue-on-error: true`, so a failing shard leaves the run `success`, which `scripts/promote-release.py` requires. Matrix jobs use `fail-fast: false` so one failing shard does not cancel the others.
- **Reports are merged per browser.** Each shard uploads a Playwright blob report (3-day retention). `report-chromium` and `report-webkit` run with `if: !cancelled()`, merge the blobs into one HTML report (`e2e-report-<browser>`, 14 days) through `.github/actions/merge-e2e-reports`, and write the counts and failed test titles to the run summary. Traces and screenshots of failed tests travel inside the blobs. The report jobs are informational and never decide the run result. Playwright shard artifacts share one browser cache key and one rust-cache key per purpose; a shard that loses the race to save a cache logs a warning and continues.
