---
title: "Rule target evidence and new-source preparation"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Rule target evidence and new-source preparation

- Exact titles alone do not identify a reading target when a chapter container, introduction, and complete article share a name. The AoN audit now compares source-specific opening passages and explicitly selects the dedicated article where appropriate. Identical paragraph positions collapse root/leading-heading aliases; unresolved ties stay explicit.
- Whole-chapter context uses a separate evidence method and retains opening-text gaps. The two Core Rulebook cases match the named canonical chapter and many descendant headings. This does not establish complete prose or alter printed order.
- Evidence binds to hashes of the typed entry files and book index. Applying stale evidence fails rather than silently guessing a new target. Fetched article HTML stays outside the repository; committed evidence contains hashes, metadata, and selected IDs/anchors.
- Missing-source research counts actual source-tagged records and source manifests at exact pins. It does not equate a book title or character-option catalog with complete book prose. A native Pf Data adapter should start with the missing Magic Basics passage before a new-book import.

- Star export/import was removed. Stars stay local to the browser profile; no archive file format is offered. This supersedes the manual star transfer decision above.
