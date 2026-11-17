---
title: "Two-step release"
date: 2026-11-06
kind: release
---

# Two-step release, 2026-11-06

Pages now serves the canonical corpus library. Both steps followed `docs/releasing.md`, and the
owner approved them in [#127](https://github.com/phynics/cast-iron/issues/127).

| Step | `release` | App commit | Library | Pages run |
|---|---|---|---|---|
| Before | `d48eaa3` | `d48eaa3` | PSRD (old shell) | — |
| 1. Alias-capable shell, PSRD library | `ae032ba` (one-time fast-forward push) | tag `psrd-last-buildable` plus `--psrd` | `vcdd730994d863e50`, 9,990 entries | [42200932251](https://github.com/phynics/cast-iron/actions/runs/42200932251): build and deploy succeeded |
| 2. Canonical corpus library | `ccbfb7c` (`promote-release.py --push`) | `main` `ccbfb7c` (#177, #179, #184) | `vc01cc1c20a652c37` | [42206465391](https://github.com/phynics/cast-iron/actions/runs/42206465391): build and deploy succeeded |

**Superseded.** These changes shipped in the `20e6a31` release above and then in the `1dacf59`
release. `release` points at `1dacf59`.

**CI**
- Prep commit `ae032ba`: [42190040210](https://github.com/phynics/cast-iron/actions/runs/42190040210)
  succeeded on attempt 2, including the PSRD determinism steps. On attempt 1, WebKit shard 3/3
  (informational) hung for over an hour in `setup-e2e` before any test ran, so that job was rerun.
- `main` `ccbfb7c`: full run [42201946869](https://github.com/phynics/cast-iron/actions/runs/42201946869)
  succeeded, with every Chromium and WebKit shard green.

**Local**
- Step 1, in a worktree of the tag:
  - `check-library-upgrade.py` from PSRD to canonical: 9,968 old ids; 0 missing, 0 unresolvable,
    0 alias problems, 0 dangling references.
  - `build-site --psrd` succeeded; `check-publication.py` exit 0; 30 Chromium tests passed (with the
    four documented exclusions).
- Step 2, at `ccbfb7c`:
  - `cargo xtask content` and `build-site` gave `vc01cc1c20a652c37`; `check-publication.py` exit 0.
  - Chromium e2e for canonical-cutover, offline, install, update, shell-update, search and
    reference: 40 passed.
  - A scripted walk of the app served under `/cast-iron/` passed: install, search, entry, offline
    reload, search and legal page while offline, a legacy PSRD id redirecting to `ref/`, spell
    lists, a feat entry, and Books.
  - `cargo xtask published-ids` added 0 ids.

**Live** (headless Chromium against https://phynics.github.io/cast-iron/)
- Fresh profile, after each step:
  - The library installed, an entry opened, and an offline reload worked; search and the legal page
    worked while offline.
  - Every request went to `phynics.github.io`, with 0 console errors and 0 failed requests.
  - After step 2, the legacy id `core-rulebook/spells/fireball` opens `ref/spell/fireball`.
- Existing profile, upgraded from step 1 to step 2:
  - The profile was installed on the step 1 release, with Fireball starred.
  - Settings offered the app update, and Reload applied it. The library update then downloaded.
  - Afterwards, only `cast-iron-content-vc01cc1c20a652c37` remains. The star survived, and its
    bookmark moved to `ref/spell/fireball`. Offline reading works, with 0 page errors.

**Not verified:** physical devices and the accessibility check (#12), Firefox, and WebKit against
the live site.
