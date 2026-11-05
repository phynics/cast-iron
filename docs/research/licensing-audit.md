# Licensing audit: PSRD-Data and PF1e rules material for Cast Iron

Status: fact gathering, not legal advice. Community Use Policy findings were re-verified from directly fetched official pages on 2026-11-03. Every claim below has a source (URL or file path) and, where useful, a short quote. Items marked UNVERIFIED were not checked.

## Open risks for redistribution in a public GitHub Pages deployment

1. **The derived dataset has no license.** `devonjones/PSRD-Data` has `license: null`, no LICENSE file on any branch, and no README on `release`. The only license text is inside `book-ogl.db`. Redistributing data scraped from Paizo pages rests on whatever Paizo's Open Game Content grant covers, not on the repo.
2. **OGL 1.0a covers only Open Game Content.** Paizo's PRD excludes Product Identity from the grant: "All trademarks, registered trademarks, proper names (characters, deities, etc.), dialogue, plots, storylines, locations, characters, artworks, and trade dress." The Community Use Policy (CUP) separately lets a free project make descriptive references to Product Identity such as names, deities, places, plots, and incidents. It does not license copying Product Identity prose or artwork (see "Community Use Policy review" below). Longer descriptive Product Identity text, such as deity or NPC write-ups, must be removed or rewritten; `content-policy.toml` removes it. A per-record OGC/PI audit has not been done.
3. **Community Use is a relevant permission route, and it needs no registration.** Paizo's licensing overview lists free online references and tools as a use. It supplements, rather than replaces, the OGL for rules. The registry and approved-products list no longer exist. See the dated review below.
4. **Ads and donations are expressly permitted.** The policy says so in terms. Paid sponsorship is also allowed if the sponsor is identified. Anything that charges for access, gates access, or looks like a business operation ends the permission.
5. **OGL 1.0a Section 10 requires a copy of the license with every distribution.** The app must ship the full OGL 1.0a text, not only the Section 15 line.
6. **Trademark.** OGL Section 7 forbids indicating compatibility with a trademark except under a separate agreement. The CUP allows descriptive references to trademarks and requires an unofficial project, but it does not mention compatibility claims. A 2019 FAQ answer treats the CUP as that separate agreement, but it refers to the removed product list. This project does not use the logo and makes no compatibility claim (AGENTS.md rule 11). Descriptive use of "Pathfinder First Edition" in the app description is a descriptive reference.
7. **GPL code is separate from the data.** PSRD-Parser and PathfinderOpenReference are GPLv3 for code. That says nothing about the data they generate. Copying code from them would bring GPLv3 obligations.
8. **Source is legacy.** Paizo: "The Pathfinder Reference Document will no longer be updated" (`https://paizo.com/pathfinderRPG/prd/`).

## Q1. Repository license statements and release branch

- **PSRD-Data** (`https://github.com/devonjones/PSRD-Data`): API `license: null`; default branch `release`. Root listing has no LICENSE, README, or COPYING (`gh api repos/devonjones/PSRD-Data/contents`). LICENSE returns 404 on `release`, `modified`, and `initial_load`.
  - Latest `release` commit: **76ea4784604145ba3dea40eb56f83bb904759467**, committed **2015-05-08T22:24:04Z**, message "latest updates of data as I modify rules to work on the dice roller" (`gh api repos/devonjones/PSRD-Data/commits?sha=release`).
  - Repo `pushed_at` is 2015-10-30 (API). A `gh search repos` result showed `updatedAt` 2026-10-15; that discrepancy is unexplained and unverified.
- **PSRD-Parser** (`https://github.com/devonjones/PSRD-Parser`): API license GPL-3.0. README: "PSRD Parser is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation". Its README describes it as parsing "the Paizo Pathfinder Reference Document".
- **PathfinderOpenReference** (`https://github.com/devonjones/PathfinderOpenReference`): API license GPL-3.0. README: "Pathfinder Open Reference is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License". README describes it as "an android app for displaying rules".
  - UNVERIFIED: the user's label "Masterwork Tools". The string "Masterwork" does not appear in the README.
- No repo states a data license for PSRD-Data.

## Q2. OGL 1.0a text: location in the dataset and on the web

