---
title: "Acceptance limits, tracked in [#12](https://github.com/phynics/cast-iron/issues/12)"
date: 2026-11-03
kind: record
---

# Acceptance limits, tracked in [#12](https://github.com/phynics/cast-iron/issues/12)

- Firefox: the Playwright project remains available for debugging, but CI no longer installs or runs it. Playwright's Firefox does not start on the development machine (macOS 27). In [CI run 41697800843](https://github.com/phynics/cast-iron/actions/runs/41697800843), all 37 Firefox tests failed and the step took 57 minutes 55 seconds. The installer remained at its initial storage check. Local verification: `cargo xtask check` passed, and a parsed-workflow check confirmed Firefox is absent from install and test commands while Chromium, mobile Chromium, and informational WebKit remain enabled. CI verification of the updated workflow is pending.
- Physical devices: Android Chromium and iOS Safari, including offline standalone relaunch. Simulator standalone relaunch passed with networking available.
- Persistent storage: headless Chromium reports "not granted". On real browsers the result depends on browser heuristics.
