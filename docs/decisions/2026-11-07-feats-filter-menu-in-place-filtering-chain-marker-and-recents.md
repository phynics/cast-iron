---
title: "Feats filter menu, in-place filtering, chain marker and recents with scope"
date: 2026-11-07
status: current
superseded_by: []
issues: [193, 210, 213]
prs: [2, 3]
area: ui
---

# Feats filter menu, in-place filtering, chain marker and recents with scope, 2026-11-07

Refs [#193](https://github.com/phynics/cast-iron/issues/193), PR 2 of 5. The council's spec (filter menu, filtering in place, Feat chains as a plain link) applies to Feats; the owner's two decisions in the issue comments are recorded here too.

- **One filter menu replaces the feat-type pill wall.** A button `Type: All types` opens an in-flow panel (not a dialog) because a fixed panel with a find field would sit under the iOS keyboard. Options are links to `#/browse/kind/feat?facet=feat_type:<value>`, so the route is the state and Back and Forward walk the filter. The role is written up in DESIGN.md.
- **Deviation from the council: opening focuses the find field (the current row when a menu has no find field).** The council kept focus on the current row so a phone's keyboard would not rise. The brief for [#210](https://github.com/phynics/cast-iron/issues/210) asks for the find field, and the panel is in flow, so the keyboard cannot cover it. The tradeoff is that the keyboard rises on phones at once. [#213](https://github.com/phynics/cast-iron/issues/213) keeps the same behaviour for every kind's menu; the spell school menu has no find field, so it opens on the current row.
- **The class is `.choice-row`, not `.choice`.** `.choice` already styles the installer's fieldset in Settings.
- **The list filters from the search index.** `EntryStub` has no facet data, so the page asks the index for the ids that carry the value (`SearchIndex::facet_ids_eligible`, `Discovery::facet_entry_ids`). The index format does not change (`INDEX_FORMAT_VERSION` stays 1); the lookup reads document lists the index already builds. `Route::BrowseKind` gains `facets`, parsed and written like search's `facet=` pairs. Only fields with a filter menu narrow a list; other kinds still link their facets to search until PR 3.
- **Feat chains is a `.page-link`** in the filter row. The 124 px card and the lede are gone from Feats; Spells keep their card until PR 3.
- **Owner decision: the chain marker.** A feat row is marked when its feat leads to more than 5 feats down its chain (`CHAIN_MARK_MIN = 5`, transitive, each feat once; any feat qualifies, not only chain heads). Measured on the pinned library: 75 of 3,583 feats (2.1 %), against 369 chain heads and 753 feats with any dependent; thresholds of 3, 5 and 8 give 113, 75 and 56. The count is computed in the client from the relations graph once per page, not per render, so no compiled data or `SCHEMA_VERSION` changes. The accessible name gains `, leads to N feats`.
- **Owner decision: recent searches store their scope.** The stored list changes from strings to `{text, scope?}` (`scope`: kind plus facets). Old plain strings are read as searches with no scope, so nothing is lost; a list written by this version is not readable by an older client, which then shows no recents. Two recents are the same when their text (any case) and their scope match. Tapping one restores its scope even when the page that opened Quick search has another. Starter searches take the page's scope, as before.
- **The results status line names the scope** (`12 results in Combat feats · Show all feats`, `18 results in Feats`). `Show all feats` clears facets and publications and keeps the text and the kind. Muse's advice to keep both the scope pill and the pressed `Feats` kind button stands.
- **Digits are grouped in the remaining counts** (`See all (N)`, `+N more`, `N chosen`, `Collections (N)`, `Showing all N`, `N publication entries`, the screen-reader counts). Book views, the Books directory and `book_count_line` stay plain until the Books rows change; the `fmt::count` comment says exactly that.
