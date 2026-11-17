---
title: "CI scope: Rust checks on pull requests, the full pipeline daily on main"
date: 2026-11-05
status: partly-superseded
superseded_by: ["2026-11-09-test-scope-affected-specs-locally-the-full-suite-once-in-ci", "2026-11-17-test-scope-owner-rules-of-2026-11-17"]
issues: []
prs: []
area: process
---

# CI scope: Rust checks on pull requests, the full pipeline daily on main (2026-11-05)

- **Pull requests run only `Rust checks`** (`cargo xtask check` and the release-promotion script tests). Content determinism, the site build, and the Chromium and WebKit browser shards are skipped there. Each pull request push used to start both a `push` and a `pull_request` run of the full pipeline, which kept the runners busy for tens of minutes per push.
- **The full pipeline runs on `main` once a day** (03:17 UTC) and on demand with `gh workflow run ci.yml --ref main`. Pushes, including merges to `main`, no longer start CI.
- **Refined 2026-11-09** ("Test scope: affected specs locally, the full suite once in CI"): a pull request that changes code now gets one full run on its branch before merge.
- **Consequence:** a content, build, or browser regression is found by the next daily run, not at merge. Run the affected checks locally before merging (`cargo xtask content`, `cargo xtask build-site`, and the affected e2e specs), as `AGENTS.md` already requires.
- **Release promotion still needs a green full run for the exact commit.** `scripts/promote-release.py` accepts any successful `ci.yml` run on `main` for that SHA (daily or dispatched), not only a push run.
