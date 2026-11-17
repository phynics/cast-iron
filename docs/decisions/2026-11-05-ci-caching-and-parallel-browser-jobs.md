---
title: "CI caching and parallel browser jobs"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: process
---

# CI caching and parallel browser jobs, 2026-11-05

- **One composite action sets up every build job.** `.github/actions/setup-build` installs the pinned Rust toolchain, configures `Swatinem/rust-cache`, and (with `trunk: "true"`) restores Trunk from `actions/cache`. CI and `pages.yml` use it, so the Trunk pin (version 0.22.1 and its SHA-256) now lives in one place. The binary cache key carries the version and the SHA, so a changed pin cannot reuse an old binary. The tarball is downloaded and verified only on a cache miss.
- **Trunk's tools are cached too.** Trunk downloads wasm-bindgen and wasm-opt from GitHub at build time into `~/.cache/trunk`. The key is the Trunk version plus the `wasm-bindgen` version in `Cargo.lock`. A warm run makes no download. A cold run retries `cargo xtask build-site` up to three times, because run 41999738126 failed with `failed downloading release archive`.
- **Rust cache keys are shared by purpose, not by job.** `check` (Rust checks), `content` (compiler runs), and `site` (the wasm and Trunk build). The CI `site` job and the Pages build use `site`, so a release build restores the dependencies that main's CI compiled (a cache saved on the default branch is readable from `release`).
- **The site is built once.** The `site` job builds `dist/`, runs the publication screen and the dist size summary, and uploads `dist` (1 day). `e2e-chromium` and `e2e-webkit` download `dist` and `generated` and run in parallel, so the critical path is the slower suite instead of their sum. Test code and thresholds are unchanged.
- **Playwright browsers are cached per browser.** The key is the `@playwright/test` version from `tests/e2e/package-lock.json` plus the browser name. `playwright install <browser>` runs only on a miss. `install-deps` always runs, because system libraries live outside the browser cache. `.github/actions/setup-e2e` holds these steps for both jobs.
- **WebKit stays informational by job-level `continue-on-error: true`.** A failed job with `continue-on-error` does not fail the workflow run, so the `ci.yml` push run on `main` that `scripts/promote-release.py` reads still concludes `success`. Chromium and mobile Chromium remain required.
- **Report artifacts are split.** `e2e-report-chromium` and `e2e-report-webkit` replace the single `e2e-report`; both keep 14-day retention. `dist-size.txt` moves to its own 14-day `dist-size` artifact from the site job.
- **Not cached: the Rust toolchain install.** It takes about 13 seconds, and restoring `~/.rustup` would cost nearly as much.
