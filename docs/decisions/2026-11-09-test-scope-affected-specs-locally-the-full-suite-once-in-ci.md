---
title: "Test scope: affected specs locally, the full suite once in CI"
date: 2026-11-09
status: partly-superseded
superseded_by: ["2026-11-17-test-scope-owner-rules-of-2026-11-17"]
issues: [348]
prs: []
area: process
---

# Test scope: affected specs locally, the full suite once in CI, 2026-11-09

Refs [#348](https://github.com/phynics/cast-iron/issues/348). The rule is the "Test scope" table in `AGENTS.md`.

- **The full suite ran more than once per pull request.** Implementers ran the full Chromium and WebKit suite locally, as the `verify` skill required. A full `workflow_dispatch` CI run then ran it again on the branch. The two delegate runs measured on 2026-11-09 (#161 and #159) spent about 2 hours on local e2e between them: full Chromium took 12–22 minutes, full Chromium and WebKit took 45–60 minutes, and the load of parallel delegates made tests time out, which caused reruns. The checks that found real problems took 5–10 minutes: `cargo xtask check`, the site build, and the changed specs.
- **Locally, run only the specs that the change can break.** These are the changed spec files and the specs of the feature-map row in the `verify` skill, on Chromium. Add mobile-chromium when the spec is in that project. Add WebKit on those specs only for service-worker, installer, storage, or offline changes.
- **CI owns the full suite.** A pull request that touches code gets one full `ci.yml` run on its branch after the review fixes. Its Chromium shards are required and its WebKit shards are informational. A pull request that changes only docs or research scripts (`scripts/*-nested.py`, `docs/research/`) gets no full run: no build or browser test reads those files. The daily run on `main` stays.
- **Determinism no longer blocks the site build.** The `content` job compiles once and uploads `generated`. A separate `determinism` job compiles again and compares the result with that artifact (`diff -r`), alongside the site build and the browser shards. In run 42614828911 the second compile took 137 s on the critical path.
