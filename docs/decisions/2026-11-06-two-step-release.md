---
title: "Two-step release"
date: 2026-11-06
status: current
superseded_by: []
issues: [127]
prs: []
area: releases
---

# Two-step release, 2026-11-06

Refs [#127](https://github.com/phynics/cast-iron/issues/127). The owner approved the release and chose how the PSRD-first prep commit reaches `release`.

- **The PSRD step was a one-time fast-forward push.** `promote-release.py` refuses the prep commit (`ae032ba`, the tag `psrd-last-buildable` plus one `pages.yml` line that adds `--psrd`) because it was not on `main` at the time and has no `main` CI run for that SHA. The owner chose `git push origin ae032ba:refs/heads/release` over an exception in the helper. `release` was at `d48eaa3`, an ancestor of the prep commit, so nothing was reset or forced. The helper is unchanged, and every later promotion uses it.
- **The release history reaches `main` through an `ours` merge.** #184 merged `release` into `main` with `-s ours`, so `main`'s tree did not change and the `--psrd` line never reached `main`. That merge commit makes `release` an ancestor of `main`, so the canonical promotion (`ccbfb7c`) was a normal fast-forward with the helper.
- **The owner's hand check before the canonical step was done by the agent alone.** On 2026-11-06 the owner chose "agent checks, then promotes": a local walk of the corpus-built app and a Chromium e2e sample on `ccbfb7c`, then promotion without waiting for the owner. Physical devices (#12) remain open.
