---
title: "Drift between typed fields and body text is an error"
date: 2026-11-06
status: current
superseded_by: []
issues: [180]
prs: []
area: corpus
---

# Drift between typed fields and body text is an error, 2026-11-06

Refs [#180](https://github.com/phynics/cast-iron/issues/180). The checker is
`crates/corpus/src/drift.rs`; `corpus-check` runs it.

- **A disagreement fails the check.** `corpus-check` and `cargo xtask content` exit non-zero and
  list every disagreement, so a value cannot disagree in `[fields]` and the body without failing
  the build. The pinned corpus `v0.1.2` has 0 disagreements.
- **The report stays available.** `--drift-report FILE` still writes the count per kind and one
  line per disagreement, whether the check passes or fails. It no longer needs a warning to pass:
  drift was a warning only while the freeze data still held disagreements.
