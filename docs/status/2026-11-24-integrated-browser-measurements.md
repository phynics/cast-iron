---
title: "Integrated-browser measurements of release 2b56cc1"
date: 2026-11-24
kind: record
---

# Integrated-browser measurements of release 2b56cc1, 2026-11-24

**These are integrated-browser numbers, not iPhone numbers.** The run used the T3 Code integrated browser, a desktop Chromium, not a physical iPhone and not an iOS Simulator. The build host has no iOS Simulator, so the device checks for #423 are still open. Nothing here is a timing, memory or storage figure of an iPhone.

Context (as recorded on [#423](https://github.com/phynics/cast-iron/issues/423), 2026-11-24):

- **Browser:** T3 Code integrated browser, Chromium 157 on Linux.
- **Viewport:** desktop, 1280×800.
- **Profile:** fresh incognito profile.
- **Release SHA:** live release `2b56cc138` (`release` at `2b56cc1`).
- **Content version:** `v3f3d0b708758820d` (corpus v0.1.40).
- **Schema:** 6.
- **Library bytes:** 217,014,990.

## Results

| Measure | Result |
|---|---|
| Install: Download click to a usable Home | about 38 s (upper bound; polled) |
| Storage after install | 220 MB, all in Cache Storage; persistent storage not granted |
| Cold start, installed (reload) | first contentful paint 0.59 s, largest contentful paint 1.94 s; 0 bytes over the network; no third-party requests |
| First search, cold index (`fireball`) | 0.81 s to results |
| Typical searches (`power attack`, `cure light wounds`, `owlbear`, `wizard`), route change included | 0.60–0.96 s |
| Wizard class page | opens in 0.37 s; JS heap 72 MB before and after (WASM memory not included); 1,610 DOM nodes |
| Offline restart | not measured here (the integrated browser has no network toggle); covered by the Playwright offline specs in CI |
| VoiceOver loading behaviour (#419), Safari storage eviction | not measurable in this browser |

## Not measured, and why

- **Offline restart:** the integrated browser cannot switch off the network, so no offline restart was timed. The offline behaviour is covered by the Playwright offline specs (`tests/e2e/tests/offline.spec.ts`) in CI. That is CI evidence, not a measurement from this run.
- **VoiceOver loading behaviour (#419):** needs a screen reader on a real device or simulator. This browser cannot run one.
- **Safari storage eviction:** needs Safari on a device. This browser is Chromium.

## Decisions taken from these numbers

No follow-up issue came from these numbers. Install and search are within what the format work (#264, #183) was meant to decide, and both stay post-v1.

## Still open

The physical-iPhone run for #423 and the VoiceOver check (#419) are not done. The build host has no iOS Simulator, so they need a device or a simulator that is attached to the host first.
