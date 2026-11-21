---
title: "Prestige classes: a Requirements stop in the section switcher"
date: 2026-11-21
status: current
superseded_by: []
issues: [170, 443]
prs: []
area: ui
---

# Prestige classes: a Requirements stop in the section switcher, 2026-11-21

Refs [#170](https://github.com/phynics/cast-iron/issues/170), the "View council follow-up", and [#443](https://github.com/phynics/cast-iron/issues/443). It refines `2026-11-17-class-pages-section-switcher`, which left prestige classes without a switcher because two stops (Overview, Features) were too few.

- **The gap, measured on the rendered pages** (current `main`, corpus v0.1.36, all 115 prestige classes, 390 × 844 and 1280 × 900): every prestige class has a `Requirements` heading and none had a switcher. At 390 px the heading starts at 455 px on the page where it is nearest, and 1,139 px is the median across all 115 pages. The three farthest pages (Bellflower Tiller, Death Slayer, Golden Legionnaire) reach 2,009 px. It is below the first screen on 87 of the 115 pages. At 1280 px it is below the first screen for 42. Arcane Archer has the heading at 1,427 px at 390 px and 958 px at 1280 px, with no contents disclosure. Only four prestige classes (Crimson Templar, Feysworn, Mystery Cultist, Prophet of Kalistrade) had four headings or more, so only they had `On this page`.
- **A fifth stop, `Requirements`, between Overview and Features.** It starts at the top-level heading titled exactly `Requirements`, found by the same rule as the other stops (the shallowest heading level, so an archetype's own deeper `Requirements` is not a stop). A prestige class now has Overview, Requirements and Features: three stops, which meets the existing minimum, so it gets the existing bar with no new component. The stops are listed in a fixed order: Overview, Requirements, Features, Archetypes, Spells. That is the order of the page in the current corpus; the code does not sort the stops by heading position. No base, hybrid or NPC class has a top-level `Requirements` heading in the pinned corpus (v0.1.36), so their bars do not change. The code does not check the class kind: a corpus change that added such a heading would add the stop, and the e2e stop lists for Wizard, Fighter and Arcanist would catch it.
- **Same behaviour as every other stop.** `?s=requirements` is a query, not part of the entry id (rule 3); a stop is a plain link and a history entry; Back is the browser's; the bar is sticky under the masthead; the heading takes focus. A prestige class without a `Requirements` heading keeps no switcher.
- **The contents disclosure stays on a prestige class.** On the pages that had `On this page`, the bar does not replace it (the Wizard decision removed it, because the bar replaced a 40-heading list). A prestige class's disclosure reaches one feature and costs one line, and the council follow-up asked to keep the existing contents controls.
- **Nothing is cut.** The full introduction stays in the page, ahead of the Requirements; no heading, anchor or entry id changes, and `?a=` links keep working. The compiler output, `SCHEMA_VERSION` and the stored data do not change.
