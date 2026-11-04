# Gemini screenshot review follow-up

The user supplied a completed Gemini Argon review on 2026-11-03 after manually guiding image access. This supersedes the earlier blocked attempt, which could only read PNG bytes. The supplied review concerns the existing four-tab screenshots in `/tmp/ci-pw-ui2`, before this implementation. It is screenshot evidence, not runtime or accessibility verification.

| Finding | Disposition in this implementation |
|---|---|
| P1: relationship navigation precedes primary class, race, and feat rules | Primary typed rule content now precedes relationship panels. A Related entries jump preserves quick research access, and archetype groups start collapsed. |
| P2: ambiguous active search scope and maneuver ranking | Selected quick scopes use a filled accent state. Kind facets and scopes derive from the same route state. Exact named rule sections receive a ranking preference over longer title prefixes, and matching rule results link to their heading. |
| P2: redundant Browse/Books switcher and heading | Removed the picker. Books has its own directory and heading. The old chapter-count label is no longer rendered. |
| P2: prestige requirement links and duplicated References | Feat and skill names in the matching requirement sections resolve to installed active entries. Punctuation is normalized; repeated names expose explicit publication choices. References excludes targets already exposed as racial-trait or archetype relationships. |
| P2: metadata pills and prestige breadcrumbs | Kind and source pills were already real links, so the screenshot does not establish a static-control defect. They remain links. Rule kind links lead to Books. The redundant Classes section label is removed from prestige breadcrumbs. |
| P3: Home card/row layout and school capitalization | Desktop Home shortcuts use three columns. Star rows keep title and kind close together within the reading measure. School labels are capitalized without changing filter values. The two reported Combat navigation labels now use consistent connector casing. Canonical titles and rule bodies remain verbatim. |

The initial desktop and mobile search captures showed different result/filter states. They do not prove a synchronization bug by themselves. The implementation and browser checks must establish that behavior.

The supplied review also asks for keyboard, VoiceOver, contrast, and touch-target checks. These remain separate verification requirements. Static screenshots cannot establish them. Physical-device behavior and complete assistive-technology acceptance must remain explicit gaps until exercised.


## Verification outcome

The implemented normal screens were captured at phone and desktop widths in both themes. Primary class stats and the start of rule text fit the first phone viewport. Separate settled search captures showed Rules text and the Rules facet selected together, with 49 rule results for `grapple`. The earlier immediate-after-click capture still showed the previous All entries results; waiting for route and result state removed that ambiguity.

Selected-scope contrast measured 6.02:1 in light mode and 6.54:1 in dark mode. The capture measurements found no horizontal overflow or alphabet/spell-level controls below 44 pixels. These checks do not establish full WCAG or VoiceOver compliance. The browser acceptance tests separately verify core-reading order, real source badges, prestige requirement links, and repeated section navigation.

## Portable-star and publication-choice follow-up

Home supported manual JSON star transfer at the time of this review. Repeated same-kind titles expose publication choices in category lists, search, and ambiguous prestige requirement links. Two reported Combat navigation labels use reviewed display aliases. The AoN audit now distinguishes absent sources from missing or ambiguous sections; it does not establish complete corpus coverage.

Note: manual star transfer was later removed; stars stay local to the browser profile (see `docs/decisions.md`).

The follow-up browser checks exercise keyboard focus, native disclosures, active-tab semantics, all 24 contents stops in the sampled Armor article, typed cross-links, history, and offline reading. WebKit uses Option-Tab in this environment to include buttons in sequential keyboard navigation. A manual browser walkthrough also read the rendered Armor article and followed its book navigation. VoiceOver and physical-iPhone offline standalone restart remain open.
