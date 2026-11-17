---
title: "Adventures series UI"
date: 2026-11-09
status: current
superseded_by: []
issues: [159]
prs: []
area: ui
---

# Adventures series UI, 2026-11-09

Refs [#159](https://github.com/phynics/cast-iron/issues/159). The corpus fields from the entry
above are now shown in Books, Manage books and the search Publication filter.

- **One pure grouping.** `views/book_series.rs` owns grouping, ordering, title cleanup and the
  fallback. Books, Manage books and search read the same function, so a title or an order can
  never differ between the three screens. Native tests cover each kind and the no-series case.
- **Scenario numbers and issue numbers sort numerically.** A digit-run key reads `#3-01` and
  `3-1` as the same sequence, so a padded scenario number does not sort after `3-10`. Ties fall
  back to the title and then the publication id. An unparseable number keeps its place in the
  group and sorts before numbers rather than being excluded.
- **Titles lose the printed prefix.** Adventure Path volumes read `#1 Burnt Offerings` (the
  `Pathfinder #1:` prefix is replaced by `#1`), Modules lose `Pathfinder Module:`, and a Society
  scenario reads `#3-10 The Immortal Connundrum`. A title without the prefix still renders, so
  the fallback can never blank a row.
- **A recognized series kind decides the tier when the category is absent.** The pinned Society
  scenario has `series_kind = "society"` and no reviewed category, so `Tier::of` places it in
  Adventures. A reviewed category still wins over the series kind; the third-party marker still
  wins over both.
- **Every series section starts open inside the tier.** The tier itself stays closed by default
  like every non-Rulebooks group. Opening it shows the three series headings and the same
  volumes the flat list had, one level down; a reader folds a path or a season to shorten the
  list. No state is stored beyond the mounted page, so a session keeps whatever the reader
  opened or closed while the tier stays open.
- **The bulk control belongs to a series group, not a tier.** Manage books keeps the presets as
  its page-wide action and adds a 44 px `Turn all on` / `Turn all off` button per Adventure Path,
  `Modules` list and season. The write is planned inside `apply_active_plan`, so two quick taps
  land on the second answer, and a group change ends the preset's Undo like a single row does.
- **The search Publication filter stays flat.** Its Adventures disclosure groups the chips under
  AP name, `Modules`, season and `Other` labels, but the chips stay in one list under the one
  tier disclosure. A nested disclosure would hide chips from the filter's flat budget and its
  focus order; the labels alone give the grouping the issue asks for.
