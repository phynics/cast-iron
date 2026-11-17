---
title: "Capture evidence: manifests in the repo, images on the issue, no full-page shots while scrolled"
date: 2026-11-17
status: current
superseded_by: []
issues: [443, 448]
prs: []
area: process
---

# Capture evidence: manifests in the repo, images on the issue, no full-page shots while scrolled, 2026-11-17

Refs [#448](https://github.com/phynics/cast-iron/issues/448). The numbers and the diagnosis are in [the status record](../status/2026-11-17-conditional-view-captures.md).

- **Manifests in the repository, images outside it.** The repository holds no screenshots (`AGENTS.md`: no artwork, and PNG sets bloat history). A capture set is kept as a JSON manifest under `docs/research/capture-evidence/` and the PNGs go into a comment on the issue that asked for them. The manifest carries, for every capture, the commit, the shell and content version, the corpus tag, the engine and its version, the viewport, the theme, the settled state and the masthead probe. A manifest without the images still says what the images were.
- **Engine simulation is labelled.** A capture taken in WebKit with an iPhone user agent has `evidence_class: "engine-simulation"` and a `simulation` text. It never counts as device evidence; that stays with [#423](https://github.com/phynics/cast-iron/issues/423).
- **Viewport shots by default; the frame is read back.** The header is sticky, so a full-page shot of a scrolled page has no masthead at its top (it sits at the scroll offset, or at an unstable row under mobile emulation), while the DOM still reports y=0. Capture tooling takes viewport shots, scrolls to the top before a full-page shot, and checks the saved PNG for the masthead. A DOM measurement of the header is not evidence that the frame shows it.
- **Conditional states come from test-server hooks and init scripts.** The storage failure, a pending shell and a pending library are built with `/__test/*` hooks and an init script, so a capture never needs a change to the app.
