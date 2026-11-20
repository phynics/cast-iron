---
title: "Release `7c987fc` (corpus v0.1.30, schema 6)"
date: 2026-11-20
kind: release
---

# Release `7c987fc` (corpus v0.1.30, schema 6), 2026-11-20

`release` was promoted from `58ca785` to `7c987fcf3db8f8ab0d2c72dfe621369edbe16305` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and
search index format 2, as in the previous release. The range is 4 first-parent merges (pull requests #518, #519,
#521 and #522). The release carries one corpus pin, v0.1.30 ([pin entry](2026-11-20-corpus-pinned-to-v0-1-30.md)).

| Field | Value |
|---|---|
| Release commit | `7c987fcf3db8f8ab0d2c72dfe621369edbe16305` |
| Previous release | `58ca785`, content `vb23c2dd0a96f4c21` (schema 6, corpus v0.1.29) |
| Content | `v867a87d7bb3ce27d`, schema 6, index format 2, 31,825 entries, 443 publications |
| Library size | 223,245,147 bytes in 292 files, from the live manifest (about 223 MB, 213 MiB; the previous release was 223,592,777 bytes in 292 files) |
| Shell | `0cc32985d5369a1e`, released 2026-11-20 (previous `3e99cf1efd636e7c`) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.30`, commit `fb77db5d9e7ee4bb7cedde53b4b295095d560044` |

**CI.** Main full CI run
[44642077086](https://github.com/phynics/cast-iron/actions/runs/44642077086) at `7c987fc` concluded `success`.
The content compiler, Rust checks, content determinism, the site build and the three required Chromium shards
passed, and so did WebKit shards 1, 3 and 4 of 4. **WebKit shard 2/4 (informational) failed one test:**
`mythic.spec.ts:68`, "Mythic Adventures active: the base stacks one expanded version", at a
`toBeGreaterThanOrEqual` assertion (`Expected: >= 0`). WebKit is informational, so the run is green. The failure
was not investigated or fixed for this release. No second full suite was run. The promotion helper accepted the
SHA.

**Deployment.** Pages run
[44645225960](https://github.com/phynics/cast-iron/actions/runs/44645225960) succeeded for
`headBranch: release` at `7c987fc` (build and deploy). It was the only run for this push, so it is also the
final successful run. The live `content/manifest.json` reads `schema_version: 6`,
`content_version: v867a87d7bb3ce27d`, `entry_count: 31825`, `total_bytes: 223245147`, `corpus_tag: v0.1.30`. The
live `sw.js` stamps `SHELL_VERSION = "0cc32985d5369a1e"` and `SHELL_DATE = "2026-11-20"`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-20, with a throwaway Playwright script
(Chromium and WebKit, a fresh profile each; the script is not committed). Both runs passed on the first attempt.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `v867a87d7bb3ce27d`, schema 6, 31,825 entries, corpus tag `v0.1.30`, 223,245,147 bytes in 292 files | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens | pass, 25 s | pass, 22 s |
| `#/entry/ref/spell/summon-monster-2`: no Notes group and no footnote markers | pass | pass |
| `#/entry/ref/spell/detect-law`: a "Notes" group (complementary landmark) with two notes | pass | pass |
| `#/entry/ref/monster/aatheriexa`: the `B` after the bonus feat is a superscript link to `?a=footnote-b`, smaller than the line (11.9 px against 17 px), weight 400, no underline, the colour of the notes; the note reads "Bonus feat." | pass | pass |
| `#/entry/ref/magic-wondrous/figurine-of-wondrous-power-basalt-dragon`: a "Sources:" line with 15 books and pages (Ultimate Equipment p. 297-298 first, Mythic Adventures p. 151 last); no Notes group and no markers | pass | pass |
| Page errors during the run | none | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper does.
The install used the Download button that a profile without a library shows, not the Settings page.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The release was checked
  with Chromium and WebKit in a browser only.
- No upgrade from the previous release: no profile with the `58ca785` library and stars was updated, and no
  offline reload of this release was run. The library format is unchanged, and CI's update spec covers schema 6
  to 6 on a modified copy of the candidate library; it is not an upgrade of the published library.
- The live check did not open any other entry whose page citations became sources, other
  stat-block notes, the touch hit area of a marker, or hover and keyboard focus on a marker. `footnotes.spec.ts`
  in the CI run covers the marker styling, name and links.
- `cargo xtask published-ids` was not run for this release. The pin adds no entry and no alias, and the
  published-id guard of `cargo xtask content` passed at the pin.
- The WebKit failure above, `mythic.spec.ts:68`, was not reproduced or run again.

**Known open issue.** [#121](https://github.com/phynics/cast-iron/issues/121): intermittent WebKit e2e failures.

## Release notes

Changes since the release at `58ca785` (corpus v0.1.29, 2026-11-19). The release pins corpus v0.1.30.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 223 MB (213 MiB), 0.3 MB
smaller than before, so download it on Wi-Fi. Install it from **Settings**; the app does not download it by itself
and keeps the old library in use until the new one has passed its hash checks. Keeping stars, recent entries and
active books is the intended behaviour; it was not tested for this release against the published library (see
"Not run"). The app shell has changed (`0cc32985d5369a1e`): accept **Reload** when the app offers it.

### Changes to the library

- **Page citations are sources** (corpus v0.1.30). 438 footnotes in 70 entries only said where a rule is printed
  (for example "Ultimate Equipment pg. 297"). They held 484 book-and-page citations. 480 of those are now on the
  entry's **Sources** line (as new rows, as pages added to a row, or as pages the row already had), and the
  footnote blocks and their markers are gone. The Figurine of Wondrous Power (Basalt Dragon) entry is an example: its Sources line
  lists 15 books with pages, and it carries no note for them. 4 citations to Pathfinder #30 and #43 could
  not be checked against print and are recorded in the corpus's `exclusions.toml` instead.
- **Notes that no source carries are removed** (corpus v0.1.30). 81 footnotes and 60 editorial paragraphs that
  neither Archives of Nethys nor the PRD carries are gone, each recorded in the exclusions. Summon Monster II no
  longer shows notes. Three footnote blocks with converter residue are split into one block per note, 27 duplicate note
  anchors in 6 entries are renamed, and 20 markers that pointed at nothing are removed.
- **Counts.** The library keeps 31,825 entries and 443 publications. It has 147,420 resolved links, down from
  147,942, because the removed notes held links; 0 are broken.

### Changes to what you see

- **Quieter note markers.** A footnote marker that links to its note, such as the mark after a monster's bonus feat
  (the `B`), is now a small superscript in the muted colour of the notes, in regular weight and without an underline, so it no longer
  reads as a bold part of the feat name. It is still a link to its note; hover and keyboard focus bring back the
  accent colour and an underline. A screen reader announces the `B` of a bonus feat as "Bonus feat, note B" and
  another marker by its own label, for example "Note 1". Plain superscripts in tables and stat rows keep their
  earlier look. Its tap area grows up, down and to the right, not onto the feat name, and is taller
  on a touch screen. (#519)

### For contributors

- **`corpus-check` warns about two footnote problems** without failing: a footnote with no marker in the entry
  body, and converter residue such as `\|X~`. It also counts a marker in a section heading. Both can become errors
  once the corpus clean-up is done. The documentation of the corpus format describes them. (#521)
- **`footnotes.spec.ts`** tests the missing-marker case on Detect Law, because the pin removed the notes of Summon
  Monster II. The assertions are the same. (#522)
- The previous release record is on `main` (#518), and the pin has a status entry (#522).

### Known limits

- 110 notes in the corpus have no marker in the text (mostly monster stat-block notes, plus the Detect spells,
  Minor Creation and Type G). They show in the Notes group with no way back to the text.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending (`notice_pending` is unchanged).
