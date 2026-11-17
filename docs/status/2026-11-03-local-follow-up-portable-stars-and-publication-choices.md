---
title: "Local follow-up: portable stars and publication choices"
date: 2026-11-03
kind: record
---

# Local follow-up: portable stars and publication choices

Note: manual star export/import described below was later removed in PR #28;
stars stay local to the browser profile (see `docs/decisions.md`). The
publication-choice grouping remains shipped behavior.

Home now offers manual star export/import through a bounded, versioned JSON archive. Import validates the complete file before writing and merges by permanent entry ID in one transaction. Existing stars win and repeated imports add no duplicates. This transfers files between independent browser profiles; it does not synchronize Safari and Home Screen storage automatically.

Search and category lists group repeated titles within an entry kind after active-book filtering. Publication choices retain separate IDs, source labels, rule content, and star state. Search limits groups rather than truncating publication choices, and reports the underlying entry count. Prestige requirements retain multiple named matches as explicit source choices. Two reviewed Combat navigation labels use consistent connector casing without changing canonical titles or rule bodies.

The AoN audit now includes nested typed headings, inline formatting, entry IDs, and anchors. Of 283 topic/subsection rows, 90 have one exact target, 49 have ambiguous targets, 39 lack a matching section in an installed source, and 105 belong to 14 source headings outside the installed corpus. These are mapping dispositions, not imported content. The complete missing-source list and regeneration command are in [mapping resolution](../research/aon-mapping-resolution.md). The corpus remains 9,967 entries in 16 books at `v171f0348e5717ee0`, schema 3 and search format 1.

Native verification passes `cargo xtask check`, including 99 web tests. Six extractor fixture tests pass, and two mapping regenerations compare byte-identically. The final distribution build has shell `1135a12922754f39`; its content manifest matches `generated/`.

The primary agent also used a manual browser walkthrough to open Core Rulebook > Equipment > Armor, read the rendered article, jump to Masterwork Armor, follow Armor Spikes, use Back and Forward, and step to Banded Mail and back. No console errors appeared. Automated offline acceptance visits all 24 Armor article contents stops and follows a typed cross-link with reading history. The article is the sampled scope; this does not claim every linked equipment entry was read.

A 52-capture batch covers phone and desktop widths in both themes, including open publication choices and star transfer. It reports no horizontal overflow, undersized alphabet/spell-level controls, or console errors. Representative PNGs were opened and inspected. The screenshots were kept outside the repository and are not committed.

Physical-iPhone VoiceOver and genuine offline standalone restart remain unverified. Only an iPhone 18 Pro iOS 27.2 simulator is connected. The device driver's settings support no VoiceOver control, and its Wi-Fi/airplane switches only override the simulator status bar. The verify skill contains the remaining physical-device steps. PR #6 is merged; this follow-up is not deployed, and its CI result is not yet verified.

The final browser matrix passed 73 of 75 cases. The two WebKit failures were test setup issues: the keyboard check expected Tab to focus a button, while this WebKit environment requires Option-Tab; and the transfer test navigated twice before installation, aborting the initial requests with `pageerror: Load failed`. The keyboard failure reported `Expected: focused; Received: inactive`. The corrected two-test WebKit rerun passes. All 75 distinct cases are therefore verified across Chromium, WebKit, and the mobile Chromium layout project. The entire matrix was not repeated after these test-only corrections. No browser console-error allowance was added.

Earlier exploratory failures caught stale assertions that counted nested source-choice titles, attempted to open a closed race choice before incremental list rendering, and assumed Special Attacks supplied typed cross-links. Tests now wait for actual rendered choices, follow the disclosure, count top-level result groups, and use the linked Armor article for the combined reading journey. The corrected Chromium cases pass. The final native/build verification and the 52-capture batch use the final application code.

### Independent review follow-up

An independent review of head `2982678` reported no P1/P2 findings and one P3 archive-size bug, tracked in [#17](https://github.com/phynics/cast-iron/issues/17). A compact 1,991,340-byte archive could import but exceed the export limit after pretty-printing. Export now uses compact JSON, and import checks canonical serialized size, including numeric normalization, before any write. Two native boundary tests cover the reproduced case and a compact integer-date input whose normalized encoding exceeds the limit. `cargo xtask check` passes with 101 web tests. The rebuilt distribution has shell `9104b7a769594cbf`; the unchanged content manifest matches generated content. Targeted Chromium and WebKit star-transfer acceptance both pass. This follow-up is locally verified; no new deployed result is claimed.
