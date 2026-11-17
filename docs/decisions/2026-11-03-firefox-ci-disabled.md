---
title: "Firefox CI disabled"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: process
---

# Firefox CI disabled, 2026-11-03

- Stop installing and running Firefox in CI. In [run 41697800843](https://github.com/phynics/cast-iron/actions/runs/41697800843), all 37 Firefox tests failed, mostly after the shared 90-second installation wait. The sequential suite took 57 minutes 55 seconds. Although informational, the step delayed completion of the required browser job. Keep the Playwright project for targeted debugging. Re-enable CI coverage after the installer hang is diagnosed and verified. Chromium, mobile Chromium, and informational WebKit coverage remain enabled.
