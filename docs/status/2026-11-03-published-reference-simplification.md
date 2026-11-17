---
title: "Published reference simplification"
date: 2026-11-03
kind: record
---

# Published reference simplification

The local implementation separates Browse categories from Books, removes the duplicate mode picker and Rules category, and keeps rules accessible through Books, search, saved links, and starred entries. Home lists every star with an independent Unstar control. Collection and bookmark-manager UI is retired while legacy stored stars remain readable. Old manager URLs lead to Home, and the former Rules-category URL leads to Books.

Books has persistent Active controls. Disabling filters ordinary discovery, counts, search candidates, relationships, and spell lists, while preserving the download and canonical reading access. Corrupt book preferences offer Retry or Reset active books without discarding the library or stars. Direct disabled-book entries show a notice. Book placement determines visibility; search publication filters still describe attribution sources.

Primary rule text precedes related navigation. Large archetype lists start collapsed. Rule search links to matching typed headings, article contents supports repeated jumps to the current section, and exact named sections rank ahead of longer item-title prefixes. Prestige requirements link unambiguous named feats and skills. Source badges remain links, and source-grouped References avoids duplicate racial-trait and archetype targets. Installer copy explains the optional Home Screen installation before the library download and the observed possibility of separate Safari data.

The installed corpus remains 9,967 entries at `v171f0348e5717ee0`, schema 3 and search format 1. No source content was imported. The AoN inventory contains 23 source headings, 50 topics, and 233 subsections. Strict within-book title mapping yields 95 unique section matches, 16 ambiguous matches, and 172 unmapped rows. These are mapping evidence, not printed-order verification or complete AoN coverage.

Local verification: `cargo xtask check` passes, including 21 search and 94 web unit tests. The real `dist/` build succeeds and its content manifest matches `generated/`. Six targeted Chromium checks pass for the integration fixes and recovery paths. The full browser run passed 66 cases and found one WebKit fixture failure when setup navigated away during its initial manifest request. Setup now waits for the Download offer; that WebKit case passed on rerun. All 67 distinct cases are verified across Chromium, WebKit, and mobile Chromium. The complete suite was not repeated after the fixture-only change.

Two bounded visual batches covered 390 x 844 and 1280 x 900 in both themes. The confirmation captures reported zero horizontal overflow, no undersized alphabet or spell-level controls, and no console errors. Separate settled search captures confirm exactly one selected scope and rule-only results. The search-only capture waits for both the selected scope and rule-only result kinds, avoiding the stale-state screenshot that appeared during capture setup. Selected-scope foreground/background contrast measured 6.02:1 in light and 6.54:1 in dark. Representative PNGs were opened and inspected. The rerunnable capture command is in the verify skill.

The first browser pass caught blocked inline article-contents indentation and stale test assumptions about star ordering, labels, and open disclosures. Those were corrected. A later affected test exposed repeated section links failing to scroll when the URL already matched; explicit scrolling fixes that case. The corrected six-test run passed. The final full run passed the CSP and repeated-heading cases in both engines. An independent bounded integration review found no concrete P1 or P2 issue.

This simplification merged in [PR #6](https://github.com/phynics/cast-iron/pull/6) at `d48eaa3`. Its successful main CI and Pages deployment are linked at the top of this page. This migration did not repeat live browser acceptance for that artifact. Physical iPhone offline relaunch, VoiceOver, and a full physical-device character-building walkthrough remain unverified in [#12](https://github.com/phynics/cast-iron/issues/12). Earlier published simulator observations and automated offline checks remain separate evidence.
