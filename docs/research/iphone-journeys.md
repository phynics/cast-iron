# iPhone journeys on the published site

Observed on 3 November 2026 in the device panel, iPhone 18 Pro simulator, iOS 27.2, Safari, at https://phynics.github.io/cast-iron/. The earlier baseline used the published five-tab navigation before PR #5 deployed. Settings reported app version `d4c49d51d4bf4bc8`, library `vdb8e844bfc068446`, content format 3, 9,967 entries and 59.0 MB.

## Earlier five-tab baseline

### Installation and update

The library download started before this session and completed during it. Home displayed the 9,967-entry search and categories. Settings confirmed the installed library and showed its installed time. Settings > Updates > Check for library update returned "The library is up to date." Settings > Storage said persistent storage was not granted and warned the browser may clear the library under storage pressure.

Safari Page Menu > Share opened the share sheet. The visible initial actions included Add to Bookmarks, Add to Reading List, and View More. Pressing View More returned to the page during this session, so Add to Home Screen and standalone launch were not established. Offline launch and a replacement-library update were not tested; the update check had no newer library available.

### Quick lookup

From Home, entering `grapple` and pressing the keyboard Search key opened search results. The top visible results began with two Grappler's Mask items, then Greater Grapple and Improved Grapple feats. The rule itself was not visible in the initial screen. Attempting to tap the Improved Grapple result near the bottom instead opened Browse: the fixed bottom navigation occupied that location. This observation does not establish whether scrolling the result farther above the bar would make it tappable.

### Book reading and saving

Browse > By book > Advanced Player's Guide showed 974 entries, nine chapters on the book list, source attribution, and a prominent warning that the contents are a draft and may have missing, misplaced, or differently titled sections. Expanding Races exposed Read Races, Races, Racial Attitudes, Racial Traits, Racial Favored Classes, and nested Dwarves. Read Races opened the rule, with a source link back to Advanced Player's Guide and links including Dwarves and Elves. The Bookmark switch changed to Bookmarked, and Collections opened a form to create a named collection.

The collection was not created. The device command to fill its name failed with `no text input found at the provided coordinates to clear`. The advised retry with `--debug` failed with `Ref @e31 belongs to an expired ref frame`. Device work stopped at that point. Persistence after reload and entry previous/next controls were not checked.

### Prioritized observations

1. The fixed navigation can overlap a result at the bottom of the viewport. A tap intended for Improved Grapple activated Browse. Check scrolling and bottom padding with real touch targets, including with Safari's browser bar visible.
2. A quick `grapple` lookup initially presents equipment and feats before a visible grapple rule. Test whether ranking or a rule filter can make the common rules lookup faster. The first screen alone does not prove the rule is absent from the results.
3. Home already has a search field, while Search also occupies a permanent tab. The published five-tab bar and Safari's browser controls consume vertical space. Keyboard Search did submit the Home query successfully; keyboard and safe-area behavior across other screens remain unverified.
4. The book draft warning is specific and visible before entering a chapter. The book tree requires Browse > By book > book > expand chapter > Read to reach the sampled rule, and its nested links add further taps.
5. Safari reported persistent storage was not granted after installation. The visible warning makes the risk clear, but an offline restart and whether requesting persistence succeeds were not checked.

### Journeys not completed

A full character-building journey through race traits, class and archetype choices, spell lists, character traits, equipment, and a saved collection was not completed. Neither a comprehensive combat subsystem read with cross-links and backtracking nor original-order previous/next reading was completed. Add to Home Screen, standalone launch, offline persistence, replacement updates, and dark theme were not verified. These are open checks, not failures attributed to the app.

## Follow-up on the published four-tab build

The primary agent continued personally on the same iPhone 18 Pro simulator and iOS 27.2 on 3 November 2026. PR #5 had merged at `a9360c5`. Main CI run 41595871949 and Pages deployment 41595872013 succeeded. Safari Settings reported app version `88beba653a369ae7`. The observations below concern the public site, not a new local build.

### Updates and installation

Safari displayed an Update ready banner. Reload activated the four-tab shell, with Home, Browse, Books, and Settings. The existing Races bookmark remained on Home. Settings > Check for updates offered library `v171f0348e5717ee0`. Download update completed, and Settings confirmed that version, 9,967 entries, content format 3, and 59.1 MB. The Races bookmark remained after library activation.

