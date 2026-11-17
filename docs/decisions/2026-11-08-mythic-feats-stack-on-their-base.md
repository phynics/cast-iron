---
title: "Mythic feats stack on their base"
date: 2026-11-08
status: partly-superseded
superseded_by: ["2026-11-09-the-sources-line-follows-the-visible-text"]
issues: [178, 246, 262]
prs: []
area: content
---

# Mythic feats stack on their base, 2026-11-08

Refs [#178](https://github.com/phynics/cast-iron/issues/178), [#246](https://github.com/phynics/cast-iron/issues/246), [#262](https://github.com/phynics/cast-iron/issues/262). The app half of slice 2; corpus v0.1.7 holds the data.

- **144 add-on edges, not the council's estimated 148.** Four carried twins name a different feat in their prerequisites: Fabulous Figments names Spell Focus, Marked for Glory names three feats, Two-Fisted Drinker names Quick Draw, and Valiant Vault names Mounted Combat and Ride-By Attack. Their title extends a `ref/feat` entry that is itself the mythic printing, so none has a non-mythic base to attach to. They stay base-less with the 14 mythic-only feats. The 143 pasted sections that equal their twin are gone.
- **Powerful Shape keeps its paste.** The paste says "quick rules version"; the twin adds "from Pathfinder RPG Bestiary 295", and print agrees with the twin. The two texts differ, so the base still holds its section until the print-reviewed follow-up of #262. The add-on edge is already there.
- **The base page stacks the twin.** After the body: an `h2` `Mythic`, the add-on's title as an `h3` link, a muted `Sources` line, then the add-on's body expanded. The line names the add-on's book; the corpus carries no page number. The add-on's stat block is left out, because the page's own stat block keeps the only accent rule, and its headings become bold paragraphs so the section has only the `h2` and the title's `h3`. Only the add-on's book gates the section: the base's book never hides it.
- **Rows read the edge; the order reads the title.** A mythic row reads `Mythic · {book}` from its `addon` edge, in lists, search results and Quick search. In a feat list it is its own row directly after its base (`Dodge`, then `Dodge, Mythic`), without a row tag. The place uses the corpus title `{base}, Mythic`, because rows render before the relation file arrives; nine pairs would otherwise sit behind a title such as `Dodge Anything`.
- **Add-ons are not chain followers.** Chains, `Leads to`, chain heads and the chain marker read only `prerequisite` edges. Re-measured after the add-ons: 70 of 3,583 feats (2.0 %) pass `CHAIN_MARK_MIN`, and Power Attack leads to 92.
- **Labels and gating read the whole graph.** The discovery projection drops both ends of an edge, which would remove a mythic row's label while its base's book is inactive. Rows read the raw graph (a hit or row is already eligible), and the base page tests only the add-on's id against the discovery snapshot. The projected graph still serves the panels that need both ends.
- **Quick search follows the same rule.** Its suggestion lines read `Mythic · {book}`; the layer loads the relation index once, with the search engine already loaded.

Refs #178, #246, #262.
