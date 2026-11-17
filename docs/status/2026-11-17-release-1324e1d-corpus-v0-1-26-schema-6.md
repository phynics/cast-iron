---
title: "Release `1324e1d` (corpus v0.1.26, schema 6)"
date: 2026-11-17
kind: release
---

# Release `1324e1d` (corpus v0.1.26, schema 6), 2026-11-17

`release` was promoted from `6365dba` to `1324e1de12baccce0f11ee90526dbe903c21567d` with
`scripts/promote-release.py --push`, a fast-forward push. This is the "format 5" release
([#441](https://github.com/phynics/cast-iron/issues/441)): it ships library schemas 5 and 6 together. The
search index format stays 2.

| Field | Value |
|---|---|
| Release commit | `1324e1de12baccce0f11ee90526dbe903c21567d` |
| Previous release | `6365dba`, content `v17a954e5c0f1d11c` (schema 4, corpus v0.1.17) |
| Content | `vb4ec72f397a03202`, schema 6, index format 2, 31,800 entries, 439 books |
| Shell | `cf5cf4d6590bd6a9`, released 2026-11-17 |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.26`, commit `a4b3ef6` |

**CI.** Main full CI run
[44167404059](https://github.com/phynics/cast-iron/actions/runs/44167404059) succeeded at `1324e1d`
with every job green, all three WebKit shards included.

**Deployment.** Pages run
[44180026338](https://github.com/phynics/cast-iron/actions/runs/44180026338) succeeded for
`headBranch: release` at `1324e1d` (build and deploy). The live `content/manifest.json` reads
`schema_version: 6` and `content_version: vb4ec72f397a03202`. The live `sw.js` stamps
`SHELL_VERSION = "cf5cf4d6590bd6a9"` and `SHELL_DATE = "2026-11-17"`.

**Not run.** The owner waived the separate browser acceptance run (upgrade from schema 4, interrupted
download, persistence, offline, fresh install) and the live-site checks of
[releasing.md](../releasing.md) "Verify publication" steps 2 and 3. **The upgrade from the published
schema 4 release to schema 6, and the persistence of stars, recents and active books across it, were
not verified for this release.** The CI suite above covers the individual install, update, storage
and offline paths on Chromium and WebKit, but its update spec copies the current library and so
exercises schema 6 to 6, not 4 to 6. The reader accepting schemas 1 to 6 is implementation evidence,
not a completed upgrade check. No physical-device check was run; the owner accepts browser evidence
(decision of 2026-11-17 on #441).

**Known open issue.** [#121](https://github.com/phynics/cast-iron/issues/121): WebKit can raise
`Context is stopped` around navigation. The cause is in something the app does and is not found.
Tests tolerate it only during main-frame navigation; no user-visible effect has been observed.

## Release notes

Changes since the release at `6365dba` (corpus v0.1.17, library format 4, 2026-11-09). The release pins corpus v0.1.26.

### Upgrading

The library format changes twice in this release, from 4 to 5 and from 5 to 6. Format 5 records every book that prints an entry. Format 6 stores archetype and class-option trades in a new file. The search index format does not change.

The new library is about 223 MB (213 MiB), so download it on Wi-Fi. The new app opens a format 4 library. Most of the changes below need the new library, which you install from **Settings**. The app does not download it by itself. The app is built to keep the old library in use until the new one has downloaded and passed its hash checks, so a stopped download leaves you reading with the old one.

The update is designed to keep your stars, recent entries, and active books as they were. The upgrade from the previous release was not tested end to end for this release, so tell us if anything is lost. A book that the new library no longer has (see "Books removed") is dropped from the list without an error.

### New features

- **Quick reference.** A fourth tab, **Table**, opens a page of 57 shortcuts to what you look up during play: the 35 conditions, 7 actions, 6 combat modifiers, and 9 rules you need every session. Quick search also lists it first while the field is empty. (#442, #456)
- **Share your book set.** The Books page has a **Share these books or enter a code** section. It shows a short code for the books you have on, with **Copy code** and **Copy link**. Entering a code or opening a link asks first ("Turns on 12 books, turns off 3"), changes nothing until you confirm, and offers Undo. A mistyped code is rejected. (#416)
- **"Did you mean" in search.** A misspelled name such as "magic missle" now offers the corrected search, on the results page and in Quick search. (#395)
- **Class pages.**
  - A summary card at the top shows hit die, base attack, good saves, skill ranks, class skills, alignment, starting wealth, and spellcasting. A row appears only when the app can read a reliable value. (#433)
  - A bar under the title jumps to Overview, Features, Archetypes, and Spells. Back returns to the previous section. It replaces **On this page** on the 45 classes that have it, so the long list of feature headings is no longer in one place. (#455)
  - On a phone, the progression table keeps the Level column in place while you scroll, and the spells-per-day columns sit in a closed section below it. (#391)
  - **References** and **Referenced by** start closed, with counts. (#433)
- **Printed sources on entries.** An entry ends with a line such as `Sources: Ultimate Combat p. 221`, with each book linked to its page. (#375)
- **Expanded in.** Under the sources line, `Expanded in: Monster Summoner's Handbook p. 18` credits a book that adds to an entry without printing it. The credit is dimmed and labelled **Inactive** when that book is off. (#472)
- **Archives of Nethys links.** Entries and book pages carry a **View on Archives of Nethys** link that opens in a new tab. The app makes no request until you tap it, so offline reading is unchanged. (#386)
- **Races by group.** The Races filter groups races as printed: core, featured, uncommon, and other. (#379)
- **Spell lists.** The spell hub and each class list use the same layout as other lists. A class list takes several schools at once, such as Evocation and Conjuration, and the search scope follows. (#392)
- **Book Contents.** Long hub sections, such as Universal Monster Rules in Bestiary 1 to 4 and Special Abilities in the Core Rulebook, show one row instead of a nested list. The previous and next buttons still step through every entry. (#374)
- **Diagnostics.** **Settings > This app > Diagnostics** shows the app's own timings and storage use as text, with **Copy results**. Nothing is sent anywhere. (#436)

### How books and membership work

An entry belongs to every book that prints it, and it stays visible while any one of those books is on. Before, one placement in one book decided the entry's visibility, so turning that book off could hide a rule that a book you kept on also prints. (#434)

What changes at the table:

- Acrobatic, which the Core Rulebook prints and Mythic Adventures also lists, stays visible with Mythic Adventures off.
- Book counts, book pages, and search book chips count an entry under each book that prints it. Filtering by a book finds the entry through any of its books.
- A list row names the entry's first printed book while that book is on. Otherwise it names another book that is on.
- Contents, chapter pages, and the previous and next buttons follow printed placement, as before.
- A page for a book that is off lists the entries that another active book keeps.
- An entry placed under a book that does not print it no longer counts as a member of that book. (#440)
- The chapter of rules that were never placed in a book (**New entries (page order)** and **More from this book**) is no longer shown. Every such rule is now in a printed chapter or reachable from a link. A search for its title still finds it. (#377)

**Expanded in** credits are separate from membership. A book that expands an entry but does not print it gets a credit line on the entry. It does not count as a member, so it does not keep the entry visible. Monster Summoner's Handbook, for example, expands Summon Monster I to IX, so those spells are hidden when that handbook is the only book on. (#469, #472, corpus v0.1.26)

### Books removed

Nine book records are gone because they were not real publications or duplicated another book: `prpg-core-rulebook` and `pathfinder-rpg-bestiary` (merged into Core Rulebook and Bestiary), three Paizo blog posts, three Pathfinder Comics issues, and Pathfinder Worldscape #2. The library lists 439 books, down from 448. Old links to the two merged books open the replacement. Old links to the other seven open the Books page with a note that the book is no longer in the library. The Game Mastery Guide record was also merged into one, and stored choices follow it. (#377, #440, #465)

### Fixes

- **Spell levels.** 228 spells that had taken their level and fields from a related spell now show their own printed values. This covers Summon Monster II to IX and Summon Nature's Ally II to IX, so each now sits at its own level on class spell lists and in the Variants panel. (corpus v0.1.23)
- **Missing words.** Rows like "At 6th level:" had lost their verb. The verbs are restored, with corrections from print, and the summon rows name and link the spell. About 1,600 broken character sequences in names and prose are repaired. (corpus v0.1.20 to v0.1.23)
- **Stray `p!` text.** Entries no longer show a left-over `p!` marker, such as "At 6th level: p!These bonuses +2." This affected 23 entries. (#389)
- **Races.** Races sit in one list, with the seven Core Rulebook race chapters merged into the typed race records, so every race has a group and appears under it. (corpus v0.1.21, #393)
- **Bloodrager.** The class card gives the spellcasting ability as Charisma. (corpus v0.1.25)
- **Bestiary 2.** Universal Monster Rules sit in the printed chapter, after Monsters A to Z. (corpus v0.1.25)
- **Third-party content.** Content from third-party publishers that had leaked into Paizo books is separated. (corpus v0.1.18)
- **Search.** A short prefix, such as "ma", now ranks the most common completions first. Before, it kept the first completions in alphabetical order and could drop the ones you wanted. (#429)
- **Search results on a phone.** The controls take one row, so the first result starts higher on the screen. Book-section matches sit behind one **Book sections** row, so entries come first. (#385, #388)
- **Browse lists.** Long titles and book names wrap between words. Before, a long publication name could split a title mid-word, as in "Abominatio" and "n". (#467)
- **Wide entry pages.** On a desktop-width screen, a large gap between the header and the entry text is gone. (#474)
- **Wide screens.** The header, Settings, Legal, and the footer line up with the left edge of the page content. Home shows Starred in the same columns as the category cards, with six starred entries before **See all**. (#409, #411, #413)
- **Error messages.** When the app cannot read your book choices or a link names an unknown book, the message says what happened and what to do, without internal codes. The recovery screen offers **Try again** and **Reset book choices**, and your library and stars are safe. A search link with an unknown book gets a **Remove it** button. (#465)
- **Keyboard focus after navigation.** Moving to a new page no longer takes focus away from a control you have already focused, and a quick second navigation no longer moves focus twice. A plain navigation still moves focus to the page, so screen readers announce it. (#479)
- **Frozen page after an internal error.** If the app hits an internal error, it now shows a **Reload** notice. Before, the page could stay on an old screen while the address changed. The cause of the report was not reproduced, so this is a safeguard, not a confirmed fix. (#471)

### Library and corpus

The release pins corpus v0.1.26. It replaces v0.1.17 and adds the changes above. Beyond them:

- **Printed sources.** 25,895 entries carry typed citations with page numbers. (v0.1.19)
- **Archives of Nethys links.** 12,467 entries and 426 books link out. (v0.1.19)
- **Membership fixes, v0.1.26.** The books checked did not print these entries, so they no longer count as members:
  - 141 racial-trait memberships on 51 entries, across 8 books, from Advanced Class Guide to Heroes of the Streets (data #96).
  - 129 racial-trait memberships on 46 entries, across Heroes of the Wild, Horror Adventures, Legacy of Dragons, Legacy of the First World, Occult Adventures, Rival Guide, and Ultimate Intrigue (data #97).
  - 75 memberships removed where a book only mentions an entry, prints a related entry that now carries the credit (such as a spell variant), or was credited by mistake. Ten more rows become **Expanded in** credits: Monster Summoner's Handbook p. 18 on Summon Monster and Summon Monster I to IX. One page number is corrected (Magical Beast Shape, Ultimate Wilderness p. 232), and Ultimate Combat p. 45 is added to Firearms and Monk (data #98).
  - 57 more memberships: Ultimate Wilderness racial traits, spell variants credited to the wrong book, three passing mentions, and two Bestiary 5 sea urchin poisons whose printed version differs from the entry. Siege Engines gains Ultimate Combat p. 45, which prints that weapon group (data #99).
  - In total, book memberships fall by 409 across 35 books, and no book gains an entry.
- **Entry count.** 31,800 entries and 439 books, unchanged from v0.1.25.

### Known limits

- Some archetype and class-option trades do not resolve to a class feature. The release stores them as printed text, so the Replaces line works but a compatibility filter has nothing to compare. This affects no screen yet. (#470)
- Class pages for prestige classes and four NPC classes have no section bar. (#455)

