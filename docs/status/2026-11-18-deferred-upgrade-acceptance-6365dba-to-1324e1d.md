---
title: "Deferred upgrade acceptance: 6365dba to 1324e1d"
date: 2026-11-18
kind: record
---

# Deferred upgrade acceptance: `6365dba` to `1324e1d`, 2026-11-18

Refs [#441](https://github.com/phynics/cast-iron/issues/441). This is the browser
acceptance that the [release record](2026-11-17-release-1324e1d-corpus-v0-1-26-schema-6.md)
listed as not run: an installed profile on the published schema 4 release
(`6365dba`, corpus v0.1.17) upgrading to the published schema 6 release
(`1324e1d`, corpus v0.1.26). The run was 2026-11-17. Chromium and WebKit each ran
the sequence in one fresh persistent profile against one origin,
`http://127.0.0.1:4318/cast-iron/`, with the two artifacts served in turn from
that path.

This is local automation evidence. It is not a device result, not a live-site
check, and not a CI run. The physical iPhone checks stay with
[#423](https://github.com/phynics/cast-iron/issues/423).

## Artifacts under test

| | Old (`6365dba`) | New (`1324e1d`) |
|---|---|---|
| Source | rebuilt at the release commit with corpus `v0.1.17`; the published artifact is gone | `github-pages` artifact of Pages run [44180026338](https://github.com/phynics/cast-iron/actions/runs/44180026338) |
| Content | `v17a954e5c0f1d11c`, schema 4, 31,807 entries, 225,487,861 bytes | `vb4ec72f397a03202`, schema 6, 31,800 entries, 223,130,012 bytes |
| Content files verified against the manifest | 293 of 293 | 292 of 292 |
| Shell | rebuilt `831c17bb103a2600`, date 2026-11-09 | published `cf5cf4d6590bd6a9`, date 2026-11-17 |

The new artifact is the published one: its `content/manifest.json` and `sw.js`
are byte-identical to the files served by https://phynics.github.io/cast-iron/
on 2026-11-17 (`sw.js` SHA-256 `290edc0d700ca86c920c001a55c40a0b223df0779ca2d81353a6bff5bbf6893c`).

**The old artifact could not be used.** Run `42683286107` lists no artifacts, and
artifact `13703523583` from its build job returns HTTP 404, so the published
`github-pages` zip has expired. The old side is a rebuild at the exact release
commit with the pinned corpus. Its library is the published library by identity:
`content_version` is a SHA-256 over every content file's path and hash, and it
equals the published `v17a954e5c0f1d11c`; all 293 files match their manifest. Its
shell is not the published shell: the rebuild stamps `831c17bb103a2600` where the
release published `5464362442e5568f`. The shell hash covers the Rust WASM binary,
and the local dev shell compiles with rustc 1.95.0 while CI used 1.99.1 from
`rust-toolchain.toml`. The CI archive listing and the rebuild share every file
name except that binary (`cast-iron-web-3aea756b22873ed3_bg.wasm` in CI,
`cast-iron-web-c2025da3ad68e7c8_bg.wasm` locally). Trunk names the JavaScript,
the stylesheet and the bridge snippet after their contents, and all three names
match the CI archive, which points to identical content for those three;
`index.html`, the icons
and the unstamped `sw.js` carry no content hash in their names, and the WASM
differs. So the library side of this check is exact; the shell side is the same
source built by a different compiler, not the bytes users downloaded in
November.

## Seeded state

The fresh profile installed the old library, failed one download on purpose, and
completed it on Retry. It then starred Power Attack, Magic Missile and Fireball;
visited six entries (the three stars plus Acrobatic, Weapon Finesse and Summon
Monster I); and in Manage books turned **Ultimate Magic** and **Mythic
Adventures** off from the 28 active rulebooks, leaving **Core Rulebook** and
**Bestiary** on. That is 3 stars, 6 recents and 413 disabled book ids.

## What happened, in order

1. The new artifact was served at the same origin and path. The installed app
   kept the old shell and the old library; Home and Fireball opened.
2. Settings → **Check now** found the shell update and the newer library at the
   same time. The old shell refused the library with *"This library uses content
   format 6, but this version of the app reads format 4. Reload the app to get
   the latest version, then try again."* It offered no download, which is the
   intended order: the app update comes first.
3. **Reload** on the app update banner installed shell `cf5cf4d6590bd6a9`.
   Settings then showed the old library (`v0.1.17`, content format 4, 31,807
   entries) and Fireball opened under the new shell.
4. **Download update** ran until the server dropped every request. Chromium had
   written 169 of 292 files (90.4 MB of 213 MB); WebKit 189 of 292 (100 MB).
   Both showed the failure (`vb4ec72f397a03202/entries/….json: network error …`)
   with **Retry**. The active library stayed `v17a954e5c0f1d11c` (schema 4), a
   second tab opened Magic Missile from it, and Settings still reported format 4.
5. **Retry** resumed from the verified files and activated
   `vb4ec72f397a03202`: schema 6, 31,800 entries, 223,130,012 bytes, format 6 in
   Settings. Only `cast-iron-content-vb4ec72f397a03202` remained in the content
   caches, the old one was deleted, and no pending-install marker was left.

## Results

| Check | Chromium | WebKit |
|---|---|---|
| Fresh install of the old library, one interrupted download, retry completes | PASS | PASS |
| Old identity: `v0.1.17`, format 4, 31,807 entries, 215 MB | PASS | PASS |
| Stars, recents and book choices seeded and recorded | PASS | PASS |
| App update offered; old shell refuses format 6 and keeps the old library | PASS | PASS |
| Shell update applied; old library stays active, Fireball opens | PASS | PASS |
| Library download interrupted; old library stays active and readable in a second tab | PASS | PASS |
| Retry activates the new library; format 6, 31,800 entries, 213 MB | PASS | PASS |
| 3 stars and 6 recents unchanged; Ultimate Magic and Mythic Adventures still off; Core Rulebook and Bestiary still on | PASS | PASS |
| One content cache, no pending install, old cache removed | PASS | PASS |
| Fireball: `Sources: Core Rulebook p. 283`; Summon Monster I: `Expanded in: Monster Summoner's Handbook p. 18 · Inactive`; Acrobatic opens with Mythic Adventures off | PASS | PASS |
| Offline reload: Home keeps the star, Fireball opens, search finds a spell | PASS | PASS |
| Page errors | 0 | 0 |
| Third-party requests | 0 | 0 |

Console messages: Chromium logged 18, all resources the interruption step cut on
purpose. WebKit logged those plus one `style-src-elem` CSP entry per
`page.screenshot()` (10 in this run), which is the stylesheet Playwright injects
to hide the caret; a load of the same build and profile without a screenshot
logs none. Neither engine logged an unexpected application error.

**Fresh install of the new artifact, separate fresh profiles.** Both engines
downloaded `vb4ec72f397a03202` from the published artifact, showed format 6 and
31,800 entries, starred Power Attack, reloaded with the network off, opened
Fireball and found Magic Missile by search. 0 page errors and 0 third-party
requests. Chromium 0 console messages; WebKit 1, the screenshot stylesheet
again.

## Limits

- The old shell is a rebuild, as described above. The shell update path is real
  (the service worker bytes differ and the update waits for Reload), but it ran
  from a locally compiled old shell.
- Only the default active set was exercised. Share codes, a book removed
  between the releases, and the membership matrix are covered by the CI browser
  suite, not by this run.
- No physical device, no iOS Safari, no Home Screen app, no Firefox.

## Where the evidence is

The run manifest is
[441-upgrade-acceptance.json](../research/capture-evidence/441-upgrade-acceptance.json):
per engine, every step and result, the seeded ids, the active library record, the
cache names, the interrupted progress and error text, and the console and
request counts. The 20 screenshots (10 per engine: old Home and Settings, the
format 6 refusal, the old library under the new shell, the interrupted update,
the new Settings, Home, a sources line, and the offline page) are local files and
are not committed; attaching them to [#441](https://github.com/phynics/cast-iron/issues/441)
is still pending.

## Reproduce

```sh
# The new artifact: the Pages deployment for the release.
gh run download 44180026338 --repo phynics/cast-iron -n github-pages -D new-dist
mkdir new && tar -xf new-dist/artifact.tar -C new
curl -fsS https://phynics.github.io/cast-iron/sw.js | sha256sum   # same bytes as new/sw.js

# The old artifact: rebuild at the release commit with the pinned corpus.
git worktree add --detach ../cast-iron-6365dba 6365dba
cd ../cast-iron-6365dba
cargo xtask fetch-sources && cargo xtask content
cargo xtask build-site --public-url /cast-iron/
```

Serve one directory at a time at `http://127.0.0.1:4318/cast-iron/` with
`node tests/e2e/server.mjs` after setting `E2E_DIST`, and run a real Chromium and
WebKit with a new persistent profile per engine. The hooks used are the ones the
server documents: `POST /__test/refuse {"on":true}` to drop every request (the
interrupted download; set it to `false` before Retry). The order of clicks is the
one under "What happened": install, interrupt, Retry, star, visit, Manage books,
swap the served directory, reload, Check now, Reload, Check now, Download
update, interrupt, Retry, then Settings, Manage books, entries and an offline
reload.
