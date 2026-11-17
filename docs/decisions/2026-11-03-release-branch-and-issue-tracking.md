---
title: "Release branch and issue tracking"
date: 2026-11-03
status: current
superseded_by: []
issues: [10]
prs: [7]
area: releases
---

# Release branch and issue tracking, 2026-11-03

The app is drawing positive reactions, and the user requested stricter
development hygiene. Separate integration from publication and track outstanding
work in GitHub Issues. Main remains the integration branch. A dedicated
`release` branch advances only through explicit, reviewed promotion of tested
main commits. It starts at the last successful published main SHA, rather than
the unrelated UI work in draft PR #7. Normal fast-forward pushes preserve shared
history and let a later reader inspect the exact release range.

Pages already uses Actions artifacts, so changing its legacy source-folder field
is not the migration. Restrict the `github-pages` environment to the branch
`release`, and guard both workflow jobs against other refs, including manual
dispatch. Preserve the project path, permissions, deployment serialization, and
existing CI/browser checks. The environment rule blocks main publication before
the workflow PR merges. No additional approval gate is needed.

GitHub Issues owns new development and remaining acceptance work. Docs retain
observations, corpus limits, decisions, and historical results with canonical
issue links. Completed local UI tasks are history, not a new backlog. Physical
device and printed-order gaps stay open until their specific evidence exists.
See [contributing](../../CONTRIBUTING.md), [release operations](../releasing.md), and
[migration issue #10](https://github.com/phynics/cast-iron/issues/10).
