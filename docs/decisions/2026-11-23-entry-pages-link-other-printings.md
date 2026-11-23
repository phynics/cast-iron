---
title: "Entry pages link the other printings of the same rule"
date: 2026-11-23
status: current
superseded_by: []
issues: [650]
prs: []
area: ui
---

# Entry pages link the other printings of the same rule, 2026-11-23

Closes [#650](https://github.com/phynics/cast-iron/issues/650). It builds on `2026-11-23-list-folds-order-printings-like-search` and `2026-11-23-search-prefers-the-latest-printing`, without editing them.

- **The line.** A muted `Other printings:` line sits directly under the `Expanded in:` line (under `Sources:` when the entry has no credit). Each link names its book and opens that printing. There is no `Latest` label and no year.
- **The order.** Newest first, by `PrintingOrder` (`crates/web/src/views/book_tier.rs`): product line, then the newest printing, then book title, then id. `printing_cmp` in `crates/web/src/views/source_variants.rs` is the one comparison that the browse folds and this line both sort by. Two links that read alike take a last `(n)`, as in a fold.
- **The set.** The entries with the same kind, normalized title and parent. `fold_key` in `source_variants.rs` is the one definition: the lists and this line both call it, so they cannot disagree.
- **Excluded.**
  - Creatures: one name prints different stat blocks in different books (Acolyte CR 1/2 in the Game Mastery Guide, CR 1/3 in the NPC Codex).
  - A pair that a `revision` edge joins, in either direction: the revision notice already links it. The unfiltered relation graph decides, so the answer does not change with the books that are on.
  - A pair whose primary book is the same: an in-book duplicate (eleven groups, phynics/cast-iron-data#267). They stay out until the data side merges or aliases them.
- **Turned-off books.** A printing whose books are all off stays in the line, dimmed with `--ink-dim` and ending in the `Inactive` Label, as an inactive `Expanded in` credit does. The line never rests on dimming alone.
- **Finding the set at view time.** The page compares the normalized title of each entry of its kind with its own, without building a string (`same_normalized_title`; about 8,100 comparisons for the largest kind). Only when another entry shares the title does it read the parent source: `relations.json` (racial traits and archetypes take their parent from it, and every kind needs it for the revision edges) and `option-groups.json` for a class option. A page with no same-name entry reads nothing more than before. A library that lacks the needed file shows no line, because two rules that share a name could then not be told apart.
- **Wide screens.** At 64 rem and wider the line owns the grid row `printings` between `expanded` and `aon`.
- **No format change.** Nothing new is serialized: `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION` stay.
