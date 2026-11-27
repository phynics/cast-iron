---
title: "Release `36cacaa` (corpus v0.1.65, schema 8)"
date: 2026-11-27
kind: release
---

# Release `36cacaa` (corpus v0.1.65, schema 8), 2026-11-27

`release` was promoted from `feb35542147da2fe3b65785663b8a70fff4ff3c8` to `36cacaacd32fcb478745316baf003000df9953df`
with `scripts/promote-release.py 36cacaacd32fcb478745316baf003000df9953df --push`, a fast-forward push, at the owner's
request ("ship schema 8"). `main` and `release` are now the same commit. **This is the first release with schema 8**:
the library is shipped and stored as gzip files ([decision](../decisions/2026-11-27-precompressed-library.md)).

The range is 24 merged pull requests, #736 to #765, in 98 files (8,624 insertions, 755 deletions). List them with
`git log --oneline --first-parent feb35542147da2fe3b65785663b8a70fff4ff3c8..36cacaacd32fcb478745316baf003000df9953df`.
The main changes are the precompressed library (#748, #751, #753, #761), the resume of paused library updates (#750),
the corpus pins v0.1.60 to v0.1.65 (#738, #741, #752, #759, #765), and the upgrade check's documentation (#760).

Step 7 of the upgrade check, as [releasing.md](../releasing.md) writes it, needs an iPhone simulator. This host has
none (the simulator needs macOS with Xcode), so the owner chose to measure in the T3 integrated browser on the live
site instead. That is a desktop Chromium, not a phone (see Live).

| Field | Value |
|---|---|
| Release commit | `36cacaacd32fcb478745316baf003000df9953df` |
| Previous release | `feb35542147da2fe3b65785663b8a70fff4ff3c8`, content `vaaeb1e4c7340254b` (schema 7, corpus v0.1.59) ([entry](2026-11-26-release-feb3554-corpus-v0-1-59-schema-7.md)) |
| Content | `v6c76ae276d557997`, schema 8, index format 2, 31,766 entries, 445 publications |
| Library size | 44,541,066 bytes stored in 275 files, from the live manifest (42.5 MB as the app shows it, which is MiB). The previous release was 215,286,180 bytes in 276 files |
| Shell | `eaa227c6`, the shell a reload applied on the live site (the previous release's shell was `a5d3b8e5`) |
| Corpus pin | `phynics/cast-iron-data` tag `v0.1.65`, commit `1d78219f05a18259c44aa8c417f1ce99cc425aee` |

**Local.** The schema 7 to 8 upgrade check of [releasing.md](../releasing.md) ran on a Linux host, with Playwright and
persistent browser profiles, in Chromium and WebKit. The previous release `feb35542` was served on one origin, then the
candidate `36cacaacd`. Steps 1 to 6 passed in both browsers.

1. *Schema 7 install.* The install had stars, active books, a chapter address and Continue reading. It passed.
2. *Explicit check.* The explicit update check refused schema 8 with the reload message. The daily check fetched only
   `content/manifest.json` and showed no message (see [#766](https://github.com/phynics/cast-iron/issues/766)). No `.gz`
   file was requested. It passed.
3. *Shell update.* The shell went from `750298bf547b950f` to `e62267d272b3b327`. The schema 7 library stayed readable,
   with search (50 hits), stars and an offline reload. It passed.
4. *Interrupted download.* `entries/item-011.json.gz` stalled at 274 of 275 files. On reopen the old library was still
   active, and Settings read "Update paused · 274 of 275 files". Resume fetched the one missing file, with
   `resumed_files` = 274. It passed. In WebKit the paused row showed "Check now" for about 2 s before "Update paused".
5. *Activation.* After activation the library was schema 8 at 44,541,066 bytes, and the old cache was gone. The storage
   estimate dropped by 169,283,671 B in Chromium and 173,819,907 B in WebKit, measured after a reopen. Verify printed
   "All 275 files match their checksums (42.5 MB)". The offline reload worked. It passed.
6. *Corrupt file.* With `creature-044.json.gz` corrupt in a fresh profile, the install showed "checksum mismatch" after
   3 attempts. Nothing was activated, and Retry was shown. It passed.

Also seen: WebKit once logged a CSP `style-src` refusal of a stylesheet on the new shell ([#767](https://github.com/phynics/cast-iron/issues/767)).

**CI.** Full CI run [46046218947](https://github.com/phynics/cast-iron/actions/runs/46046218947) on `main` at
`36cacaacd` concluded `success`, with every job passed.

**Deployment.** Pages run [46056179370](https://github.com/phynics/cast-iron/actions/runs/46056179370) for
`headBranch: release` at `36cacaacd` concluded `success`: the `build` and `deploy` jobs both succeeded. Pages
`build_type` is `workflow`. The `github-pages` environment allows only the `release` branch.

**Live.** On `https://phynics.github.io/cast-iron/`, 2026-11-27, after the deployment.

- *Step 8, headers and hashes.* `content/v6c76ae276d557997/search.bin.gz` returned HTTP 200 with
  `Content-Type: application/gzip`, no `Content-Encoding`, and `Content-Length: 20178944`, equal to the `bytes` of the
  manifest row. Its SHA-256 `07f427788d93da7d004f5bdf82dc228e82cb2d9012b6ba26ecb71e5b9cbb453a` equals the row's
  `sha256`. The same holds with `Accept-Encoding: gzip, deflate, br`. All 275 rows of the live manifest are gzip rows.
- *Step 7, in the T3 integrated browser.* This browser is desktop Chromium (Chrome/157, Linux), **not a phone**. It
  was a fresh profile on the live site. Diagnostics: "search index: read 438 ms, decode 200 ms, 65.6 MB; ready at
  16.9 s; stored size 19.2 MB". The first full search painted after 153 ms. The first install hung at 231 of 275
  files: one request (`entries/rule-048.json.gz`) never completed in the browser, while `curl` fetched it in 0.35 s
  ([#768](https://github.com/phynics/cast-iron/issues/768)). After a reload the install resumed with 232 files kept,
  and completed.
- *Existing profile, the owner's default browser profile.* Before: shell `a5d3b8e5` and library v0.1.40
  (`v3f3d0b708758820d`, schema 6, 207 MB, installed 2026-11-24), 441 of 443 books active. A reload applied shell
  `eaa227c6`, and the schema 6 library stayed readable. "Check now" offered v0.1.65 at 42.5 MB. "Download update"
  activated it: 42.5 MB on the device, only the new shell and content caches left, 443 of 445 books active (the same
  two off as before). Verify library printed "All 275 files match their checksums (42.5 MB)".

**Not verified.**

- A physical iPhone or Android device, and the iOS Simulator (no macOS host). Step 7 as releasing.md writes it is not
  done.
- The daily update check's reload message ([#766](https://github.com/phynics/cast-iron/issues/766)).
- Anything beyond the pages named above, at phone width, in dark mode, or with large text.

**Follow-ups.**

- [#766](https://github.com/phynics/cast-iron/issues/766): the upgrade-check text of the daily check.
- [#767](https://github.com/phynics/cast-iron/issues/767): a WebKit CSP `style-src` refusal of a stylesheet on the new shell.
- [#768](https://github.com/phynics/cast-iron/issues/768): the installer has no timeout for a request that hangs.
