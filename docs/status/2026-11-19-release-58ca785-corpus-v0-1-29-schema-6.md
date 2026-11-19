---
title: "Release `58ca785` (corpus v0.1.29, schema 6)"
date: 2026-11-19
kind: release
---

# Release `58ca785` (corpus v0.1.29, schema 6), 2026-11-19

`release` was promoted from `52086b3` to `58ca785c56f59ebd11925161ed395440d8c24d5a` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and
search index format 2, as in the previous release. The range is 13 first-parent merges (pull requests #500 to
#503, #505, #506, #508, #509, #510, #512, #513, #515 and #516). The release carries two corpus pins, v0.1.28 and
v0.1.29, which this entry lists under "New in the library".

| Field | Value |
|---|---|
| Release commit | `58ca785c56f59ebd11925161ed395440d8c24d5a` |
| Previous release | `52086b3`, content `v6bd1da0aee54f66d` (schema 6, corpus v0.1.27) |
| Content | `vb23c2dd0a96f4c21`, schema 6, index format 2, 31,825 entries, 443 publications |
| Library size | 223,592,777 bytes in 292 files, from the live manifest (about 224 MB, 213 MiB; the previous release was 223,171,347 bytes in 292 files) |
| Shell | `3e99cf1efd636e7c`, released 2026-11-19 |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.29`, commit `d9bff95` ([v0.1.28 pin entry](2026-11-19-corpus-pinned-to-v0-1-28.md), [v0.1.29 pin entry](2026-11-19-corpus-pinned-to-v0-1-29.md)) |

**CI.** Main full CI run
[44606423367](https://github.com/phynics/cast-iron/actions/runs/44606423367) succeeded at `58ca785` with every
job green: the content compiler, Rust checks, content determinism, the site build, three required Chromium
shards and four informational WebKit shards. An earlier run on `main`,
[44539453564](https://github.com/phynics/cast-iron/actions/runs/44539453564) at `3919b89`, concluded
**cancelled**; [#508](https://github.com/phynics/cast-iron/pull/508) reports that Chromium shard 3/3 failed on a
page error in `books-share.spec.ts`, which the same PR fixed in the app. [#510](https://github.com/phynics/cast-iron/pull/510)
reports that WebKit shard 1/3 hit its 15-minute limit and was cancelled, and split WebKit into four shards. The
promotion helper requires a successful main run on the exact candidate SHA; it accepted this one. The owner
accepted this run as the promotion testing; no second full suite was run.

**Deployment.** Pages run
[44610016618](https://github.com/phynics/cast-iron/actions/runs/44610016618) succeeded for
`headBranch: release` at `58ca785` (build and deploy). It was the only run for this push, so it is also the
final successful run. The live `content/manifest.json` reads `schema_version: 6`,
`content_version: vb23c2dd0a96f4c21`, `entry_count: 31825`, `total_bytes: 223592777`, `corpus_tag: v0.1.29`. The
live `sw.js` stamps `SHELL_VERSION = "3e99cf1efd636e7c"` and `SHELL_DATE = "2026-11-19"`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-19, with a throwaway Playwright script
(Chromium and WebKit, a fresh profile each; the script is not committed). Both runs passed on the first attempt.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `vb23c2dd0a96f4c21`, schema 6, 31,825 entries, corpus tag `v0.1.29`, 223,592,777 bytes in 292 files | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens | pass, 26 s | pass, 25 s |
| `#/entry/ref/feat/shield-master`: the prerequisites line reads "Improved Shield Bash, Shield Proficiency, Shield Slam, Two-Weapon Fighting, base attack bonus +11" and Two-Weapon Fighting is a link | pass | pass |
| `#/entry/ref/domain/death` renders with the title "Death" and its granted-powers text | pass | pass |
| `#/books/manage` has the **Share these books or enter a code** section (once); `#/books` does not have it | pass | pass |
| `#/entry/ref/spell/summon-monster-2`: a "Notes" group (complementary landmark) after the text with two notes, and no level-six heading in the text | pass | pass |
| Page errors during the run | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper does.
The install used the Download button that a profile without a library shows, not the Settings page.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The owner gave the go for
  this promotion on 2026-11-19 ("cut the next pin and promote that") with Chromium and WebKit browser evidence in
  place of the device, as for the previous release ([#441](https://github.com/phynics/cast-iron/issues/441#issuecomment-6958680276)).
- No upgrade from the previous release: no profile with the `52086b3` library and stars was updated, and no
  offline reload of this release was run. The library format is unchanged, and CI's update spec covers schema 6
  to 6 on a modified copy of the candidate library; it is not an upgrade of the published library.
- The live check did not open the other 16 new domains, the other 85 feat prerequisite changes, an old
  `#/books?set=<code>` link, the Undo behaviour, or the marker links in a monster's notes. Specs in the CI run
  cover the share section, its old links and the Notes group (`books-share.spec.ts`, `footnotes.spec.ts`).
- The Android and iOS behaviour of the Archives of Nethys link was not checked on a device. See the decision
  `2026-11-19-aon-link-opens-where-the-platform-decides`.

**Known open issue.** [#121](https://github.com/phynics/cast-iron/issues/121): intermittent WebKit e2e failures.
It stays open; this release changes tests around it ([#502](https://github.com/phynics/cast-iron/pull/502),
[#505](https://github.com/phynics/cast-iron/pull/505)) and not the app.

## Release notes

Changes since the release at `52086b3` (corpus v0.1.27, 2026-11-19). The release pins corpus v0.1.28 and v0.1.29.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 224 MB (213 MiB), 0.4 MB
larger than before, so download it on Wi-Fi. Install it from **Settings**; the app does not download it by itself
and keeps the old library in use until the new one has passed its hash checks. Keeping stars, recent entries and
active books is the intended behaviour; it was not tested for this release against the published library (see
"Not run"). The app shell has changed (`3e99cf1efd636e7c`): accept **Reload** when the app offers it.

### New in the library

- **17 cleric domains** (corpus v0.1.28). Artifice, Chaos, Community, Darkness, Death, Evil, Glory, Good,
  Knowledge, Law, Liberation, Magic, Nobility, Repose, Rune, Strength and Trickery now ship as entries, checked
  against print. The converter had left them out. The library has 31,825 entries, up from 31,808. Each carries the
  Open Game License reference and the Community Use notice reference, as the Animal domain already does. Death and
  Repose also cite the Golarion Day blog post and include its Pharasma-Friendly adjustments. The 13 subdomain
  records that point to these domains are not new entries.
- **Protection and Animal** (corpus v0.1.28). The usage text of the two domain powers matches the printed
  rounds per day.
- **Feat prerequisites** (corpus v0.1.29). On 86 feats the prerequisite text and the prerequisite links disagreed;
  both now follow print. Shield Master and Shield Slam regain Two-Weapon Fighting. Rapid Shot, Far Shot, Manyshot and
  Shot on the Run lose a wrong Precise Shot link. Deflect Arrows loses a wrong Shield Focus link. Ledge Walker points
  at the dwarf racial trait. The library has 18 more resolved links than before the v0.1.29 pin (147,942 against
  147,924), and none broken.

### Changes to what you see

- **Notes on entries.** A footnote is no longer shown as rules text. Notes now sit in one quiet **Notes** group at
  the end of an entry, in smaller, muted type, in light and dark. A marker in the text (for example ^1^) links to
  its note, and the note links back to each marker that cites it. A note whose marker is missing from the text, as
  with Summon Monster II, shows its label and text without a way back. A note that a monster's stat block prints also
  moves to Notes, with markers that link to it and back; the stat-block lines after it stay in the text. An old link to `#footnote-1` still lands on
  the note. The Notes group is not in the "On this page" list. (#509)
- **Book sharing moved to Manage books.** The **Share these books or enter a code** section (the code, Copy code,
  Copy link, Enter a code with Review, the question, Undo) is no longer on Books. It sits on **Manage books**, after
  the presets and before the book groups, as one collapsed section. Books shows the count line, the Manage books
  link and the first group. New share links read `#/books/manage?set=<code>`; an old `#/books?set=<code>` link
  still works, opens the same question on Manage books and rewrites its address. (#506)
- **Undo on Manage books.** The presets and the share section each keep a status line with Undo, and a line
  disappears as soon as any later change succeeds, so only one Undo is live. An Undo that was tapped while another
  change was still saving does nothing, so it cannot put back books that the newer change turned off. A save that
  fails changes no line. (#506)
- **Answered share links stay answered.** A share link whose question you had answered or cancelled no longer
  reopens it when you return to its history entry with Back and Forward after visiting Settings and Quick search.
  The marks last until the app reloads; a leftover `?set=` entry then asks again. (#512)

### Fixes

- **A failed save leaves no stray error.** When the browser aborted a database transaction while a write was still
  pending (a full disk, or a store giving up), the app left an unhandled error behind next to the save-failure
  message it already showed. The bridge now observes it. (#508)

### For contributors

- **Resolved rows can supersede rejected rows** in a corpus's `exclusions.toml`, not only excluded ones. The coverage
  report lists them in a "Resolved records" section and no longer counts a superseded rejected record as rejected. The
  report type gains an optional `resolved` list, which is absent when empty; `SCHEMA_VERSION` stays 6. (#500)
- **Full CI runs WebKit in four shards,** so a shard stays under its time limit. `books-share.spec.ts` no longer
  navigates to the address it is on, which reloaded the page in WebKit only. (#510)
- **Test races.** The console guard closes its window only when a new document commits, not on a same-document
  change of the old page, and the class-sections specs cold-load their addresses. These are test changes; the app
  is unchanged. (#502, #505)
- **Archives of Nethys link.** On an installed Android app the link opens in a Chrome Custom Tab laid over the app;
  the platform decides this, and the app keeps a plain `target="_blank"` link. A test checks that the link opens a
  separate page, and a decision records the platform behaviour. (#515)
- **Footnote audit.** A research report, two data files and three scripts audit the 1,284 footnote blocks of the
  corpus against print, Archives of Nethys and the PRD. It changes no corpus entry and removes no note. (#513)
- The previous release record is on `main` (#501), and the two corpus pins have status entries (#503, #516).

### Known limits

- The Notes group takes only the paragraph after a footnote heading. A marker inside a table or a stat row stays a
  plain superscript and does not link.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending (`notice_pending` is unchanged in both pins).
