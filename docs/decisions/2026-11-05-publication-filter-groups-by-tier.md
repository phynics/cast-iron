---
title: "Publication filter groups by tier"
date: 2026-11-05
status: current
superseded_by: []
issues: [101, 109]
prs: []
area: ui
---

# Publication filter groups by tier, 2026-11-05

Refs [#101](https://github.com/phynics/cast-iron/issues/101), Phase B. It builds on the Books tiers above and the Publication filter of #109 (below).

- **The same `Tier`, no second mapping.** `publication_page` groups the visible publications with `Tier::from_category(SourceInfo.category)` and lists the groups in `Tier::ALL` order: Rulebooks, Player Companions, Campaign Setting, Adventures, Other. A group with nothing visible is not rendered. Inside a group the #109 order holds: selected first, then result count, then title.
- **The first-page limit is one budget, spent in group order.** `PUBLICATION_TOP` (12) still caps what shows until "Show all publications" or the name box. Selected publications with results spend it first; the rest are taken group by group, so a Rulebook with results is never hidden while an unselected lower-tier publication shows. A broad query with twelve or more Rulebooks therefore opens with Rulebooks only, and "Show all publications (N more)" reveals the lower tiers. Two rejected alternatives: a per-tier limit would hide a Rulebook while a Player Companion shows; exempting Adventures from the budget would put "Showing 12 of N" out of step with the DOM and render the whole Adventures group (over a hundred chips) on the first page.
- **The summary line keeps its meaning.** "Showing X of Y publications with results" counts every visible publication with results, including those inside a closed group, so it stays equal to what is in the page. A pinned selection at zero is not counted, so it never reads "1 of 0". `Adventures (N)` counts the publications with results that the group holds, which is fewer than the corpus total until "Show all" is on.
- **Adventures is a native `<details>`; selected filters are never hidden.** It starts closed (`Tier::collapsed_by_default`). A selected publication in it forces it open: the toggle handler reopens it. A non-empty name box opens it too, so matches are not behind a click; the reader's own open or close choice persists otherwise. The choice lives in a signal outside the keyed group list, so the group stays open after a result refresh, and a group that drops out and returns keeps the choice.
- **Roles and access.** Each group is a `role="group"` labelled by its title (`aria-labelledby`). The Adventures title is the text of the `<summary>`, which keeps a full 44 px target. Titles are plain muted text, not pills; the publication chips stay filter pills (`DESIGN.md`). The keyed lists keep every chip and group mounted across refreshes, so focus order and disposal behave as in #109.
