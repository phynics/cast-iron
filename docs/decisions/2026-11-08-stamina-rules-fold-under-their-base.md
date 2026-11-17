---
title: "Stamina rules fold under their base"
date: 2026-11-08
status: partly-superseded
superseded_by: ["2026-11-09-the-sources-line-follows-the-visible-text"]
issues: [246, 250]
prs: []
area: content
---

# Stamina rules fold under their base, 2026-11-08

Refs [#250](https://github.com/phynics/cast-iron/issues/250), [#246](https://github.com/phynics/cast-iron/issues/246). The app half of slice 3; corpus v0.1.10 holds the data.

- **547 stamina add-ons in corpus v0.1.10, one per base.** Each is a kind `rule` entry `ref/feat/<slug>/stamina` titled `{Feat} (Stamina)` with an `addon`/`stamina` edge. Corpus figures: 465 come from Pathfinder Unchained, 45 from Weapon Master's Handbook, and 37 from Armor Master's Handbook. The base feats no longer carry the text.
- **The base page folds each eligible row.** After the Mythic section, one collapsed `details.rel-disclosure` per add-on, its summary `Stamina · {book}` on a 44 px target. The shard-backed body is created on the first open only and kept through close and reopen, so a closed row reads no shard and shows no status line. Opening it shows the add-on's body with headings demoted and no stat block, like a mythic version. The printed `Sources` line stays in the body because it carries the page number; the summary names only the book.
- **Only the add-on's book gates the row.** An inactive base never hides it, the same rule as a mythic version. With the add-on's book inactive the base page renders nothing.
- **The add-on's own page** ends its metadata line with `Stamina rules for {base}`, linking the base, plus a muted `Inactive` when the base's book is off.
- **Rows reuse the edge label.** Search and Quick search read `Stamina · {book}` through the same `addon` label path as a mythic row. Stamina entries are kind `rule`, so they never join a feat list; their titles already end in `(Stamina)`.
- **Chains ignore them.** The existing `addon` exclusion covers `stamina`; the base's `Leads to` panel and the chain marker read prerequisites only.

Refs #250, #246.
