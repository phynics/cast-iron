---
title: "Test scope: owner rules of 2026-11-17"
date: 2026-11-17
status: current
superseded_by: []
issues: [462]
prs: []
area: process
---

# Test scope: owner rules of 2026-11-17

Refs [#462](https://github.com/phynics/cast-iron/issues/462). The owner moved most browser testing from
each branch to a batch run on `main`. The rule is the "Test scope" section of `AGENTS.md`.

- **Locally, run `cargo xtask check`.** Add `cargo xtask content` when the compiler, the schema or the
  corpus pin changed, and at most the one new or changed spec. Do not run the full browser suite locally,
  and do not run Playwright for a change to docs or xtask only.
- **A reviewed pull request merges on the required `Rust checks`.** A reviewer reads the diff and the code
  and cites the CI result; it does not run builds or tests. The merge helper (`scripts/merge-pr.py`)
  reads which pull requests still need a full run, as `CONTRIBUTING.md` says.
- **A full CI run on `main` follows each batch of merges.** It replaces the full run per pull request.
  A regression that only the browser suite finds is found there, on the merged result, not on each branch.
- **Some changes still get a full run on the branch before merge:** CI configuration, the toolchain and
  Cargo files, install, storage and offline code (the service worker, `install.rs`, `content_cache.rs`,
  `store.rs`), and a change to `SCHEMA_VERSION` or `INDEX_FORMAT_VERSION`. A fault there can stop the
  app from opening an installed library, and a later run on `main` finds it too late to undo.
- **Promotion needs a green full run on the exact `main` SHA.** `docs/releasing.md` and
  `scripts/promote-release.py` already require it; nothing about promotion changes.

Reason: the 2026-11-09 rule ran the full suite once on every code branch, and several pull requests that
touched the same files each needed another run after every merge of `main`. One run per batch costs less
and tests the code that ships.
