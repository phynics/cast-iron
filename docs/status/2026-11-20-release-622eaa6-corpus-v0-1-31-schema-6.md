---
title: "Release `622eaa6` (corpus v0.1.31, schema 6)"
date: 2026-11-20
kind: release
---

# Release `622eaa6` (corpus v0.1.31, schema 6), 2026-11-20

`release` was promoted from `7c987fc` to `622eaa6e87c4998b1b3e4d7d176fb185f8c210df` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and
search index format 2, as in the previous release. The range is 8 first-parent merges (pull requests #523 to
#530; #523 is the record of the previous release). The release carries one corpus pin, v0.1.31
([pin entry](2026-11-20-corpus-pinned-to-v0-1-31.md)). No file of the app shell changed in the range
(`web/` and `crates/web` are identical between `7c987fc` and `622eaa6`), so the shell stamp is the previous one.

| Field | Value |
|---|---|
| Release commit | `622eaa6e87c4998b1b3e4d7d176fb185f8c210df` |
| Previous release | `7c987fc`, content `v867a87d7bb3ce27d` (schema 6, corpus v0.1.30) |
| Content | `v4192151b29702eab`, schema 6, index format 2, 31,825 entries, 443 publications |
| Library size | 223,207,549 bytes in 292 files, from the live manifest (about 223 MB, 213 MiB; the previous release was 223,245,147 bytes in 292 files) |
| Shell | `0cc32985d5369a1e`, released 2026-11-20 (unchanged from the previous release) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.31`, commit `4ec8d26683ea09b7aaba2336935dcddd179ea264` |

**Local.** The pin entry records `cargo xtask check`, `cargo xtask content`, `build-site`, `size-budget` and
`footnotes.spec.ts` on Chromium for the pin PR. Nothing else was run locally for the release itself. This record is a docs-only
change and needs `cargo xtask check` alone.

**CI.** Main full CI run
[44739978417](https://github.com/phynics/cast-iron/actions/runs/44739978417) at `622eaa6` concluded `success`.
Every job passed: Rust checks, the content compiler, content determinism, the site build, the three required
Chromium shards and all four WebKit shards (informational). The WebKit failure of the previous release
(`mythic.spec.ts:68`) did not recur in this run; it was not run again on its own, so this is one pass, not proof
of a fix. The promotion helper accepted the SHA.

**Deployment.** Pages run
[44759576175](https://github.com/phynics/cast-iron/actions/runs/44759576175) succeeded for
`headBranch: release` at `622eaa6` (build and deploy). It was the newest run for this push and the only one.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-20, with a throwaway Playwright script
(Chromium and WebKit, a fresh profile for each run; the script is not committed). Each browser ran twice. The
first runs reported one failure each in my script, described below the table; the second runs passed every check.
The live `content/manifest.json` reads `schema_version: 6`, `content_version: v4192151b29702eab`,
`entry_count: 31825`, `total_bytes: 223207549`, `corpus_tag: v0.1.31`, 292 files. The live `sw.js` stamps
`SHELL_VERSION = "0cc32985d5369a1e"` and `SHELL_DATE = "2026-11-20"`.

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `v4192151b29702eab`, schema 6, 31,825 entries, corpus tag `v0.1.31`, 223,207,549 bytes in 292 files | pass | pass |
| Library installed with the **Download** button of a fresh profile, reader opens | pass, 14 s (first run 29 s) | pass, 15 s (first run 26 s) |
| `#/entry/ref/spell/summon-monster-2`: no Notes group | pass | pass |
| `#/entry/ref/spell/detect-law`: a "Notes" group with one note, labelled "Note 1", reading "Excludes undead and outsiders, which have their own entries on the table." | pass | pass |
| `#/entry/ref/spell/detect-relations`: the statistics keep `Range: N/A` | pass | pass |
| `#/entry/ref/spell/torrent-of-elemental-rage`: no Range line and no "The original source did not include a range line" text | pass | pass |
| `#/entry/ref/monster/aatheriexa`: the bonus-feat marker links to `footnote-b` | pass | pass |
| Offline: reload on Detect Law with the network off shows the entry | pass | pass |
| Offline: a search for "fireball" lists the Fireball spell | pass | pass |
| Page errors and console errors during the run | none in the second run; see below | none |
| Requests to a host other than the app's | none | none |

The script set the book choice to settled before the install, so every book is active, as the test helper does.
The install used the Download button that a profile without a library shows, not the Settings page.

- **Detect Law in the first runs.** The first runs failed this check because the script read the label with
  `innerText`, which returned the label and the note text as one string. The second runs read `textContent` and the label is
  "Note 1" in both browsers. The page did not change between the runs.
- **One 503 in the first Chromium run.** That run logged one console error, "Failed to load resource: the server
  responded with a status of 503", and the script did not record the URL. The second Chromium run logged the
  URL of every response with a status of 400 or more and saw none, and its console was clean. The first WebKit
  run had no console error. The failing resource and the source of the 503 are unknown. The error did not recur
  on retry.

**Not run.**

- No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The release was checked
  with Chromium and WebKit in a browser only.
- No existing installed profile: no profile with the `7c987fc` library, stars and active books was updated, and no
  app-update or library-update flow was run. The library format is unchanged, and CI's update spec covers schema 6
  to 6 on a modified copy of the candidate library; it is not an upgrade of the published library.
- The live check did not open entries other than those in the table. It did not check the marker styling or the
  hit area of a marker. CI's `footnotes.spec.ts` checks marker styling and desktop hit-area dimensions. The
  touch-specific hit area was not verified.
- `cargo xtask published-ids` was not run for this release. The pin adds no entry and no alias, and the
  published-id guard of `cargo xtask content` passed at the pin.
- The WebKit `mythic.spec.ts` failure of the previous release was not run again on its own.

**Known open issue.** [#121](https://github.com/phynics/cast-iron/issues/121): intermittent WebKit e2e failures.

## Release notes

Changes since the release at `7c987fc` (corpus v0.1.30, 2026-11-20). The release pins corpus v0.1.31.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 223 MB (213 MiB), 38 KB
smaller than before, so download it on Wi-Fi. Install it from **Settings**; the app does not download it by itself
and keeps the old library in use until the new one has passed its hash checks. Keeping stars, recent entries and
active books is the intended behaviour; it was not tested for this release against the published library (see
"Not run"). The app shell has not changed (`0cc32985d5369a1e`), so the app offers no reload.

### Changes to the library

- **Editor-added notes are removed** (corpus v0.1.31). 60 footnotes that only the Pathfinder Data editor added
  (40 pointers, 15 commentary notes, 5 compiled lists) and 15 editorial paragraphs are gone, each recorded in the
  corpus's exclusions. A note stays when Archives of Nethys, the PRD or the printed book carries it
  ([decision](../decisions/2026-11-20-printed-footnotes-stay.md)); no note of unclear origin is removed.
- **Markers of kept notes are back.** The markers of the footnotes that stay are restored in the text. A note on
  Detect Law that copied a note from the detect evil table is removed, so the entry keeps one printed note.
- **Range lines.** A placeholder Range line is dropped: Torrent of Elemental Rage no longer shows a Range line, and
  Detect Relations keeps `Range: N/A`.
- **Counts.** The library keeps 31,825 entries and 443 publications. It has 147,298 resolved links, down from
  147,420, because the removed notes held links; 0 are broken.

### For contributors

- **Licensing docs** say that the notes the corpus keeps from the Pathfinder Data may count as application
  content, and point to the basis in the corpus `LICENSE.md` (#525).
- **Editorial footnotes audit.** The review of the candidate notes against Archives of Nethys and the PRD is in
  `docs/research/` (#526), and the keep test above is a decision record with the audit scripts updated (#528).
- **`corpus-check` warns** when a front-matter field holds an HTML entity such as `&mdash;` (#530).
- **E2E specs.** `mythic.spec.ts` waits for the feat list before it reads it (#524) and checks the whole "Leads to"
  panel (#527). `footnotes.spec.ts` expects one note on Detect Law (#529).
- The previous release record is on `main` (#523), and the pin has a status entry (#529).

### Known limits

- 22 notes in the corpus have no marker in the text (the orphan warning of `corpus-check`). They show in the Notes
  group with no way back to the text.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending (`notice_pending` is unchanged).
