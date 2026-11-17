---
title: "Plain-text corpus, step 6: the upstream import is removed"
date: 2026-11-06
kind: record
---

# Plain-text corpus, step 6: the upstream import is removed, 2026-11-06

Step 6 of [#127](https://github.com/phynics/cast-iron/issues/127). The content compiler,
`cargo xtask`, `sources/`, `scripts/`, CI, and the e2e suite no longer contain the PSRD-Data and
Pf Data adapters, the `--psrd`, `--canonical`, and `--from-upstream` builds, or the upstream
download. The last commit that builds them is the tag `psrd-last-buildable`
(`9800663202f4cb9bcd4fad1af43c5ebf884b4a09`). Decision:
[the upstream import is removed](../decisions/2026-11-06-the-upstream-import-is-removed.md).

Local verification, 2026-11-06 (no CI or deployment claim):

| Check | Result |
|---|---|
| `cargo xtask fetch-sources` | Downloads only the pinned corpus (tree hash `d103cc70...` verified) |
| `generated/content` before and after the removal | Byte-identical (`diff -r`): content version `v6aa587a686a3caf0`, 31,106 entries, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Two corpus builds from scratch | Byte-identical over all of `generated/` (content and reports): `7a6ec44b0e26030711cbbe41851f825e517927a828c6f5323dbf6fc4b797dc4a`. The reports changed shape (no deduplication counts, survey, or supplemental sections), so this hash differs from the step 3 entry |
| `dist/content` after `cargo xtask build-site` | Same tree hash as `generated/content` (`2c6005d6...`) |
| `python3 scripts/check-publication.py` | Exit 0. The registry comparison is replaced by a check that the sixteen core books are present |
| `npm --prefix tests/e2e test -- reference search install books legal --project=chromium` | 45 passed (a sample, not the full suite) |
| `cargo xtask check` | Passed: formatting, clippy, all workspace tests (including the new corpus build integration test), and the WASM build |
| Build from the tag | `cargo xtask fetch-sources && cargo xtask content --psrd` in a checkout of `psrd-last-buildable` gives the PSRD library `vcdd730994d863e50`, unchanged from the step 3 entry |
| `check-library-upgrade.py` from the tag against this branch's `generated/content` | Exit 0: 9,968 old ids, 0 missing, 0 unresolvable, 0 alias problems, 0 dangling references, 0 class-category errors |

Removed: 125 files and about 36,600 lines of Rust, 3,900 lines of Python, and the upstream inputs
and fixtures under `sources/`, `reports/`, and the compiler's test fixtures. Added: about 500 lines of
Rust (the corpus build integration test, the relocated facet, markup, and shard tests, and the
rewritten report writer).

Limitations: this is local evidence. CI has not run the changed content job, and nothing is
deployed. The PSRD release step needs a release commit whose Pages workflow builds with `--psrd`;
`docs/releasing.md` gives the exact preparation and verification, and records the one open
owner decision (how the prep commit reaches `release`). The tier table for Books categories is
checked against the five categories of the pinned corpus; a corpus bump with a new category fails
the `book_tier` test until the table lists it.

### Step 6, review round 1, 2026-11-06

Added after review: the published-id guard (`sources/published-ids.txt`, 41,074 ids: 9,968 PSRD
ids equal to the snapshot at the tag, and the 31,106 `ref/` ids of `v6aa587a686a3caf0`), the
`corpus-check` licensing gate, a CI-enforced `book_tier` corpus test, and an executable PSRD
release procedure in `docs/releasing.md`. Local verification, no CI or deployment claim:

| Check | Result |
|---|---|
| `generated/content` tree hash after the guard | `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3`, `diff -r` against the baseline empty; two builds from scratch identical (`7a6ec44b...dc4a` over all of `generated/`) |
| Guard on the real corpus | `cargo xtask content` passes; with an extra unresolvable id in a copy of the file the compiler exits 1 before writing any output and names the id |
| `cargo xtask corpus-check` on corpus `v0.1.0` with the licensing gate | Exit 0: 439 publications, all with a copyright; 431 covered by a `section_15` notice, 8 by `notice_pending` (Bestiary 6, Faiths of Purity, Goblins of Golarion, Horror Realms, Inner Sea Taverns, Kingmaker Player's Guide, Magic of Thassilon, Serpent's Skull Player's Guide); no exceptions |
| `book_tier` cached-corpus test | Fails with `CI=true` and no cache; prints a skip message locally without one |
| `cargo xtask check`, `check-publication.py`, `cargo xtask build-site` | Passed; `dist/content` tree hash `2c6005d6...` |
| `npm --prefix tests/e2e test -- reference search install books legal --project=chromium` | 45 passed (a sample) |
| PSRD procedure in a worktree of `psrd-last-buildable` | Followed as written: `content --psrd` (`vcdd730994d863e50`), the tag's upgrade check against this branch's `generated/content` by relative path exit 0, the one-line `pages.yml` edit on a prep branch, `build-site --psrd`, `check-publication.py` exit 0, and 30 Chromium tests pass with the four documented exclusions. The full suite does not load against the PSRD library (`entry-header.spec.ts`); six tests of the first sample fail on missing categories. `git merge -s ours` of the prep commit into `main` leaves the tree identical and makes the prep commit and `release` ancestors |
