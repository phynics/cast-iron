# AoN and legacy PRD book sections

Checked 2026-11-03 for Cast Iron's book view. This research compares three different structures: publisher print contents, legacy PRD navigation, and Archives of Nethys source/category pages. Only publisher contents can establish printed chapter order. Neither website's navigation nor its database grouping is sufficient evidence of that order.

## Directly checked sources

- [AoN Sources.aspx](https://aonprd.com/Sources.aspx) and its [RPG source listing](https://aonprd.com/Sources.aspx?ProductLine=RPG). The listing groups products by the site taxonomy (Adventure Path, Campaign Setting, Module, Player Companion, RPG, Miscellaneous) and orders each displayed table by product release date, newest first. This is a source catalogue, not a table of contents.
- [AoN Core Rulebook source display](https://aonprd.com/SourceDisplay.aspx?FixedSource=PRPG%20Core%20Rulebook) and [Advanced Player's Guide source display](https://aonprd.com/SourceDisplay.aspx?FixedSource=Advanced%20Player%27s%20Guide). These pages organize extracted records under database entity types (for example, spells, feats, traits, equipment, monsters). AoN itself says the page shows all items pulled from that source. It does not expose printed chapter boundaries or a reliable within-book sequence. The Core source display's category headings include material spread across different print chapters; do not turn them into book chapters.
- [Legacy PRD navigation index](https://legacy.aonprd.com/). Its HTML has a site-wide menu, with book headings and links below them. The link order is a legacy site navigation sequence. It sometimes groups multiple books under a heading (notably “Bestiaries”) and includes indices, appendices, and auxiliary pages. This is not a publisher TOC.
- Representative legacy book pages, with their URLs checked for HTTP success: [Core Rulebook Getting Started](https://legacy.aonprd.com/coreRulebook/gettingStarted.html), [Advanced Player's Guide Races](https://legacy.aonprd.com/advancedPlayersGuide/advancedRaces.html), [Advanced Race Guide Core Races](https://legacy.aonprd.com/advancedRaceGuide/coreRaces.html), [Bestiary Introduction](https://legacy.aonprd.com/bestiary/introduction.html), [Game Mastery Guide Planar Adventures](https://legacy.aonprd.com/gameMasteryGuide/planarAdventures.html), [Monster Codex Boggards](https://legacy.aonprd.com/monsterCodex/boggards.html), [Mythic Adventures Glossary](https://legacy.aonprd.com/mythicAdventures/glossary.html), [NPC Codex Core Classes](https://legacy.aonprd.com/npcCodex/core/index.html), [Technology Guide Skills](https://legacy.aonprd.com/technologyGuide/skills.html), [Ultimate Campaign Character Background](https://legacy.aonprd.com/ultimateCampaign/characterBackground.html), [Ultimate Combat Class Archetypes](https://legacy.aonprd.com/ultimateCombat/classArchetypes.html), [Ultimate Equipment Arms and Armor](https://legacy.aonprd.com/ultimateEquipment/armsAndArmor/index.html), and [Ultimate Magic Spellcasting Class Options](https://legacy.aonprd.com/ultimateMagic/spellcastingClassOptions.html). These confirm page titles and that standalone pages exist, not print sequencing.

## What the site structures tell us

Legacy navigation supplies useful **section labels and page paths**, which can help map Cast Iron records into human-readable sections. Examples include Core Rulebook's “Using Skills” and separate “Skill Descriptions”; Advanced Player's Guide's separate “Spell Lists” and “Spell Index”; Advanced Race Guide's “Age, Height & Weight” and “Spell Lists”; and Ultimate Equipment's “Rings, Rods, and Staves.” These page divisions may differ from the printed chapter boundaries. The Core Rulebook also lists both “Spell Lists” and “Spell Index” before “Prestige Classes,” while Cast Iron's current outline merges spell data into “Spells.” This is a mapping difference to review, not evidence for where the corresponding printed sections occur.

AoN's SourceDisplay grouping answers “which records does AoN associate with this source?” It can catch inclusion/coverage differences and gives category page links, but its entity buckets (including entries from class, NPC, spell, feat, monster, and equipment tables) are not book sections. Source title aliases also matter: the RPG list calls the first Bestiary “Pathfinder RPG Bestiary” and the Core Rulebook “PRPG Core Rulebook,” whereas the legacy menu says “Bestiary” and “Core Rulebook.” SourceDisplay's source-specific result is therefore an entity grouping, never a sequence for the book view.

## Comparison against Cast Iron's current outline

`book-order.toml` contains **16** `[[book]]` records, matching the count in the product note. They are Advanced Player's Guide, Advanced Race Guide, Bestiary, Bestiary 2, Bestiary 3, Bestiary 4, Core Rulebook, Game Mastery Guide, Monster Codex, Mythic Adventures, NPC Codex, Technology Guide, Ultimate Campaign, Ultimate Combat, Ultimate Equipment, and Ultimate Magic. AoN's RPG list contains 15 of these titles under its aliases. The [Campaign Setting list](https://aonprd.com/Sources.aspx?ProductLine=CampaignSetting) contains Technology Guide. The legacy menu represents Bestiary and Bestiaries 2–4 as monster-index links under the shared “Bestiaries” heading rather than full book sections. It also includes Bestiary 5, which is outside the current outline. This means legacy's top-level navigation is not a complete one-heading-per-volume index.

Legacy navigation's chapter-like link sequences are broadly useful as candidates for mapping Cast Iron's proposed groups, but they reveal material absent or collapsed in the present outline:

- Core Rulebook menu separates “Using Skills” from “Skill Descriptions”, has “Spell Lists” and “Spell Index”, and separates “NPC Classes” from “Creating NPCs”; Cast Iron groups skills and spells, and models NPC classes as a “Creating NPCs” chapter. The publisher PDF independently verifies the 15 main chapters; the website menu splits some of those chapters into several links.
- Advanced Player's Guide has distinct Spell Lists/Spell Index links and lists Magic Items after them. Cast Iron's single “Spells” group and its proposed chapter placement remain provisional.
- Advanced Race Guide has a Spell Lists page in addition to its race-builder/age-height pages; it is not represented as a section in the current Cast Iron outline.
- Bestiary navigation exposes auxiliary Monster Roles, Monster Creation, Monster Advancement, Universal Monster Rules, Creature Types, and Monster Feats links alongside monster indices. Their menu position does not certify appendix order.
- Game Mastery Guide menu has separate NPC Boons and NPC Gallery links; current Cast Iron groups NPC material together and has no publisher TOC evidence here.
- Mythic Adventures menu begins with Glossary, then Mythic Heroes/Feats/Spells and their spell index/list pages. This is a site-menu order only; it need not be print order.
- The current outlines for Bestiaries 2–4 include only Monsters, while legacy navigation exposes a monster-index page for each. A single index page is not proof that the source contains only that section.
- Ultimate Combat and Ultimate Magic menu links reveal page-level substructure, but cannot establish if those pages are separate print chapters or their original print order.

For a reproducible transcription of the complete `#nav-menu`, see [`legacy-prd-sections.json`](legacy-prd-sections.json). It preserves menu order, link labels, and absolute URLs, with `evidence` set to `site_navigation_not_printed_contents`. The shared “Bestiaries” menu remains shared; its links include indices for Bestiaries 1–5 and common aids, so the file does not imply a one-menu-per-volume structure. Nested Technology Guide links are emitted in their navigation order, with their enclosing menu labels in `groups`. Technology in the World is a menu label, not a standalone page at `technologyGuide/technologyInTheWorld.html`; that guessed URL returns 404. The snapshot includes Advanced Class Guide, Pathfinder Unchained, and Occult Adventures; these were omitted from the earlier hand-transcribed 16-volume summary. The file also records non-book menus (Index and Open Game License), since they are present in the navigation.

Regenerate it from a saved copy of the official Legacy PRD root page with `python3 scripts/extract-legacy-sections.py PATH/TO/index.html`. The extractor uses only Python's standard library and reads only `#nav-menu`, excluding page body and footer links. The input is local so the result can be reproduced offline.

## Limits and use

The directly checked legacy index is enough to document legacy navigation order and page names, but it is not a publisher contents listing. AoN source catalogue order establishes product grouping and displayed release-date order only. SourceDisplay establishes record association and category grouping only. None of these checks overrides the existing rule in `book-order.toml`: dataset tree order is not print-order evidence. Keep the book view explicitly provisional where printed contents have not been confirmed. The Core Rulebook publisher PDF and the verified 15-chapter sequence are recorded in `docs/research/book-order-evidence.md`. Nested placement and appendix grouping still need review.


## Rules source directory baseline, 2026-11-03

The user selected [Rules.aspx?Name=Source&Category=None](https://aonprd.com/Rules.aspx?Name=Source&Category=None) as the baseline for Cast Iron's book directory and rules browser. The page was fetched directly after the web reader could not open it. Its main content groups expandable source headings with linked topics and nested subsections. Core Rulebook topics include Basics, Combat, Environment, Exploration, Gamemastering, Items, Magic, and Skills. Combat links to Special Attacks among its subsections. This is a different page from Sources.aspx and SourceDisplay.aspx, which the earlier comparison examined.

This structure supplies a navigation baseline and mapping candidates. For example, its Core Rulebook topic sequence is not the printed chapter sequence verified above. The user's original-order requirement still applies. The implementation backlog therefore separates navigation mapping from printed-order evidence, unavailable books, and licensing review. No source selection or book-order configuration changed in this documentation pass.


## Reproducible Rules directory inventory

`scripts/extract-aon-rule-sections.py` extracts labels and absolute destination URLs from saved HTML. It preserves navigation order and can map normalized exact source titles to the installed catalog, then exact section titles within that matched book. It never matches a section against an unrelated book. No article bodies or artwork are retained.

The committed inventory has 23 source headings, 50 topics, and 233 subsections. The current strict mapping matches eight source headings to installed books. Among 283 topic and subsection rows, 95 have one matching entry, 16 are ambiguous, and 172 are unmapped. These statuses do not prove completeness or print order. Missing content was not imported.

Regenerate against a saved copy of the linked AoN page and the current compiled library:

```sh
python3 scripts/extract-aon-rule-sections.py /path/to/Rules.html docs/research/aon-rule-sections.json --catalog generated/content/v171f0348e5717ee0/catalog.json --books generated/content/v171f0348e5717ee0/books.json --mapping-output sources/psrd-data/aon-rule-mapping.json
```

Repeated extraction and mapping produced byte-identical files. The mapping is reviewed evidence, not compiler input.
