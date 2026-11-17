---
title: "Published navigation and common journeys"
date: 2026-11-03
kind: record
---

# Published navigation and common journeys

Primary destinations are Home, Browse, Books, and Settings. Search is a bar, and Home shows up to six saved entries plus recent reading. Quick scopes expose Rules text, Spells, and Feats. Rule headings feed the existing search alias field: `grapple` with Rules text selected surfaces Special Attacks first in the checked build. A native regression requires that entry in the first three results.

Content version `v171f0348e5717ee0` has 9,967 entries, schema 3, 61,956,172 bytes raw, and a 20,329,611-byte search index. Search format remains 1. The upgrade check preserves all 9,967 prior IDs and titles with no kind changes or dangling derived references. Publication checks and `cargo xtask check` pass. The full built-site suite passes 55 tests. Desktop and mobile screenshots in both themes were inspected; the screenshot run reported no horizontal overflow or console errors.

Composed offline journeys cover quick lookup, creating a collection and reopening a bookmark from Home, racial traits, class options and archetypes, class spell levels, traits, equipment, rules, and reading onward from book contents. A new app-shell test verifies that the worker waits for Reload, preserves the installed library and Home bookmarks, and works after an offline reload. The library-update test verifies replacement activation separately.

### Published iPhone Simulator checks

On iPhone 18 Pro, iOS 27.2, the primary agent personally applied the waiting app-shell update and replaced library `vdb8e844bfc068446` with `v171f0348e5717ee0` in Safari. The existing Races bookmark survived both updates. A named collection held that entry. Native Add to Home Screen installation and standalone launch succeeded.

The standalone app required its own library download and initially had no Safari bookmarks. After installing there, sampled journeys passed through Dwarves and racial traits, Fighter and Archer, Magic Missile and the Wizard level 1 list, combat traits, armor, Core Rulebook contents, and Generating a Character. Rules text placed Special Attacks first for `grapple`, but opened the beginning of the long article. Next in book opened Defensive Training from Darkvision. Closing and reopening the standalone app preserved a Special Attacks bookmark and recent reading. Sampled light and dark screens fit the viewport.

True offline simulator networking was not available through the driver. The relaunch check ran with networking available. A full subsystem read with backtracking, feat chains, and a complete character build remain open manual checks. See [iPhone journey observations](../research/iphone-journeys.md) for the earlier baseline, follow-up, driver failures, and recovered actions. Automated offline acceptance is separate evidence.

Navigation CI and deployment succeeded in runs 41595871949 and 41595872013. The earlier schema-3 reference UI deployed successfully in run 41590678502. The 16-book schema-2 deployment succeeded in run 41579539956. POC PR CI passed on GitHub in runs 41569305309 and 41569305238. The first Pages attempt failed before Pages was enabled. Deployment run 41576342801 then succeeded and the public site passed the live smoke check.
