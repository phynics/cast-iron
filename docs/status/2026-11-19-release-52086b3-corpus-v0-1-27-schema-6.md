---
title: "Release `52086b3` (corpus v0.1.27, schema 6)"
date: 2026-11-19
kind: release
---

# Release `52086b3` (corpus v0.1.27, schema 6), 2026-11-19

`release` was promoted from `1324e1d` to `52086b316ca13e4d118fedf0424197627b688aa1` with
`scripts/promote-release.py --push`, a fast-forward push. The library format does not change: schema 6 and
search index format 2, as in the previous release. The range is 16 first-parent merges (pull requests #482
and #484 to #495, #497 to #499).

| Field | Value |
|---|---|
| Release commit | `52086b316ca13e4d118fedf0424197627b688aa1` |
| Previous release | `1324e1d`, content `vb4ec72f397a03202` (schema 6, corpus v0.1.26) |
| Content | `v6bd1da0aee54f66d`, schema 6, index format 2, 31,808 entries, 443 publications |
| Library size | 223,171,347 bytes in 292 files, from the live manifest (about 223 MB, 213 MiB) |
| Shell | `26f84fe929f2d360`, released 2026-11-19 |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.27`, commit `65f3967` ([pin entry](2026-11-18-corpus-pinned-to-v0-1-27.md)) |

**CI.** Main full CI run
[44481627642](https://github.com/phynics/cast-iron/actions/runs/44481627642) succeeded at `52086b3`
with every job green, all three WebKit shards included. An earlier run,
[44422502000](https://github.com/phynics/cast-iron/actions/runs/44422502000) at `3b79760`, concluded
**cancelled**: WebKit shard 3 failed, shard 2 was cancelled and shard 1 passed
([#477](https://github.com/phynics/cast-iron/issues/477)). The promotion helper requires a successful main CI
run on the exact candidate SHA, so it refused that candidate. A rerun of the failed jobs failed the same way.
[#499](https://github.com/phynics/cast-iron/pull/499) fixed the racing test, and the run above is the new one.
The owner accepted this run as the promotion testing; no second full suite was run.

**Deployment.** Pages run
[44484570445](https://github.com/phynics/cast-iron/actions/runs/44484570445) succeeded for
`headBranch: release` at `52086b3` (build, publication screen, deploy). It was the only run for this push, so
it is also the final successful run. The live `content/manifest.json` reads `schema_version: 6`,
`content_version: v6bd1da0aee54f66d`, `entry_count: 31808`, `corpus_tag: v0.1.27`. The live `sw.js` stamps
`SHELL_VERSION = "26f84fe929f2d360"` and `SHELL_DATE = "2026-11-19"`.

**Live check** on `https://phynics.github.io/cast-iron/`, 2026-11-19, with a throwaway Playwright script
(Chromium and WebKit, a fresh profile each; the script is not committed).

| Check | Chromium | WebKit |
|---|---|---|
| Shell loads (title "Cast Iron") | pass | pass |
| `content/manifest.json`: content `v6bd1da0aee54f66d`, schema 6, 31,808 entries, 223,171,347 bytes | pass | pass |
| Library installed from Settings (**Download**), reader opens | pass, 28 s | pass, 23 s |
| `#/entry/ref/pclass/thought-thief-arcane-trickster` renders "Thought Thief (Arcane Trickster)"; footer reads "Hodgepodge from the Grand Lodge! © 2019, Paizo Inc.; Author: Luis Loza. Used under Paizo's Community Use Policy." | pass | pass |
| `#/legal` shows **Sources using the Community Use Policy**: Hodgepodge from the Grand Lodge! and The Gauntlet Will Be Ours! (Open Game License and Community Use Policy); More Magic of Thassilon, Golarion Day, Ultimate Cantrips (Community Use Policy only) | pass | pass |
| Page errors during the run | none | none |

Both runs set the book choice to settled before the install, so every book is active, as the test helper does.
The first WebKit run passed every check above and then failed at its own full-page screenshot of the Legal page
(over 32,767 px tall); the script was changed to a viewport screenshot and WebKit was run again with the result
shown. The first Chromium and WebKit attempts also failed on a wrong wait in the script, before the entry check;
that was the script, not the site.

**Not run.** No check on the real iPhone ([#423](https://github.com/phynics/cast-iron/issues/423)). The
owner gave the go for this promotion on 2026-11-19 with Chromium and WebKit browser evidence in place of
the device, as for the previous release ([#441](https://github.com/phynics/cast-iron/issues/441#issuecomment-6958680276)). No upgrade-from-previous-release acceptance was run for this release:
the library format is unchanged, and CI's update spec covers schema 6 to 6. That spec updates a modified copy
of the candidate library and checks a star and a disabled book; it is not an upgrade of the published library.

**Known open issue.** [#121](https://github.com/phynics/cast-iron/issues/121): WebKit can raise
`Context is stopped` around navigation. Unchanged by this release.

## Release notes

Changes since the release at `1324e1d` (corpus v0.1.26, 2026-11-17). The release pins corpus v0.1.27.

### Upgrading

The library format does not change (schema 6, search index 2). The new library is about 223 MB (213 MiB), the
same size as before, so download it on Wi-Fi. Install it from **Settings**; the app does not download it by
itself and keeps the old library in use until the new one has passed its hash checks. Keeping stars, recent entries
and active books is the intended behaviour. CI's update spec checks a star and a disabled book on a modified copy of
the new library, but the upgrade from the published library was not tested for this release.

### New in the library

- **Paizo blog sources.** Eight records that were held back before now ship, each on its own
  licence basis: the archetype Thought Thief (Arcane Trickster) under Paizo's Community Use Policy only (its
  publication, Hodgepodge from the Grand Lodge!, lists both licences, but the entry does not); six cantrips from
  Ultimate Cantrips (Breeze, Drench, Jolt, Penumbra, Root, Scoop) under the Community Use Policy only; and the race
  Primitive Human under the Open Game License, from its source. The library
  has 31,808 entries, up from 31,800.
- **Four new books.** Hodgepodge from the Grand Lodge!, Golarion Day: Other Gods and New Subdomains, Ultimate
  Cantrips and The Gauntlet Will Be Ours!. The library lists 443 books, up from 439. Golarion Day adds Scalykind
  and Void as book members; The Gauntlet Will Be Ours! also places Primitive Human under Rare Races.
- **Web supplements.** The four new books are web supplements, and More Magic of Thassilon moves into the same
  category. The category "Web supplement" sits under Other on the Books page. The four new web supplements list
  their entries without chapters; More Magic of Thassilon still has one chapter. (#495)

### Changes to what you see

- **Licence on entries.** An entry footer now follows the entry's own basis: "Open Game License content." where the
  entry names it, "Used under Paizo's Community Use Policy." for an entry that rests only on that policy.
- **Legal page.** A new section, **Sources using the Community Use Policy**, lists each source under that policy and
  says whether it is "Community Use Policy only" or also under the Open Game License. A source under the policy alone
  is no longer listed as a publication with a pending Open Game License notice.
- **Wide entry pages.** The layout of a wide entry page is declared in one place. The computed grid is the same, so
  no visible change is intended. (#482)

### Fixes

- **Deadeye Devotee.** The page now cites only Hodgepodge from the Grand Lodge! and has the archetype relation to
  Arcane Archer. It is no longer a Core Rulebook member, so Core Rulebook has no archetype members. (#483, #489)

### For contributors

- `cargo xtask check` now fails when the Archives of Nethys coverage report in `docs/research/` is stale against the
  pinned corpus (#488, #490).
- The corpus can mark a source as Community Use only, and an exclusion row can be superseded by a later resolved row
  (#491, #493).
- The publication screen checks an entry's Open Game License reference. It accepts an entry without one only when the
  entry has a Community Use reference, cites at least one source, and every cited source lists Community Use among its licence bases
  (#495); it has unit tests (#498). A share-code test now documents that codes cover the highest slot in the library
  catalog, which was already the behaviour (#494).
- A WebKit test race around the install button and update check is fixed (#499), and the specs that the v0.1.27 pin
  broke are repaired (#497).
- The previous release record, the deferred upgrade acceptance from `6365dba` and the conditional-view assessment
  (#448) are recorded (#484, #486, #487). The AoN coverage report is published (#488). Four book slots are reserved
  ahead of the corpus: slot 439 (#489) and slots 440 to 442 (#492).

### Known limits

- The four new web supplements have no chapter tree, so the book page lists their entries flat.
- Hodgepodge from the Grand Lodge!, Kingmaker Player's Guide and Serpent's Skull Player's Guide still have a
  Section 15 notice pending.
