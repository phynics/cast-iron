---
title: "The daily update check is opt-out and same-origin"
date: 2026-11-06
status: partly-superseded
superseded_by: ["2026-11-23-settings-has-seven-sections-and-one-update-row"]
issues: [206]
prs: []
area: install-offline
---

# The daily update check is opt-out and same-origin, 2026-11-06

Refs [#206](https://github.com/phynics/cast-iron/issues/206). Owner decision, recorded in the issue on 2026-11-06. This narrows `docs/spec.md` line 269 ("No app-initiated network requests occur during ordinary reference use"), line 279 ("Content updates are optional and explicitly initiated while online"), and AGENTS.md rule 1.

- **What is allowed.** The app may fetch its own `content/manifest.json` without a tap: same origin only, never a third-party host, and never any other file. It never downloads the library, never activates anything, and never asks for persistent storage. Downloading still needs **Download update**.
- **When.** At app start and again when the document becomes visible, but only while online and at most once every 24 hours. A start event and a visibility event cannot run at the same time: the in-flight flag is set before the fetch starts. `should_check` in `crates/web/src/updates.rs` is the rule, and its native tests pin the interval, the online gate, the switch, and a clock that moved backwards. A failed attempt counts as the day's attempt, so a flaky server is not asked on every focus.
- **On by default, opt-out.** Settings shows the switch **Check for updates automatically**, on unless the reader turns it off. The choice is stored in the IndexedDB `meta` store (`auto_update_check`); an absent value means on. The switch is the only way to stop the check, and the manual **Check now** always remains.
- **State survives a reload and works offline.** The check stores `last_checked`, the newest manifest's `content_version`, and its `total_bytes` in the `meta` store (`update_check`). After a reload or offline, the header pip and the Settings row come from that record. The manifest itself is kept only for the session: the installer panel mounts only when this session has it, because the panel fetches a manifest on mount and that would bypass the switch and the daily budget. Without it, Settings names the version and size it stored and offers **Check now**.
- **Small and silent.** The check is a background request (a few tens of KB) that no screen waits for. A failure changes nothing on screen unless the reader pressed **Check now**; the automatic path ignores it.
- **The header names what is waiting.** The update action appears for an app update or a known newer library, and opens Settings → Updates. Accessible names: "App update ready. Open Settings", "Library update available. Open Settings", and "Updates available. Open Settings" when both wait. The one-action rule still gives Update priority over Install.
- **Updates come first while something waits.** Settings renders the Updates section at the top when an app update is ready or a library update is known, and in its usual place otherwise. It shows the app row (with **Reload**) and the library row (version and size, the note "Your stars and book choices are kept.", and the installer panel) separately, then **Check now** and "Last checked: …".
