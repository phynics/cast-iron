---
title: "Books group by priority tier"
date: 2026-11-05
status: current
superseded_by: []
issues: [101]
prs: []
area: ui
---

# Books group by priority tier, 2026-11-05

Refs [#101](https://github.com/phynics/cast-iron/issues/101), Phase A. The owner's priorities: core rulebooks matter most; Adventure Paths, Modules and Society matter least. Every book starts active, and one action deactivates a whole group.

- **One mapping, built on `SourceInfo.category`.** `crates/web/src/views/book_tier.rs` holds a `Tier` enum and one table from the reviewed category (#107) to a tier, with no new field and no second categorization. Order: Rulebooks (`RPG`), Player Companions (`Player Companion`), Campaign Setting, Adventures (`Adventure Path`, `Module`, `Society`), Other (`Miscellaneous`, any unrecognized value, and no category). A native test reads `sources/canonical/publication-metadata.toml` and fails, naming the publication, when a category is not in the table, so a new upstream category cannot fall to Other unnoticed. `Society` is in the table although the file has no such value today; Pathfinder Society scenarios without a reviewed category show under Other.
- **Tier grouping is the default `#/books` view and replaces the old category groups.** The category groups were the same idea with a worse order: an alphabetical-by-product-line list that put Adventure Paths first. Keeping both would make two competing groupings of one list. The sorts now order books inside each tier: the default sorts by title, `?sort=category` by product line and then title (this separates Adventure Paths from Modules inside Adventures), and `?sort=date` by date newest first with unknown dates last. `?sort=library` is the only ungrouped view, in catalog order. A link that carries an old or unknown `sort` value opens the default tier view. Year groups are gone: a date sort inside a tier carries the same information, and the date still shows on each row.
- **Adventures start collapsed, and the heading shows the count.** Each tier is a native `<details>` with its `<h2>` inside the `<summary>`, such as `Adventures (226)`, and a muted `N of M active` line. Empty tiers do not render.
- **Group controls sit outside the disclosure.** `Activate all <Group>` and `Deactivate all <Group>` are real buttons beside the summary, so the collapsed Adventures group still deactivates in one click. They carry the group in their accessible name. `Only Rulebooks` is one page-level button (Rulebooks active, every other book inactive); it is disabled in a library that has no book with the `RPG` category, such as the PSRD carry-over, so it cannot leave no active book.
- **One write.** The controls call `Library::update_active_books`, which edits the stored disabled set (`disabled_books`) once and bumps the selection revision once. The per-book checkbox now goes through the same method. Discovery, search counts and the checkboxes update with no reload. Downloads and stars are untouched; ids in the stored set that the catalog does not know are kept.
- **No change to the default.** The stored set stays empty on a fresh install, so every book, including Adventures, starts active.
- **Not in this change.** The grouped Publication filter in search is Phase B (waits for #109).