The Safari bookmark manager contained the named collection `Iris - fighter research`. Assigning Races to that collection changed its count to one. Opening the collection showed the saved Races entry.

Safari Page Menu > Share > View More > Add to Home Screen opened the native installation sheet. Open as Web App was enabled. Add created a Cast Iron Home Screen icon, and launching it opened the app without Safari browser controls.

The standalone app started with a fresh library installer, despite Safari already having the library and saved entry. Installing the 59.1 MB library succeeded, but Home initially had no bookmarks. Safari and the installed app had separate stored data in this run. This observation does not establish the behavior of other iOS versions or physical devices.

### Reference journeys in the standalone app

| Journey | Observed result |
|---|---|
| Quick rule lookup | `grapple` initially included two Grappler's Mask results. Rules text narrowed the results to 49 and placed Special Attacks first. Opening it showed the rule body. |
| Subsystem reading | Special Attacks began at Aid Another. Normal swipes reached Charge and its movement rules. The entire subsystem and its cross-link history were not read. |
| Race and racial traits | Races > Core Rulebook races > Dwarves exposed 20 traits grouped by source. Darkvision opened its 60-foot rule and parent race link. Next in book opened Defensive Training with its +4 dodge bonus against giants. |
| Class options | Classes > Fighter exposed 23 archetypes grouped by source. Archer opened its parent Fighter link and Hawkeye replacement text. |
| Spell lists | Magic Missile displayed class and level links. Wizard 1 opened the Wizard level 1 list with 116 spells and navigation through levels 0 to 9. Name filtering and another level were not exercised manually. |
| Character traits | Browse > Traits > Combat showed 50 results. Anatomist from Advanced Player's Guide opened its critical-confirmation bonus and Combat category link. The list also showed same-name entries from other books. |
| Equipment | Items > armor showed 27 results. Adamantine Breastplate opened with its 10,200 gp price, 30 lb. weight, and damage-reduction description. |
| Book contents and character rules | Books > Core Rulebook > Getting Started > Generating a Character opened the character-building introduction and ability-score step. The book's draft warning remained visible before entering its contents. |
| Saved reading after relaunch | Special Attacks was bookmarked. Closing and reopening `com.apple.webapp` launched Home with that bookmark and recent reading intact. Opening the saved entry restored the article with Bookmarked checked. Network remained available. |
| Appearance | A light-theme article and dark-theme Settings screen were inspected. Match system followed the simulator's dark appearance. The sampled text and navigation fit the screen. This was not a complete theme audit. |

These samples cover the components of character research, but do not establish a complete character build or eligibility calculation. Feat chains, physical-device behavior, and a full subsystem read with backtracking remain open manual checks.

### UX findings

1. Installing the Home Screen app after using Safari required another library download and did not carry the saved entry into the standalone app. Installation guidance needs to explain this observed transition before users invest in saved research. A transfer mechanism would need separate design and verification.
2. Rules text finds Special Attacks for `grapple`, but opens the beginning of the long article. A matched-heading link or article contents would shorten the remaining lookup.
3. The expanded archetype list appears above Fighter's main class information. Returning readers must pass the options list to reach the core class rules.
4. Trait browsing repeats names across publications. Source labels distinguish them, but the user still has to choose a book copy.

### Driver limits and unverified checks

True offline launch was not verified on this simulator. The driver's Apple Simulator Wi-Fi and airplane settings change the status display rather than disabling network traffic. The successful standalone relaunch above therefore proves stored-library and bookmark persistence with networking available. Automated offline tests remain separate evidence.

Share-sheet accessibility coordinates were offset from visible controls. A screenshot-guided coordinate tap reached View More and completed installation. Standalone snapshots labeled the web app as a system web sign-in sheet even though the visible content was Cast Iron. Normal swipes worked, while scroll-until incorrectly reported no movement. A header text replacement reported `TEXT_ENTRY_MISMATCH`; retrying on the fresh Search input with an 80 ms typing delay succeeded. An Items snapshot failed with `regular iOS snapshot presentation requires a valid viewport`; screenshot-guided navigation to armor recovered the accessibility tree. These were driver failures and recovered actions, not attributed app failures.

Documentation-only follow-up validation: `cargo xtask check` and `git diff --check` passed. No app code or content changed, so the browser acceptance suite was not rerun for this report.
