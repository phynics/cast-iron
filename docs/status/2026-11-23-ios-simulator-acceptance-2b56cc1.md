---
title: "iOS Simulator acceptance of release 2b56cc1"
date: 2026-11-23
kind: record
---

# iOS Simulator acceptance of release 2b56cc1, 2026-11-23

**This is simulator evidence, not device evidence.** Simulator timings, memory and storage figures are not
numbers of a physical iPhone. Storage eviction (Part 7 of the checklist) and VoiceOver (Part 5) were not
tested. The offline restart (Part 6) was not run, because the simulator could not cut its network.

The session followed [the measurement checklist](../research/iphone-measurement.md) for
[#423](https://github.com/phynics/cast-iron/issues/423) and the device check of
[#514](https://github.com/phynics/cast-iron/issues/514), under the owner decision of 2026-11-23 that the v1
device checks run on simulators. It drove the simulator through the T3 Code device tools (`agent-device`).
No Android Emulator was available on the host, and the owner then limited the session to iOS.

## Setup

| | |
|---|---|
| Simulator | iPhone 18 Pro simulator, iOS 27.2 (build 24B114) |
| Browser | Mobile Safari 27.2. User agent: `Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.2 Mobile/15E148 Safari/604.1` |
| Host CPU | Apple M1 |
| Network | The host's ordinary network; the simulator has no network setting of its own |
| Site | The live site, `https://phynics.github.io/cast-iron/`, read-only. No change to `release` or Pages |
| Build, as Diagnostics shows it | `app: Released 23 Nov 2026 (114f05cc…)`, shell stamp `114f05ccdf222d2a`, which is `release` at `2b56cc1`. Library `v0.1.40 (v3f3d0b70…)`, library id `v3f3d0b708758820d`, content format 6, 31,742 entries, 280 files, 207 MB |

The build matched the expected release: corpus `v0.1.40`, content version `v3f3d0b708758820d`, schema 6.

## Starting state: two PSRD-era installs

The simulator still held both copies from the 2026-11-04 simulator run
([iphone-journeys.md](../research/iphone-journeys.md)). Both Part 1 and Part 2 are therefore **upgrades
from a PSRD-era library**, not fresh installs. Each upgrade fetched all 280 files ("0 already there"), so the
download figures are those of a full install.

- **Safari tab:** library `v171f0348e5717ee0`, content format 3, 9,967 entries, 89 files, 59.1 MB, installed
  2026-11-03. Old shell with "Update ready." and a bookmark to Races.
- **Home Screen app** (standalone): the same old library and an old shell, with a bookmark to Special
  Attacks.

## Results

| Step | Result |
|---|---|
| Part 1, install in a Safari tab (upgrade) | **Passed.** Reload took the new shell, which offered `v0.1.40 · 207 MB` and "Download update". Total 3 min 26 s: scan 20 ms, download 3 min 25 s (about 1.0 MB/s), verify 1.0 s, activate 74 ms. Persistent storage not granted. Storage used 330 MB of 38.4 GB just after activation. The Races bookmark became a star and opens Races (Core Rulebook) |
| Part 2, install in the Home Screen app (upgrade) | **Passed.** Total 2 min 11 s: scan 40 ms, download 2 min 10 s (about 1.6 MB/s), verify 1.1 s, activate 93 ms. Persistent storage **granted**. Storage used 271 MB after activation, then 211 MB later in the session. The Special Attacks bookmark became a star. In both installs the last file (about 66 MB) held the progress at 68% for about a minute |
| Part 3, cold starts in the Home Screen app | **Passed.** Home on screen, as the app records it (from page start): launch #3 2.4 s, #4 1.6 s, #5 1.7 s, #6 1.8 s. Launch #2, the first after a simulator reboot, took 5.8 s, with app code running at 3.6 s. Time from the icon tap until a snapshot saw the categories: 12.5 s, 9.1 s, 9.7 s, 9.8 s. These wall-clock times include about 2 s per snapshot of the automation, so they are an upper bound and not a stopwatch count |
| Part 4, search (launch #7) | **Passed.** Home at 2.4 s. Search index read 91 ms, decode 77 ms, 65.7 MB, ready at 2.5 s. "Loading search index…" was not seen. Quick search: first suggestions painted after 90 ms, the `grapple` run after 8 ms. Full results: first search (`grapple`, 1,265 results) painted after 396 ms (work 105 ms, engine 9 ms, 7 ms wait for the index). The 11 later runs: median 281 ms, slowest 504 ms, engine median 107 ms. The automation typed each query in one fill, so several later runs are 1-character runs with 29,000 to 31,000 results |
| Part 4, Wizard class page | **Passed.** On screen after 1.0 s on first open, then 311 to 573 ms on three later opens; 1,690 elements, 22,250 characters. Scrolling to the bottom and back to the top showed no blank page, no jump and no stutter in the screenshots. The launch number stayed #7 for the whole session: no suspected restart. WebAssembly memory high-water mark 181 MB after Part 4, 247 MB at the end of the session |
| Part 4, two links and Back | **Partly run.** The Wizard spell list link opened. Back could not be tested: the Home Screen app has no Back control, and the automated edge swipe once did nothing and once landed on Home. Back needs a manual check |
| Part 5, VoiceOver | **Not run.** The device tools cannot turn VoiceOver on in the simulator |
| Part 6, offline restart | **Not run.** On an iOS simulator, the airplane and Wi-Fi settings of the device tools only change the status bar, and the simulator uses the host's network. "Verify library", run online: "Library verified. All 280 files match their checksums (207 MB)." |
| Part 7, storage eviction | **Not run.** It needs a week of Safari use |
| #514, `Copy link` in the Home Screen app | **Passed.** On Fireball, `Copy link` showed "Link copied." and the clipboard held `https://aonprd.com/SpellDisplay.aspx?ItemName=Fireball` (the clipboard held a marker text before the tap) |
| #514, AoN link in the Home Screen app | **Passed.** `View on Archives of Nethys` opened the AoN Fireball page in an in-app browser sheet over the app (aonprd.com, with a close button). Closing it returned to the same Fireball entry at the same scroll position |
| #514 on Android | **Not run.** No Android Emulator on the host |

## Observations

- **Update screen after a check by the old shell.** In the Home Screen app, the first Settings screen of the
  new shell showed the update as `v3f3d0b708758820d · 207 MB` with "Check first, then download it." and no
  Download button. After "Check now" it showed `v0.1.40 · 207 MB` and "Download update". The Safari tab
  showed the second form at once. This is the path that the pre-promotion upgrade check did not exercise:
  the old shell had checked for a library before the app update. It costs one extra tap; nothing failed.
- **Cold start after a reboot.** The first tap on the old Home Screen app after the simulator restarted
  showed a black screen for about 20 seconds before the page drew. The simulator had just been restarted
  after it stopped responding, so this is not counted as an app figure.

No failure of the app was seen, and no issue was filed.

## Not shown by a simulator

Physical-iPhone timings, memory and storage; the memory of Safari's own process; storage eviction; VoiceOver;
a true offline restart; Android.
