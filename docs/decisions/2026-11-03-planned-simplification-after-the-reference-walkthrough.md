---
title: "Planned simplification after the reference walkthrough"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: ui
---

# Planned simplification after the reference walkthrough, 2026-11-03

These are accepted product directions awaiting implementation in [the UI task list](../ui-tasks.md). They supersede earlier decisions to retain the bookmark and collection manager. They do not describe shipped behavior.

- Browse and Books remain separate primary tabs. Remove the shared browsing-mode picker. Category discovery belongs in Browse. Rules belong in the book directory and search, with existing entry links and starred rules still readable.
- Use the AoN Rules source directory as the baseline for book, topic, and subsection discovery. This is a navigation and mapping baseline, not evidence of printed chapter order or permission to import new content. Preserve the user's original-order requirement and current evidence warnings.
- Remove collections and the separate bookmark manager. Home lists starred entries and lets the user open or unstar them. Preserve existing stars when retiring collection UI and routes.
- Add book activation controls. The proposed default is a persistent filter over downloaded content rather than deletion. Detailed filtering and saved-entry behavior are documented as working defaults in UI-03, not additional confirmed user requirements.
