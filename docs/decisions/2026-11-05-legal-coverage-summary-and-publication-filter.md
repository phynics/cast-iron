---
title: "Legal coverage summary and publication filter"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: [64]
area: licensing
---

# Legal coverage summary and publication filter, 2026-11-05

- **The required notices stay in full; coverage opens on demand.** `#/legal` keeps the Community Use notice, the Product Identity declaration, the Open Game License text, every Section 15 line, the Section 15 notice-pending list, and both dataset pins visible and uncollapsed. The coverage report keeps its counts per category and per reason, and its full tables sit behind disclosures whose rows are built on first open and stay mounted after it. `coverage.json` still carries every row (AGENTS.md rule 6); nothing was removed from the compiler or the shipped report.
- **The exclusion summary groups the reason, not the path.** `reason_group` keeps the converter category (`converter coverage [reference-to-failed-record]`), the rights basis, and the reviewed notice-pending reason, and leaves unrelated reason text whole. On the canonical library 10,208 excluded paths and 958 rejected records become 28 and 4 summary rows, with the paths and records columns summing back to the report.
- **The Publication facet shows matches, not every book.** The first page lists at most 12 publications with a non-zero count, largest first. A selected publication stays visible at zero so it can be cleared, a name box searches the rest, and "Show all publications" reveals every publication with results; zero-count publications stay out of the DOM. The line above the chips counts only what the name box matches, so a pinned selection outside the box never makes it read "1 of 0". Counts still come from the conditional `FilterCounts` of PR #64, and the URL keeps only the selected `source=` ids.
- **The change is measured, not assumed.** A local measurement recorded the initial rendered text length, DOM nodes, and time from reload to the coverage summary. On the canonical library in this worktree, 1280 px Chromium went from 3,332,192 to 181,481 rendered characters, 86,316 to 3,304 DOM nodes, and 1,680 ms to 325 ms; WebKit went from 4,244 ms to 2,662 ms. At 390 px, Chromium went from 1,550 ms to 211 ms and WebKit from 2,449 ms to 1,000 ms. These are local numbers, not CI or deployment claims.
