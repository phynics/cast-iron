---
title: "Public POC rollout"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: releases
---

# Public POC rollout, 2026-11-03

- The public POC uses the OGL for the selected Core Rulebook and Bestiary rules, with Paizo's Community Use Policy for permitted descriptive references. The policy review and its limits are in `docs/research/licensing-audit.md`. This does not authorize artwork, campaign books, or additional sources.
- GitHub Issues is the maintained project contact shown on the legal page and in the README. It provides a public contact route without publishing personal contact details. External navigation occurs only when the user follows the link.
- `python3 scripts/check-publication.py` screens the built library before upload in CI and Pages. It checks hashes, source scope, attribution, OGL references, and a finite list of named setting material. Its generated report records the limits. It is evidence for the selected POC, not a legal classifier or a substitute for reviewing a new source.
