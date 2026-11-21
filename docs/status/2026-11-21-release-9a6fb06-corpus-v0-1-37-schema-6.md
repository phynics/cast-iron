---
title: "Release `9a6fb06` (corpus v0.1.37, schema 6)"
date: 2026-11-21
kind: release
---

# Release `9a6fb06` (corpus v0.1.37, schema 6), 2026-11-21

`release` was promoted from `d200b4d` to `9a6fb061fb59c5e6c3ca8824169cc6df3d4f6569` with
`scripts/promote-release.py --push`, a fast-forward push. The owner gave the go ("let ongoing work land then
promote & release"). The library format does not change: schema 6 and search index format 2, as in the previous
release. The range is 11 first-parent merges: #558 (the record of the previous release) and the pull requests #559
to #568. It carries three corpus pins, v0.1.35 (#562), v0.1.36 (#564, corpus format 3) and v0.1.37 (#568)
([v0.1.35](2026-11-21-corpus-pinned-to-v0-1-35.md), [v0.1.36](2026-11-21-corpus-pinned-to-v0-1-36.md),
[v0.1.37](2026-11-21-corpus-pinned-to-v0-1-37.md)), and app changes (#559, #560, #561, #563, #565, #566, #567).
The app shell changed in the range (`web/` and `crates/web`), so the shell stamp is new.

| Field | Value |
|---|---|
| Release commit | `9a6fb061fb59c5e6c3ca8824169cc6df3d4f6569` |
| Previous release | `d200b4d81c393b12674c8edd5087a61d19765e65`, content `v24010454fb1d951c` (schema 6, corpus v0.1.34) |
| Content | `v28481793019cf75c`, schema 6, index format 2, 31,825 entries, 443 publications |
| Library size | 224,578,783 bytes in 294 files, from the live manifest (about 225 MB, 214 MiB; the previous release was 224,561,122 bytes in 294 files) |
| Shell | `738b7855d23307b9`, released 2026-11-21 (the previous release stamped `8009aeb778ed3216`) |
| WASM | `cast-iron-web-ceb5ab038a62a8aa_bg.wasm`, 2,783,498 bytes (the live `content-length`, and the length of the bytes a browser fetched) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.37`, commit `76517ea3cda1169892b8fd32eb297c091d984b9b` |

**Local.** Each pin entry records the local checks of its pin PR (`cargo xtask check`, `cargo xtask content`,
`build-site`, `size-budget` and one spec); the app PRs record their own. Nothing else was run locally for the
release itself, apart from the live check below, which ran against the deployed site. This record is a docs-only
change and needs `cargo xtask check` alone.

**CI.** Main full CI run
[44955036533](https://github.com/phynics/cast-iron/actions/runs/44955036533) at `9a6fb06` concluded `success`.
Every job passed (13 jobs), including the WebKit shards (informational).

**Deployment.** Pages run
[44957082851](https://github.com/phynics/cast-iron/actions/runs/44957082851) succeeded for
`headBranch: release` at `9a6fb06` (event `push`; build and deploy jobs both `success`). It was the newest Pages
run for `release` and the only one for this push.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-21, after the deployment, with a throwaway
Playwright script (Chromium and WebKit, a fresh profile for each run; the script is not committed). The live
`content/manifest.json` reads `schema_version: 6`, `content_version: v28481793019cf75c`, `entry_count: 31825`,
`total_bytes: 224578783`, `corpus_tag: v0.1.37`, 294 files, and lists
`v28481793019cf75c/legacy-sections.json` (400,307 bytes). The live `sw.js` stamps
`SHELL_VERSION = "738b7855d23307b9"` and `SHELL_DATE = "2026-11-21"`. The content version is the one that
`cargo xtask content` produced on `main` at the pin ([`docs/status.md`](../status.md) before this record).

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `v28481793019cf75c`, schema 6, 31,825 entries, corpus tag `v0.1.37`, 224,578,783 bytes in 294 files, `legacy-sections.json` listed | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens | pass, 13 s (24 s and 26 s in earlier runs) | pass, 13 s |
| `sw.js` stamp `738b7855d23307b9`, 2026-11-21; shipped `*_bg.wasm` 2,783,498 bytes (#565) | pass | pass |
| Abjuration (`ref/arcaneschool/abjuration`, #559): 7 powers, each with a head; Resistance (Ex) has a dotted leader (`.power-leader`, 2 px dotted border) and "Gained at Level 1", then the base effect "You gain resistance 5 …", then "At 11th level, this resistance increases to 10." | pass | pass |
| Evil Eye (`ref/hex/evil-eye`, #559): a one-power page with no power head, and a facts line that reads "Supernatural (Su)" | pass | pass |
| Fire domain (`ref/domain/fire`): "Domain Spells" is still text on the page and is not inside a power block | pass | pass |
| Wizard class page (#563): "Also available: Walking Library, Arch-Familiar"; both are links to `#/entry/ref/ability/capstones`, and following one opens "Alternate Capstones" | pass | pass |
| Arcane Archer (`ref/pclass/arcane-archer`, #567): the Sections switcher lists Overview, Requirements, Features | pass | pass |
| `#/entry/ref/pclass/arcane-archer?s=requirements` from another page: Requirements is `aria-current`, its section is focused and near the top (136 px) | pass | pass |
| Power Attack (#566): one "View on Archives of Nethys" link and one "Copy link" button; a click shows the status "Link copied." | pass | pass |
| The same click puts `https://aonprd.com/FeatDisplay.aspx?ItemName=Power%20Attack` on the clipboard | pass | not read (WebKit gives the script no clipboard permission) |
| Books (#564): in the first (Rulebooks) group, Game Mastery Guide is the 4th of 28, after Bonus Bestiary, Core Rulebook and Bestiary | pass | pass |
| `#/browse/book/the-elephant-in-the-room-feat-taxes-in-pathfinder/unpaged?a=toc-1-1` is replaced by `?a=row-ref-rule-class-ranger-ranger-teitr`; that row exists once, reads "Ranger (The …" and is in view | pass | pass |
| Transmutation (`ref/arcaneschool/transmutation`, v0.1.37): one "Anointings" heading and the line "Sources: Arcane Anthology pg 23" | pass | pass |
| Offline: reload on Detect Law with the network off shows the entry | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell | pass | pass |
| Offline: Evil Eye opens from the cache with its facts line | pass | pass |
| Page errors and console errors during the final run | none | none |
| Responses with a status of 400 or more | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper does.
The install used the Download button that a profile without a library shows, not the Settings page. The checks read
the rendered page text, attributes and the position of elements; they did not compare layout or styling beyond the
dotted leader's computed border and the section's offset.

- **First runs: three mistakes in my script, none in the page.** The first Chromium run failed two checks. The
  Copy link check read the status line at the moment of the click, before the app had written "Link copied."
  (the clipboard already held the right URL); it now waits for the line. The Elephant check opened the book
  address without a chapter, `#/browse/book/<id>?a=toc-1-1`; the address stayed as typed and no row was found.
  That form was not investigated. The published addresses name the chapter, as the e2e check of the Blood of
  the Beast addresses does, so the check now uses `/unpaged?a=toc-1-1`. A second run of both browsers then failed the Elephant
  check only because my script called a Playwright locator method that does not exist (`isInViewport`); the
  final runs, one per browser after the three fixes, passed every check. The earlier runs of the other checks
  gave the same results, and none of the four runs logged a console error, a page error, a response with a status of
  400 or more or a third-party request.
- **The WebKit clipboard.** WebKit gave no clipboard permission, so only the status "Link copied." was observed
  there, not the text on the clipboard. The e2e spec `aon-links.spec.ts` makes the same split.
- **No 503 and no `Context is stopped` this time.** Earlier release records name an occasional 503 for a library
  file and one WebKit page error; neither appeared in the four runs of this check.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The release was checked
  with Chromium and WebKit in a browser only.
- No existing installed profile: no profile with the `d200b4d` library, stars, recent entries and active books
  was updated, and no app-update or library-update flow was run. The app shell changed, so the app should offer a
  reload to a profile that has the old shell; that offer was not observed. The library format is unchanged
  (schema 6 to 6); CI's update spec covers that on a modified copy of the candidate library, not on the published
  library.
- Corpus format 3's equivalence with the stored books was not checked live beyond Game Mastery Guide's position
  and the Elephant address. The 152 books that the pin entry lists as different were not opened.
- The live check did not check styling: the look of the power head and its dotted leader at a phone width, in dark
  mode or in forced-colors mode, the Copy link button's 44 px target, and the section switcher at 390 px.
- The AoN link was not followed to `aonprd.com`, and how each platform opens it was not tried
  ([decision](../decisions/2026-11-19-aon-link-opens-where-the-platform-decides.md)).
- The live check did not read the printed notes, compare class-option powers with the books or open the five
  entries other than Transmutation that carry an Anointings section.
- `cargo xtask published-ids` was not run for this release. The pins add no entry and no alias, and the
  published-id guard of `cargo xtask content` passed at each pin.

**Known open issues.** [#121](https://github.com/phynics/cast-iron/issues/121): intermittent WebKit e2e failures.
[#557](https://github.com/phynics/cast-iron/issues/557): the class-option power check is an error from v0.1.35, with
19 powers excepted in `power-exceptions.toml`.

## Release notes

Changes since the release at `d200b4d` (corpus v0.1.34, 2026-11-21). The release pins corpus v0.1.37.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 225 MB (214 MiB), 17.7 KB
larger than before, so download it on Wi-Fi. The app shell changed (`738b7855d23307b9`), so the
app offers a reload. Install the library from **Settings**; the app does not download it by itself and keeps the old
library in use until the new one has passed its hash checks. Keeping stars, recent entries and active books is
the intended behaviour; it was not tested for this release against the published library (see "Not run").

### Changes to the app

- **Class-option powers read as printed prose** (#559). A page of several powers gives each one a head with a dotted
  leader to "Level N", the base effect, then one line per upgrade ("At 11th level, …"). A page of one power (a hex such
  as Evil Eye) folds the head into the title and shows a facts line, "Supernatural (Su)". Labels that are not part
  of a power, such as the Domain Spells of a domain, render as before.
- **"Also available" links** (#563). A class page's "Also available" capstones, such as the Wizard's Walking Library and
  Arch-Familiar, link to the shared Alternate Capstones entry.
- **Prestige classes have a Requirements stop** (#567). The Sections switcher of a prestige class reads Overview ·
  Requirements · Features, and `?s=requirements` opens at the Requirements.
- **Copy link** (#566). A button beside "View on Archives of Nethys" copies the address, for platforms where an installed
  app cannot open that link well. Copying makes no request.
- **A smaller app** (#565). The WASM is built for size (`opt-level = "z"`, search at `"s"`): 2,783,498 bytes shipped, where the build before
  the change was 3,449,043 bytes on `27e3209` (decision `2026-11-21-wasm-built-for-size`).

### Changes to the library

- **Corpus format 3** (corpus v0.1.36, #564). The 427 page-order books are derived from each entry's `[[sources]]`
  pages; 16 books stay curated in `books/`. Game Mastery Guide moves from position 123 to 7 in the book order. 15 derived
  titles change and 26 entries that had no placement now have one. The legacy address `?a=toc-1-1` of The Elephant
  in the Room opens the Ranger revision row.
- **Class-option powers** (corpus v0.1.35). 115 powers are resolved from print; 19 stay as exceptions; the compiler check is
  an error now (#562). 39 composite trades are split or settled, so trades are 5,943.
- **Flavor notes of class-option powers** (corpus v0.1.37). 15 placeholder rows such as "Text from the barbarian class." are
  removed and 6 cross-reference notes read as print words them.
- **Anointings** (corpus v0.1.37). Five entries (the Transmutation school, the Artifice domain, the Alchemist class, the
  investigator Alchemist Discovery talent and the Transformation patron) carry an "Anointings" section with the printed
  rule of *Arcane Anthology* p. 23 and `Sources: Arcane Anthology pg 23`.
- **Counts.** The library keeps 31,825 entries and 443 publications, 147,298 resolved links and 0 broken links.

### For contributors

- `corpus-check` reads `power-exceptions.toml`, and the class-option power check is an error (#560, #562). The
  class-option power spec no longer depends on fonts or the loading view (#561).
- **Docs.** New decisions: Copy link, class-option power exceptions, the legacy Ranger address, the prestige-class
  Requirements stop and the WASM built for size. The previous release record is on `main` (#558).

### Known limits

- 19 class-option powers are exceptions that the check does not hold to the canonical form.
- Two entries stay open for the owner because print is not confirmed (the Summon Devil prerequisite and the draconic
  bloodline Linnorm table).
- 115 placeholder notes in archetype entries, 5 in rule entries and 2 in class entries are not part of v0.1.37.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending (`notice_pending` is unchanged).
