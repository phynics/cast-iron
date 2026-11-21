---
title: "Release `d200b4d` (corpus v0.1.34, schema 6)"
date: 2026-11-21
kind: release
---

# Release `d200b4d` (corpus v0.1.34, schema 6), 2026-11-21

`release` was promoted from `622eaa6` to `d200b4d81c393b12674c8edd5087a61d19765e65` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and
search index format 2, as in the previous release. The range is 18 first-parent merges (pull requests numbered #532 to
#556, the gaps being issues; #532 is the record of the previous release). It carries three corpus pins, v0.1.32 (#533), v0.1.33 (#545)
and v0.1.34 (#556) ([v0.1.32](2026-11-20-corpus-pinned-to-v0-1-32.md), [v0.1.33](2026-11-21-corpus-pinned-to-v0-1-33.md),
[v0.1.34](2026-11-21-corpus-pinned-to-v0-1-34.md)), and app changes. The app shell changed in the range
(`web/` and `crates/web`), so the shell stamp is new.

| Field | Value |
|---|---|
| Release commit | `d200b4d81c393b12674c8edd5087a61d19765e65` |
| Previous release | `622eaa6`, content `v4192151b29702eab` (schema 6, corpus v0.1.31) |
| Content | `v24010454fb1d951c`, schema 6, index format 2, 31,825 entries, 443 publications |
| Library size | 224,561,122 bytes in 294 files, from the live manifest (about 225 MB, 214 MiB; the previous release was 223,207,549 bytes in 292 files) |
| Shell | `8009aeb778ed3216`, released 2026-11-21 (the previous release stamped `0cc32985d5369a1e`) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.34`, commit `724016823d36061b6d53777a53acb31c2e7d53b4` |

**Local.** Each pin entry records the local checks of its pin PR (`cargo xtask check`, `cargo xtask content`,
`build-site`, `size-budget` and one spec); the app PRs record their own. Nothing else was run locally for the
release itself. This record is a docs-only change and needs `cargo xtask check` alone.

**CI.** Main full CI run
[44894283691](https://github.com/phynics/cast-iron/actions/runs/44894283691) at `d200b4d` concluded `success`.
Every job passed, including all four WebKit shards (informational).

**Deployment.** Pages run
[44898321278](https://github.com/phynics/cast-iron/actions/runs/44898321278) succeeded for
`headBranch: release` at `d200b4d` (event `push`; build and deploy jobs both `success`). It was the newest Pages
run for `release` and the only one for this push.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-21, with a throwaway Playwright script
(Chromium and WebKit, a fresh profile for each run; the script is not committed). The live
`content/manifest.json` reads `schema_version: 6`, `content_version: v24010454fb1d951c`, `entry_count: 31825`,
`total_bytes: 224561122`, `corpus_tag: v0.1.34`, 294 files, and lists
`v24010454fb1d951c/legacy-sections.json` (400,292 bytes). The live `sw.js` stamps
`SHELL_VERSION = "8009aeb778ed3216"` and `SHELL_DATE = "2026-11-21"`. The content version is the one that
`cargo xtask content` produced on `main` at the pin ([v0.1.34 entry](2026-11-21-corpus-pinned-to-v0-1-34.md)).

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `v24010454fb1d951c`, schema 6, 31,825 entries, corpus tag `v0.1.34`, 224,561,122 bytes in 294 files, `legacy-sections.json` listed | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens | pass, 11 s (25 s in an earlier run) | pass, 12 s |
| Masthead wordmark (#551): `.brand-mark` is 144 by 22 px, its mask is `icons/wordmark.svg`, and that file serves with status 200 and an SVG type | pass | pass |
| `#/browse/book/blood-of-the-beast/unpaged?a=toc-2-26` redirects to `?a=row-ref-class-ranger`; the row `row-ref-class-ranger` exists once, reads "Ranger" and is in view (#549) | pass | pass |
| The same with `?a=toc-2-27`: redirects to `?a=row-ref-class-rogue`, row reads "Rogue" | pass | pass |
| `?a=row-ref-class-ranger` opens directly, with no redirect | pass | pass |
| `#/browse/book/core-rulebook?c=~classes` keeps the address and marks only the "Classes" row (`aria-current="location"`) (#553) | pass | pass |
| `#/browse/book/core-rulebook?c=1` is replaced by `?c=~getting-started` and marks "Getting Started" | pass | pass |
| `#/entry/ref/spell/nightmare`: the marker in a table cell is a link named "Note 1"; clicking it brings the note into view and its back link returns to the marker (#536) | pass | pass |
| `#/entry/ref/arcaneschool/abjuration`: "Resistance (Ex)" reads "You gain resistance 5 …" and then "At 11th level, this resistance increases to 10." (#548, #552) | pass | pass |
| `#/entry/ref/monster/goblin`: `Armor class: 16`, `Touch: 13`, `Flat-footed: 14`, `AC modifiers: +2 armor, +2 Dex, +1 shield, +1 size`, `Hit points: 6 (1d10+1)`, `Fort: +3`, `Ref: +2`, `Will: -1` | pass | pass |
| Continue Reading (#547): with a newer saved point for a book that does not hold the entry and an older one for the Core Rulebook, the Core Rulebook page offers "Continue reading: <entry>" and the other book's page offers none | pass | pass |
| Offline: reload on Detect Law with the network off shows the entry | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell | pass | pass |
| Offline: the Goblin entry opens from the cache with its defense rows | pass | pass |
| Page errors and console errors during the run | none in the final run; see below | none in the second run; see below |
| Responses with a status of 400 or more | none in the final run; see below | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper does.
The install used the Download button that a profile without a library shows, not the Settings page. The Continue
Reading check wrote the saved points into the profile's `meta` store (the helper `seedHome` does the same); it
did not reach them through reading. The Abjuration, Goblin and Nightmare checks read the rendered page text and
links; they did not compare layout or styling.

- **One 503 in an exploratory Chromium run.** A first exploration run (it only opened pages and printed their text)
  logged one console error, "Failed to load resource: the server responded with a status of 503", and the
  response log named the URL: `https://phynics.github.io/cast-iron/content/v24010454fb1d951c/entries/class-004.json`.
  Three direct requests for that file afterwards returned 200, and no later run (two more Chromium runs,
  one of them the check above, and two WebKit runs) saw a status of 400 or more. The cause of the 503 is not known; the
  previous release record had the same kind of one-off 503 on a different, unrecorded URL. This one
  recovered on its own and did not fail the install, which still reached the reader. It is evidence that
  the host returns an occasional 503 for a library file, not that the file is bad.
- **First Chromium check run: my script's mistake.** The Contents `?c=` check first asked for the row
  `~races`. Races is not a top-level row of the Core Rulebook's Contents, so no row was marked and the plain book
  address stayed. That matches the design (a row that Contents does not list opens the book). The check was changed to
  `~classes` and `?c=1`, which are listed rows, and both browsers ran the final script. The page did not change
  between the runs.
- **One WebKit page error.** The first complete WebKit run logged `pageerror: Context is stopped` and every check
  passed. The error has the signature that [#121](https://github.com/phynics/cast-iron/issues/121) describes
  (a navigation that interrupts a read of the library cache). A second WebKit run was clean; the cause was not isolated.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The release was checked
  with Chromium and WebKit in a browser only.
- No existing installed profile: no profile with the `622eaa6` library, stars, recent entries and active books
  was updated, and no app-update or library-update flow was run. The app shell changed, so the app should offer a
  reload to a profile that has the old shell; that offer was not observed. The library format is unchanged, and
  CI's update spec covers schema 6 to 6 on a modified copy of the candidate library; it is not an upgrade of the
  published library.
- A real Continue Reading point that goes stale after a library update was not produced. The check above seeds one.
- The live check did not check styling: the sheen of the wordmark, its look in dark mode or forced-colors mode,
  and the typography of the powers and defense rows.
- The live check did not open the entries that changed in the corpus beyond those in the table. It did not read
  the printed notes or compare class-option powers with the books.
- `cargo xtask published-ids` was not run for this release. The pins add no entry and no alias, and the
  published-id guard of `cargo xtask content` passed at each pin.

**Known open issues.** [#121](https://github.com/phynics/cast-iron/issues/121): intermittent WebKit e2e failures.
[#557](https://github.com/phynics/cast-iron/issues/557): 133 class-option powers are not in the canonical form, so
the check stays a warning.

## Release notes

Changes since the release at `622eaa6` (corpus v0.1.31, 2026-11-20). The release pins corpus v0.1.34.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 225 MB (214 MiB), 1.4 MB
larger than before, so download it on Wi-Fi. The app shell changed (`8009aeb778ed3216`), so the app offers a
reload. Install the library from **Settings**; the app does not download it by itself and keeps the old
library in use until the new one has passed its hash checks. Keeping stars, recent entries and active books is
the intended behaviour; it was not tested for this release against the published library (see "Not run").

### Changes to the app

- **New wordmark.** The masthead shows an outlined SVG wordmark in Rozha One with a faint, fixed iridescent
  sheen (#551). It follows the page ink in both themes and in forced-colors mode.
- **Continue Reading** ignores a saved point whose book no longer holds the entry, and keeps an older valid point
  of that book (#547).
- **Book section links are stable** (#549). A section address `?a=` now names the row by its identity (`row-…`),
  not its place in the tree, so it keeps working when a library update adds or removes a row. Old `toc-…`
  addresses go through the library's `legacy-sections.json` to the same row, or open the chapter when the row
  has left the book.
- **Contents addresses** `?c=` name the row by identity too (#553); an old positional `?c=3` is replaced by the
  stable form. The book views share one redirect (#555).
- **Footnote markers in table cells and stat rows** link to their notes, and the note links back (#536).

### Changes to the library

- **Class-option powers are rewritten** in the printed-prose layout (corpus v0.1.34; the layout and owner
  decisions are decision records). A power reads its base effect first and then its upgrades, for example
  Abjuration's Resistance: "You gain resistance 5 …", then "At 11th level, this resistance increases to 10."
  133 of 3,857 powers are not yet in the canonical form and stay in the review queue (#557).
- **Creature defense blocks are labelled rows** (corpus v0.1.34): Armor class, Touch, Flat-footed, AC modifiers,
  Hit points, Fort, Ref, Will, then the other defense rows. 2,897 blocks in 2,848 entries.
- **Monster spell lists** read the printed wording: `quickened spell`, `summon (level N, …)` and so on, for 404
  shorthand markers in 317 monsters (corpus v0.1.33).
- **Trades and memberships** (corpus v0.1.33). Archetype and class-option trades move to corpus format 2: 5,567 of 5,905
  trades name a class feature, 338 stay unresolved with a reason. 114 book memberships that the books do not
  print are removed from 110 entries.
- **Corpus clean-up** (corpus v0.1.32). HTML entity references in front matter are replaced, the last orphan
  footnote markers are restored, and a footnote that Detect Chaos, Detect Fiendish Presence, Detect Good and Detect Law copied from
  Detect Evil is removed.
- **Counts.** The library keeps 31,825 entries and 443 publications, 147,298 resolved links and 0 broken links.

### For contributors

- **`corpus-check`** fails on unresolved footnotes and front-matter entities (#535), on a leftover spell-list marker (#539)
  and on the eight class-option residue patterns (#548). It warns on a class-option power that is not in the
  canonical form (#544) and on a book membership that no `[[sources]]` row supports (#542).
- **Docs.** The class-option power layout, the trade step convention (#541) and the Contents-address decision are
  recorded; the previous release record is on `main` (#532).

### Known limits

- 133 class-option powers are not in the canonical form, and 338 trades in 182 entries have no resolved class feature (they show their printed names).
- 22 notes of earlier corpora had no marker in the text; v0.1.32 restored or removed them, so the orphan
  warning is at 0.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending (`notice_pending` is unchanged).
