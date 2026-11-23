---
title: "Monster stat sets: one `##` section each, a derived list, every set's CR"
date: 2026-11-23
status: current
superseded_by: []
issues: []
prs: []
area: content
---

# Monster stat sets: one `##` section each, a derived list, every set's CR, 2026-11-23

Owner ruling 5 on [phynics/cast-iron-data#166](https://github.com/phynics/cast-iron-data/issues/166). It is the app side of that ruling; the corpus converts the 17 monsters by hand later, and the app works before and after that.

- **Name.** A monster that prints several statistics (Air Elemental from Small to Elder, Ettercap, Draugr) has several **stat sets**. `family` already names the taxonomy links such as `Family: Elemental`, so it is not reused.
- **Convention.** Each set has its own `##` heading with an explicit anchor (`## Small Air Elemental {#stat-small}`), its `###` sections and its `::: statblock` blocks below it. A `##` heading is a stat set when its section holds a `Challenge rating` stat block row. No front matter marks it. `docs/corpus-format.md` documents the form.
- **Derived, additive.** The compiler derives `stat_sets` (title, anchor, CR, size) per entry from the body, in one function of the schema crate that the compiler, the drift check and the client share. The key is optional and omitted for an entry with fewer than two sets, so an older reader ignores it and `SCHEMA_VERSION` stays 6, as for `expanded_in`. Nothing is stored in the corpus.
- **Creatures only.** The compiler derives sets for entries with creature fields, and the entry page lists them for creatures only. `ChallengeRating::parse` rejects a sign, so a template's `CR +1` is no rating.
- **Half-done conversion.** `corpus-check` warns about a creature with two or more `##` sections that print `**Challenge rating:**` paragraphs and derive no sets.
- **Two or more.** One `##` section with a rating row is the entry itself and gets no list.
- **Drift.** `[fields]` describe the first set. For a creature, the drift check reads only the blocks before the second set's heading (rows and `**Challenge rating:**` lines alike), so a later set never drifts against `[fields]`, and a row that only a later set prints is not taken for the first.
- **CR filter.** The `cr` facet of a creature holds the rating of every set. The creature list shows the creature under each rating. `[fields] cr` stays the first set's rating, and the result row, other facets and the creature sort read `[fields]` only. The `size` facet is unchanged: the ruling names the rating.
- **Entry page.** A `Stat sets` list of jump links (`Small Air Elemental`, `CR 1 · Small`) sits above `On this page`. See DESIGN.md, Stat sets.
- **Before the conversion.** The 17 entries still print `**Challenge rating:**` paragraphs and no stat blocks, so they derive no sets and the page and filter do not change until the data lands.
- **Testing.** Native tests cover the derivation (two and seven sets, one set, a mythic rating), the drift rule and the facet. The browser spec `stat-sets.spec.ts` serves an overlay of the library that gives two creatures stat sets, because no pinned corpus has them yet.
