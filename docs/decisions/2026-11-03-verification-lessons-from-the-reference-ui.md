---
title: "Verification lessons from the reference UI"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: process
---

# Verification lessons from the reference UI

- A site build reuses an existing generated manifest. Compiler, schema, and source changes therefore require explicit content compilation before building. Acceptance fixtures read `generated/`, while the test server serves `dist/`; mismatched versions invalidate the comparison. The verification skill records the sequence.
- Entry titles and headings recur in relationship panels, body text, and book navigation. Browser expectations select within the relevant region. Whole-list name filtering is substring matching, and collapsed book branches create children lazily. Tests target the relevant region, expand branches before inspecting children, and derive expected matches from the corpus.
- The relationship acceptance suite disables the network after installation and service-worker control. This verifies lazy index loading from the installed library as part of ordinary navigation. Local passes, CI results, deployment, and live-site checks remain separate evidence in `docs/status.md`.
