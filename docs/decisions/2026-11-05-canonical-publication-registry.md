---
title: "Canonical publication registry"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Canonical publication registry, 2026-11-05

- **One book, one source id.** Pf Data and PSRD-Data spell the same publication differently. The canonical build may not use either spelling directly, or the same book appears twice in the book view with two source facets. `sources/canonical/publications.toml` maps every Pf Data publication title to one canonical source id and one display title.
- **PSRD's sixteen books keep their source ids.** A title that exists in PSRD's reviewed book list keeps the PSRD source id and display title, so existing per-book preferences such as disabled books keep working. Every other publication uses `slug_segment` of its Pf Data title. The registry applies the same folding aliases as the alias gate (`PRPG Core Rulebook` → `core-rulebook`, `Pathfinder RPG Bestiary` → `bestiary`, `GameMastery Guide` → `game-mastery-guide`, and the numbered Bestiaries), so the two gates cannot disagree about which book a record names.
- **The registry is generated, reviewed, and committed.** `cargo xtask data-publications` rebuilds it from the pinned inventory; `cargo xtask data-publications --check` verifies the committed bytes. It includes every title the inventory names, including a title that appears only on an out-of-scope record: a `copy` record can inherit a companion's publication, and trimming the registry to record-level scope made conversion fail on `pathfinder-comics-10`. Extra rows are harmless; a missing row is a build error that tells the reviewer to regenerate and commit.
- **Title variants fold before dedupe.** Three publications appear twice in the pin with punctuation or case differences. Both rows rewrite to one source id, and each entry's `source_ids` is deduplicated, so a record cannot publish the same book twice. The display title is chosen deterministically, preferring the PSRD title when there is one.
