---
title: "CI runs the canonical ranking comparison after the library build, and fails when the index is missing"
date: 2026-11-24
status: current
superseded_by: []
issues: [692]
prs: []
area: search
---

# CI runs the canonical ranking comparison after the library build, and fails when the index is missing, 2026-11-24

Closes [#692](https://github.com/phynics/cast-iron/issues/692). The ranking comparison of
[2026-11-24 canonical ranking golden](2026-11-24-canonical-ranking-golden-follows-the-pin.md) needs a generated
search index. `Rust checks` builds none, so the comparison used to run only on a developer machine. In CI it skipped silently.

- **Where it runs.** The `Content compiler` job runs `cargo test -p cast-iron-search --test canonical_ranking`
  right after `cargo xtask content`. That job already has the library, and the step needs no other job.
- **A missing index fails.** `CAST_IRON_REQUIRE_CANONICAL=1` is set on that step only. With it, the test panics
  when no index exists; without it, the developer-machine behaviour is unchanged and the test skips. The variable
  is not set in `Rust checks`, which still skips without an index.
- **Why a failure, not a skip.** A skip in CI reports green while comparing nothing, the same fault the golden
  repair in #680 removed for the pin check.
- **Cost.** The step compiles the search crate's test binary. The job's Rust cache keeps it between runs.