**Dataset location (only one):** `/tmp/ci-dl-1/PSRD-Data-release/book-ogl.db`, table `sections`, `section_id = 4`, column `body` (HTML `<p>` wrapped, 6617 chars). Read with `sqlite3 -readonly`.
- Starts with the PI/OGC declaration paragraph, then "OPEN GAME LICENSE Version 1.0a", ends with: `15. COPYRIGHT NOTICE</p><p>Open Game License v 1.0a Copyright 2000, Wizards of the Coast, Inc.</p>`
- Section 1 (`section_id 1`, "Open Game License") and section 2 ("OGL") have empty bodies.
- `section_id 3` ("Product Identity") holds the PI declaration (quoted in Q3).

**Per-book representation of Section 15:**
- No other book DB contains the OGL text. `grep` for "COPYRIGHT NOTICE" in `book-*.db` (except `book-ogl.db`) and `index.db` returns 0.
- No JSON file in the dataset contains "Open Game" or "COPYRIGHT NOTICE" (`grep -r`, 0 hits).
- In `book-ogl.db`, each book has a copyright-line record (`section_id` 6 to 24, e.g. id 7 "Pathfinder Roleplaying Game Core Rulebook": "&copy; 2009, Paizo Publishing, LLC; Author: Jason Bulmahn..."). These are not Section 15.

**Verbatim check:** I extracted `section_id 4` from the dataset and the OGL block from Paizo's copy (`https://paizo.com/pathfinderRPG/prd/openGameLicense.html`). After stripping HTML and collapsing whitespace, both are 6156 characters with zero differing segments (difflib). The dataset and Paizo's page match each other.

**Wizards canonical copy:** NOT VERIFIED. The `wizards.com` RTF URL returned an unrelated page, and `media.wizards.com/2023/...OGL_1.0a.pdf` returned 404. The app should not claim verbatim equality with Wizards' original until that is checked.

**Files to read for the verbatim text:**
- `/tmp/ci-dl-1/PSRD-Data-release/book-ogl.db` (`sections.section_id = 4`, the OGL block after the PI/OGC paragraph)
- Paizo's copy, `https://paizo.com/pathfinderRPG/prd/openGameLicense.html`, OGL section

## Q3. Paizo PRD Open Game Content and Product Identity

Source: `https://paizo.com/pathfinderRPG/prd/openGameLicense.html`. Quote:

> "Product Identity: The following items are hereby identified as Product Identity, as defined in the Open Game License 1.0a, Section 1(e), and are not Open Content: All trademarks, registered trademarks, proper names (characters, deities, etc.), dialogue, plots, storylines, locations, characters, artworks, and trade dress. (Elements that have previously been designated as Open Game Content are not included in this declaration.)"

> "Open Content: Except for material designated as Product Identity (see above), the game mechanics of this Paizo Publishing game product are Open Game Content, as defined in the Open Game License version 1.0a Section 1(d). No portion of this work other than the material designated as Open Game Content may be reproduced in any form without written permission."

The PRD landing page `https://paizo.com/pathfinderRPG/prd/` says: "The Pathfinder Reference Document will no longer be updated, but is available for reference in its last updated state."

**Community Use Policy (`https://paizo.com/licenses/communityuse`, "Last Updated Thursday, August 22, 2024"):**
- Scope: "This policy authorizes certain non-commercial use of the Paizo-related material specifically identified in the Permissions section of this policy ("Paizo Material")."
- Required notice (verbatim): `"[This website, product name, Twitch show title, etc.] uses trademarks and/or copyrights owned by Paizo Inc., used under Paizo's Community Use Policy (paizo.com/licenses/communityuse). We are expressly prohibited from charging you to use or access this content. [This website, product name, Twitch show title, etc.] is not published, endorsed, or specifically approved by Paizo. For more information about Paizo Inc. and Paizo products, visit paizo.com."` Note the page's own text has `&nbsp;` entities; the notice should be typed as plain text. The notice must appear "in a legible/accessible form in each project, product, or on each website that uses any Paizo Material."
- "Your project must be unofficial."
- "Non-Commercial Use. If you make any commercial use of Paizo Material beyond what is explicitly allowed by this policy, your rights will be automatically terminated"
- Donations and ads: "You are allowed to accept donations on platforms like Patreon and to generate advertising revenue via YouTube, Twitch, and web ads."
- Permissions cover the "Community Use Package", product cover images, product descriptions, and blog text/artwork, with exceptions.
- **OGL interaction:** the policy text is silent on the OGL. Paizo's [licensing overview](https://paizo.com/licenses) and the FAQ answer it. See "Community Use Policy review, 2026-11-03". The earlier inference that the combination was unresolved was too broad.

