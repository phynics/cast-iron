---
title: "Settings opens with one Version & updates section, and a paused library update survives a reload"
date: 2026-11-27
status: current
superseded_by: []
issues: [747]
prs: []
area: ui
---

# Settings opens with one Version & updates section, and a paused library update survives a reload, 2026-11-27

Refs [#747](https://github.com/phynics/cast-iron/issues/747). Owner decisions of 2026-11-27. This replaces the section list of [Settings has seven sections](2026-11-23-settings-has-seven-sections-and-one-update-row.md); its other rules (no moving section, the single update row, no maintainer words, every test id) stand. The page is described in `DESIGN.md` under Settings.

- **Seven sections, new first one.** The order is Version & updates, Appearance, Books, Library, Install, Storage, Help. Version & updates is first, never moves, and is still opened and focused by the header action through `?updates` (`#set-updates`).
- **What it holds.** The app version and its date, the rules library (tag, size on this device, book count), what waits to update with its one action, the daily-check switch, and `Check now`. The version rows leave Library and Help. Library keeps `Entries`, `Installed` and `Verify library`; Help keeps only links.
- **Test ids keep their meaning.** `library-version`, `shell-version`, `app-install`, `verify-status`, `update-status` and `download-size` are unchanged; `library-size` and `library-facts` moved into the new section, and `library-books` is new. The update row's term is `Update available`, so it no longer repeats `Rules library`.
- **A paused library update survives a reload.** The installer already keeps verified files in the version's cache and the `pending_install` marker until activation. On the next open the update shows as paused: `Update paused · 98 of 277 files` and a `Resume` button, in Settings and in the header action (`Update paused`). The marker now also stores the file count (`files_total`; an older marker shows `Update paused` without a count). The count lists the cache's keys and reads no file; the installer re-hashes every file when it resumes, so the count is progress, not proof.
- **Nothing downloads until Resume (rule 1).** Opening the app only reads the marker and the key list. `Resume` reads the app's own manifest if the session has none (the reader's tap is the request), then starts the installer, which scans the cache and fetches the rest. Offline, `Resume` is disabled with `Connect to resume.`
- **Stale or missing.** When the last check, or the manifest read by `Resume`, names another version as the newest, the paused version's cache and marker are deleted and the newest is offered with `Download update`. A paused update that this session did not start is the only kind that is dropped this way; an install running in this session is never touched. A missing or partly deleted cache counts what is there (zero if none), and `Resume` downloads the rest.
- **A second tab is not guarded.** The app has no lock between tabs, so two tabs that both tap `Resume` or `Download update` run two downloads of the same files. Each file is verified before it is cached, so the result is correct, only wasteful. A guard is out of scope here.
