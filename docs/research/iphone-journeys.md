# iPhone journeys on the published site

Observed on 3 November 2026 in the T3 Device panel, iPhone 18 Pro simulator, iOS 27.2, Safari, at https://phynics.github.io/cast-iron/. This is the published build with the five-tab navigation, not the local navigation changes. Settings reported app version `d4c49d51d4bf4bc8`, library `vdb8e844bfc068446`, content format 3, 9,967 entries and 59.0 MB.

## Installation and update

The library download started before this session and completed during it. Home displayed the 9,967-entry search and categories. Settings confirmed the installed library and showed its installed time. Settings > Updates > Check for library update returned "The library is up to date." Settings > Storage said persistent storage was not granted and warned the browser may clear the library under storage pressure.

Safari Page Menu > Share opened the share sheet. The visible initial actions included Add to Bookmarks, Add to Reading List, and View More. Pressing View More returned to the page during this session, so Add to Home Screen and standalone launch were not established. Offline launch and a replacement-library update were not tested; the update check had no newer library available.

## Quick lookup

From Home, entering `grapple` and pressing the keyboard Search key opened search results. The top visible results began with two Grappler's Mask items, then Greater Grapple and Improved Grapple feats. The rule itself was not visible in the initial screen. Attempting to tap the Improved Grapple result near the bottom instead opened Browse: the fixed bottom navigation occupied that location. This observation does not establish whether scrolling the result farther above the bar would make it tappable.

## Book reading and saving

Browse > By book > Advanced Player's Guide showed 974 entries, nine chapters on the book list, source attribution, and a prominent warning that the contents are a draft and may have missing, misplaced, or differently titled sections. Expanding Races exposed Read Races, Races, Racial Attitudes, Racial Traits, Racial Favored Classes, and nested Dwarves. Read Races opened the rule, with a source link back to Advanced Player's Guide and links including Dwarves and Elves. The Bookmark switch changed to Bookmarked, and Collections opened a form to create a named collection.

The collection was not created. The device command to fill its name failed with `no text input found at the provided coordinates to clear`. The advised retry with `--debug` failed with `Ref @e31 belongs to an expired ref frame`. Device work stopped at that point. Persistence after reload and entry previous/next controls were not checked.

## Prioritized observations

1. The fixed navigation can overlap a result at the bottom of the viewport. A tap intended for Improved Grapple activated Browse. Check scrolling and bottom padding with real touch targets, including with Safari's browser bar visible.
2. A quick `grapple` lookup initially presents equipment and feats before a visible grapple rule. Test whether ranking or a rule filter can make the common rules lookup faster. The first screen alone does not prove the rule is absent from the results.
3. Home already has a search field, while Search also occupies a permanent tab. The published five-tab bar and Safari's browser controls consume vertical space. Keyboard Search did submit the Home query successfully; keyboard and safe-area behavior across other screens remain unverified.
4. The book draft warning is specific and visible before entering a chapter. The book tree requires Browse > By book > book > expand chapter > Read to reach the sampled rule, and its nested links add further taps.
5. Safari reported persistent storage was not granted after installation. The visible warning makes the risk clear, but an offline restart and whether requesting persistence succeeds were not checked.

## Journeys not completed

A full character-building journey through race traits, class and archetype choices, spell lists, character traits, equipment, and a saved collection was not completed. Neither a comprehensive combat subsystem read with cross-links and backtracking nor original-order previous/next reading was completed. Add to Home Screen, standalone launch, offline persistence, replacement updates, and dark theme were not verified. These are open checks, not failures attributed to the app.