## Community Use Policy review, 2026-11-03

### How each source was checked

I fetched every page below directly with `curl` (HTTP 200, browser user agent) on 2026-11-03 and read the text. The HTTP 403 noted in an earlier pass did not recur. Search-index summaries only located pages and agreed with them. I relied on none of them, and I did not rely on Paizo forum threads, which are informal user and staff posts.

| Source | Status in this review |
|---|---|
| [Community Use Policy page](https://paizo.com/licenses/communityuse) | Fetched. "Last Updated Thursday, August 22, 2024". |
| [Policy PDF](https://downloads.paizo.com/CommunityUsePolicy.pdf) | Fetched (96,699 bytes, PDF created 2024-08-21). Extracted text matches the page on notice, contact, descriptive-reference, harm, trade-dress, and indemnity clauses. |
| [Community Use FAQ](https://paizo.com/paizo/faq/v5748nruor1fw) | Fetched. Marked "Last updated: September 09, 2019", with answers posted March 2011. Partly superseded; see below. |
| [Reinstatement blog](https://paizo.com/blog/updates-on-the-community-use-policy-and-fan-content-policy) | Fetched. The page text shows no date. It follows the July 2024 termination and precedes the August 22, 2024 policy. |
| [Licensing overview](https://paizo.com/licenses) | Fetched. |
| [PRD Open Game License page](https://paizo.com/pathfinderRPG/prd/openGameLicense.html) | Fetched. The Product Identity declaration matches `sources/psrd-data/legal-notices.json`. |

### Findings

- **Reinstated and current.** The blog says the July 2024 termination "was an error", and the policy was reinstated with minor changes and "We have not changed the permissions granted by the policy." The Fan Content Policy "exists alongside" it. The Approved Products List and Community Use Registry were removed. A contact-information requirement replaced registration. No registration or approval step exists.
- **Scope.** The policy applies to "websites ... software, and other extensions of Paizo products". The licensing overview lists "online references and tools" and says content must be non-commercial and not behind a paywall.
- **Names, places, deities.** The policy allows descriptive references to "trademarks, proper names (characters, deities, artifacts, places, etc.), locations, dialogs, plots, storylines, language, and incidents based on Paizo Material". This covers the Product Identity categories that the OGL excludes. It is a permission to refer to them, not a licence to copy their text.
- **Copying text.** The 2019 FAQ says descriptive reference does not allow republishing "significant sections of descriptive text"; one must "craft new text". The policy text has no such sentence, and the FAQ predates the 2024 rewrite and cites the removed product list. I treat it as informal guidance that still fits the policy text. The policy grants no permission for product text or artwork beyond the Community Use Package, covers, product descriptions, and blog content.
- **OGL interaction.** FAQ: Open Game Content "falls outside the scope of the Community Use Policy" and must follow the OGL. Rules text therefore rests on the OGL, and names and setting references rest on the CUP. The licensing overview says the OGL excludes Product Identity and that one needs "an additional license instead of or in conjunction with the OGL". The same overview says game rules that came from a stream may be made freely available "using the Community Use Policy" once stripped of Paizo IP. The overview's Fan Content Policy section bars Paizo IP in products that contain game mechanics. That restriction is stated for the Fan Content Policy, not the CUP, so it does not apply here. The earlier recheck's conclusion stands: OGL plus CUP is a viable route. The earlier uncertainty about interaction was overbroad.
- **Free, donations, ads.** The policy requires free access with no paywall, survey, email gate, or unrelated download. Donations and "web ads" are allowed. Paid sponsorship is allowed if identified. The FAQ says banner ads are fine but charging for access is not.
- **Required items.** (1) The prescribed notice, "in a legible/accessible form ... on each website". (2) Reproduction of Paizo's copyright and trademark notices and author/artist credits found on the material used. (3) "up-to-date contact information in a visible location". (4) Unofficial status, no implied endorsement. (5) Permission from other IP owners for their content. (6) No Paizo trade dress or look-alike. (7) No adult or offensive use.
- **Retained risks.** Paizo may deny use "at any time for any reason or for no reason" and amend or end the policy. Users must indemnify Paizo. Permissions are personal and not transferable. Anyone Paizo sees as "operating as a business" loses the permission. These are policy terms, not defects in this project.

### Artifact check

I compared the built `generated/content/v91f4bc8ffd606a17/legal.json` and the source files with the policy.

- **Notice:** `community_use_notice` equals the policy notice with "Cast Iron" substituted, character for character. The compiler takes the template from the first occurrence of its placeholder in this file, in Q3. Do not add the placeholder text above that line.
- **Where it appears:** the Legal page renders it, and a link to that page appears in the app footer (`crates/web/src/app.rs`), the Settings screen, and the entry view. The README repeats it.
- **Contact:** the Legal page and README link to GitHub Issues. That satisfies the visible-contact rule. It needs a GitHub account and ends if the repository goes away. An email address would be more robust; this is optional hardening, not a requirement.
- **Credits and notices:** Section 15 carries all author credits for the 16 books. The PRD page's trademark-ownership line is inside an HTML comment, so no visible Paizo trademark notice exists to reproduce. The app uses no Paizo logo, artwork, or trade dress.
- **Commerce:** no payment, sign-in, ads, or telemetry. Rule 1 in `AGENTS.md` covers the last.
- **Third parties:** the Tome of Horrors and System Reference Document notices are in Section 15. Their permission rests on the OGL, not on Paizo's policy.
- **Gaps found:** none that the policy requires to be fixed. The residual items are the finite publication screen below, and the rule that every new source needs a review.

### Built POC scope review

`python3 scripts/check-publication.py` screened all 1,937 built entries from Core Rulebook and Bestiary. It found no matches for its listed campaign locations, deities, or iconic characters. Creature titles were also inspected and identify generic creature types and stat-block variants, rather than named campaign characters. The selected rules trees cover game procedures, equipment, and terminology. Campaign books, adventure plots, and artwork are outside this source selection.

The script verifies all manifest file hashes, the two-source scope, per-entry OGL references, and expected attribution for both Paizo books, the System Reference Document, and Tome of Horrors. The app preserves the full collected Section 15 notices, including third-party credits. Its report is `generated/reports/publication.json`.

This scoped review found no additional publication issue. It does not classify every sentence as Open Game Content or prove that no other Product Identity exists. Generic descriptive prose, including creature ecology, remains in the selected upstream rules. Future sources or a discovered rights issue require review and, where needed, removal recorded in coverage. A missing dataset-level license alone does not negate a valid upstream grant.

## Q4. ORC/OGL 2023 situation

Wizards' 27 January 2023 statement (DnD Beyond, `https://www.dndbeyond.com/posts/1439`): "We are leaving OGL 1.0a in place, as is. Untouched. We are also making the entire SRD 5.1 available under a Creative Commons license." It then says of the CC license, not OGL 1.0a: "We don't control that license and cannot alter or revoke it." and "Placing the SRD under a Creative Commons license is a one-way door. There's no going back." The official SRD FAQ (`https://dnd.wizards.com/resources/systems-reference-document`) says the CC move was made "so that creators have certainty that Wizards can never revoke or deauthorize SRD 5.1 content". Neither source states that OGL 1.0a itself is irrevocable. Wizards left it unchanged and did not revoke it. OGL 1.0a Section 9 also lets Wizards publish updated versions while allowing use of "any authorized version of this License" for content originally distributed under an earlier version. The OGL 1.0a text gives no irrevocability clause. Search results said the Open Gaming Foundation reported Wizards had prepared to deauthorize OGL 1.0a and then reversed; UNVERIFIED (not fetched).

## Q5. Alternative "Pf Data 1e structured JSON" candidates

Searched `gh search repos` and `gh search code` for pathfinder/pf1e json, psrd data, and open game license terms.
- **Not found:** no repo matching "Pf Data 1e structured JSON" by name.
- **`adamjcoonen/PSRD-Data`:** repo is empty (`"Git Repository is empty"`, size 0, license null). Not usable.
- **PCGen data (`https://github.com/PCGen/pcgen`, `data/pathfinder/`):** PF1e data in `.lst` and `.pcc` text format (not JSON; `search/code` for `.json` in `data/pathfinder` returns 0 and for `.lst` returns 2808). The repo license is LGPL-2.1 (`gh api repos/PCGen/pcgen`, size 365075 KB for the whole repo). `data/pathfinder/paizo/pathfinder_for_gms.pcc` carries: `COPYRIGHT:Open Game License v 1.0a Copyright 2000, Wizards of the Coast, Inc.` and `INFOTEXT:This dataset uses trademarks and/or copyrights owned by Paizo Inc., which are used under Paizo's Community Use Policy.` The `data/readme.md` has no license text. Data license for PCGen is UNVERIFIED beyond these headers.

## Tracked unresolved rights

Issue [#34](https://github.com/phynics/cast-iron/issues/34) owns the open licensing
decisions. The current production scope remains PSRD-Data plus one reviewed Pf Data
record.

- **Comics:** records whose only publications are *Pathfinder Comics* or
  *Pathfinder Spiral of Bones* are out of scope permanently, including Apollyon
  Clasp / *Spiral of Bones #5*. No further evidence will be sought.
- **Paizo Blog:** records whose only publication is one of the five registered
  blogs are ignored for now and tracked in
  [#36](https://github.com/phynics/cast-iron/issues/36) for a possible return.
  Records also published in an in-scope PF1e book stay.
- **PF2 material:** two PF2 publications are registered (`Monster Core`,
  `Bestiary 3 (2nd Edition)`). Eight tagged records, the `main.json#ogc` ORC
  notice, and the untagged PF2 *Player Core* description in
  `gnome_flick_mace` are explicitly out of scope; ORC text is not published under
  OGL.
- **PSRD-Data dataset license:** absent. The existing 16-book baseline relies on
  the upstream OGL/CUP grant; this risk is overridden for now and tracked.
- **Product Identity kept under the CUP (2026-11-05):** the owner chose to ship Paizo Product Identity, including deity descriptions, character biographies, and NPC stories, under the Community Use Policy. Every entry carries the notice path. This goes beyond the descriptive-reference reading in Q3 above; the owner accepts that risk. See the decision log.
- **Superseded: corpus-wide PI screening:** the Pf Data screen in
  `sources/pathfinder-data-1-e/pi-screen.json` omits deity lore, unique creature
  biographies, named NPC stories, and outbound links, and every Pf Data entry carries
  the Community Use notice path. Proper names inside mechanics stay as descriptive
  references (decision of 2026-11-04). The screen is a reviewed rule set, not a
  sentence-level classification, and no blanket clearance is claimed.

## Unverified items

- The "Masterwork Tools" name for PathfinderOpenReference.
- The Wizards canonical OGL 1.0a text (not fetched).
- The Open Gaming Foundation report on the 2023 deauthorization preparation.
- The `updatedAt` discrepancy for PSRD-Data.
- Per-record OGC/PI classification of the dataset.

## Sixteen-book expansion review, 2026-11-03

The same pinned PSRD-Data commit contains 16 book directories. The reviewed selection in `source.toml` now includes their supported typed records and 101 complete top-level rules trees. `scripts/expand-sources.py` reproduces the explicit selection and refuses an unknown book directory. The scope covers the two original books, Advanced Player's Guide, Advanced Race Guide, Bestiary 2 through 4, Game Mastery Guide, Monster Codex, Mythic Adventures, NPC Codex, Technology Guide, Ultimate Campaign, Ultimate Combat, Ultimate Equipment, and Ultimate Magic. This does not add every PF1e publication, and some record families remain available only inside rules trees or excluded as reported.

The [legacy PRD license page](https://legacy.aonprd.com/openGameLicense.html) supplies the Product Identity/Open Content declaration and Section 15. Its Mythic Adventures copyright was missing from the extracted snapshot notices and has been added. Supplemental notices for The Tome of Horrors III, Aerial Servant, Demon Shadow, Grippli, Nereid, Moon Dog, and Muckdweller from Tome of Horrors Complete are also preserved. Existing notices remain intact. Source typos in supplemental notices, including the Muckdweller year, are preserved rather than silently edited. This is a notice correction, not an additional game-content source.

`content-policy.toml` removes named NPC stories, suggested character names, two deity-named items, deity descriptions and religious traits, and identified setting-specific commentary. The same policy applies to standalone records and copies nested in rules trees. The coverage report records each omitted source path and reason, including partial-body omissions. There are 432 omission occurrences across duplicated upstream representations, not 432 missing entries. Artwork remains stripped by the HTML adapter. All 16 catalog publications resolve to a copyright notice.

The publication screen examines every built entry and found no match for its explicit list of named setting material. This remains a finite screen, not a sentence-by-sentence legal clearance. The selection preserves generic mechanics and descriptive rules material under the existing OGL and Community Use approach. A new source pin or additional book requires another scope and notice review.
