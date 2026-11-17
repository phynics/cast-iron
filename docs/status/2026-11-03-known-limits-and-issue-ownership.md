---
title: "Known limits and issue ownership"
date: 2026-11-03
kind: record
---

# Known limits and issue ownership

- Historical baseline, addressed by the reference simplification below: the iPhone Simulator follow-up found separate Safari and Home Screen library and bookmark stores. Installation guidance does not explain that transition. Long-rule search results also open the article start rather than the matched heading. See the simulator report for scope and other UX findings.
- Licensing review is scoped to the 16 selected books. The publication screen covers all 9,967 entries and found no issue, but does not classify every sentence or establish blanket rights to new sources. See `docs/research/licensing-audit.md`.
- Paizo Community Use Policy: on 2026-11-03 the official policy, FAQ, reinstatement post, and licensing overview were fetched and compared with the built Legal page. The notice matches the policy text, and the page shows a contact link. No registration is required. This does not clear every sentence of the dataset, and Paizo can withdraw the permission at its discretion. Details are in the audit.
- [#13](https://github.com/phynics/cast-iron/issues/13): Book-order fidelity is incomplete. Core Rulebook main chapters are verified; other outlines, nested placement, and appendix grouping need publisher evidence and mapping work. Zero unplaced entries proves coverage, not printed order.
- [#14](https://github.com/phynics/cast-iron/issues/14): Advanced Race Guide standard racial traits remain embedded in race bodies rather than standalone records. Alternate-trait replacement links are not modeled.
- [#14](https://github.com/phynics/cast-iron/issues/14): Eight spell-list names lack a class entry, as listed above. The lists cover installed spells; they do not establish complete coverage of those classes or their books. Feat chains contain extracted named links, including alternatives, rather than complete eligibility logic.
- [#43](https://github.com/phynics/cast-iron/issues/43), [#116](https://github.com/phynics/cast-iron/issues/116): The "Referenced by" panel shipped in #106; the remaining gaps are tracked here. Typed relations have inverse lookups in the entry screens, and the panel covers body links, but old libraries and the listed follow-ups are still open.
- [#15](https://github.com/phynics/cast-iron/issues/15): Category lens counts and source counts are global. Search facet chips carry counts conditional on the other active filters; those counts are not used by the category lenses.
- [#15](https://github.com/phynics/cast-iron/issues/15): In spell and creature `spells` stat lines, upstream lists become lines with breaks, so list structure is lost.
- [#15](https://github.com/phynics/cast-iron/issues/15): Heading ids are not prefixed, so a content anchor could collide with an app element id such as `main`.
- Firefox, and a single flaky run of `Back/forward navigation works` on Chromium, which could not be reproduced (see `docs/decisions.md`).
