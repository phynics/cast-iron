---
title: "Spell variants and spell mythic sections"
date: 2026-11-08
status: current
superseded_by: []
issues: [143, 246, 250]
prs: []
area: content
---

# Spell variants and spell mythic sections, 2026-11-08

Refs [#143](https://github.com/phynics/cast-iron/issues/143), [#250](https://github.com/phynics/cast-iron/issues/250), [#246](https://github.com/phynics/cast-iron/issues/246). The app half of slice 5; corpus v0.1.11 holds the 426 `variant` edges.

- **A variant page names its direct base.** The metadata line ends with `Variant of {base}`, linking that one entry. A chain is not flattened: Mass Cure Moderate Wounds names Mass Cure Light Wounds, and only that page names Cure Light Wounds. An inactive base keeps its link and gains the plain `Inactive` label. Search and Quick search rows keep the ordinary `Spell · {book}` line, because DESIGN.md gives a variant no separate row label.
- **A base page lists only its direct variants.** `Variants` is an `.rel-list` after the body: each row is the variant's title with a muted `{levels} · {book}` line. Level 0 reads `Cantrips` through `fmt::spell_level`. The levels and book come from `lists.json` and the catalog, so the row never opens the variant's shard. The variant's own book alone gates the row; an inactive base never hides it, and a base with no eligible variant shows no `Variants` heading.
- **Spell pages load the relation index.** Both the variants row and the mythic section need it, so `needs(EntryKind::Spell)` now loads `relations.json` as well as `lists.json`. The raw graph supplies the add-on edges; eligibility is tested per add-on id, matching the feat page.
- **Mythic spells use the feat section unchanged.** `Related` calls `mythic_panel` for a spell too, so corpus cast-iron-data #30 can pin `ref/spell/<base>/mythic` add-ons without a second rendering path. The pinned v0.1.11 has no spell mythic add-on yet; the native test covers the branch until the pin PR adds the browser test.
- **A typed relation suppresses the backlink row.** The reverse index already drops a body link whose source states any typed relation to the same target, so a variant's `functions like` link does not repeat the base's `Variants` row under `Referenced by`. Feat chains read only `prerequisite` edges, so a `variant` edge never joins a chain.
