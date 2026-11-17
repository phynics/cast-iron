---
title: "CI browser jobs run in the Playwright container"
date: 2026-11-07
status: current
superseded_by: []
issues: [199]
prs: []
area: process
---

# CI browser jobs run in the Playwright container, 2026-11-07

Refs [#199](https://github.com/phynics/cast-iron/issues/199).

- **One image, pinned to the lock file.** The Chromium and WebKit shards run in
  `mcr.microsoft.com/playwright:v1.64.1-noble`. The image already contains the browser build
  and its system libraries, so `playwright install`, `playwright install-deps` and the
  `~/.cache/ms-playwright` cache are gone. The Rust job compares the tag in `ci.yml` with
  `@playwright/test` in `tests/e2e/package-lock.json`; a lock-file bump that leaves the tag
  behind fails there, on every pull request.
- **The action keeps only Node.** `.github/actions/setup-e2e` runs `actions/setup-node` (the
  npm cache is keyed on the lock file) and `npm ci`. Both work inside the container, which
  runs as root and provides `git`, `bash` and the browsers. The report jobs still run on
  `ubuntu-latest`, because merging blob reports needs no browser.
- **No apt step remains.** The install-deps stall recorded in #199 cannot recur, and the job
  ceilings added in #212 stay as the outer bound.
