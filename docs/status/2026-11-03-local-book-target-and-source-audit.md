---
title: "Local book target and source audit"
date: 2026-11-03
kind: record
---

# Local book target and source audit

Issue [#11](https://github.com/phynics/cast-iron/issues/11) tracks this work. The 283 AoN directory rows now contain 90 exact-title targets, 60 source-validated passage targets, two source-validated chapter contexts, 26 rows without a verified installed target, and 105 rows under 14 absent source headings. All 49 prior title ambiguities have target evidence; 12 prior unmatched labels refer to installed passages. These are target resolutions, not new library text. The two chapter cases preserve explicit opening-passage limitations.

The pinned Pf Data audit contains source-tagged records for all 14 absent headings. PCGen has matching source manifests for nine. A source-bounded passage probe found candidate records for 12 of the 14 remaining opening passages missing from the installed snapshot; it did not find the Bestiary or GameMastery Guide introductions. Exact source pins, count methods, JSON pointers, hashes, and directive inventories are committed as research evidence. The corpus is now 9,968 entries in 16 books. No new source is shipped by this audit, and no deployment is claimed.

Verification for this evidence change: `cargo xtask check` passes, including 23 Python audit/mapping fixtures and 95 web unit tests. Replayed passage/chapter evidence, applied mappings, and pinned source inventories compare byte-identically. Compiler inputs and client behavior are unchanged by the research files. Remaining corpus work stays open in [#11](https://github.com/phynics/cast-iron/issues/11), the parallel migration [#18](https://github.com/phynics/cast-iron/issues/18), and the bounded source adapter [#19](https://github.com/phynics/cast-iron/issues/19). A new corpus can invalidate the evidence hashes; regenerate against its actual entries/book index rather than weakening those checks.
