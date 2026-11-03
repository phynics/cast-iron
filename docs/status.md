# Status

Updated 2026-11-03, expansion branch `expand-library`. The repository is public. The 16-book expansion is locally verified and awaiting merge and deployment; the live site still has the original POC until that rollout.

## POC acceptance checklist (spec section 11)

"Local" means the Playwright suite in `tests/e2e` passed against a real `dist/` build served at `/cast-iron/` by `tests/e2e/server.mjs`. Last local run: 37 passed in 1.6 minutes (chromium 18, webkit 18, mobile-chromium 1), including offline class-category and expanded-book browsing. `cargo xtask check` also passed.

| Item | State | Evidence |
|---|---|---|
| Static GitHub Pages deployment works | Live pass | [Deployment run 41576342801](https://github.com/phynics/cast-iron/actions/runs/41576342801) succeeded for `a6636f3`. Chromium installed the library from [the public site](https://phynics.github.io/cast-iron/), verified the contact and OGL notice, reloaded offline, and found Fireball offline. No page errors or third-party requests occurred. |
| Application is installable where browser support allows | Local pass | `Application is installable where browser support allows` |
| Representative rules dataset installs completely | Local pass | `Representative rules dataset installs completely` |
| Download integrity is verified | Local pass | `Download integrity is verified` (corrupted shard rejected; retry fetches only that file) |
| Search operates entirely offline | Local pass | `Search operates entirely offline` (p95 < 100 ms asserted) |
| Category and sourcebook browsing works | Local pass | `Category and sourcebook browsing works` checks every publication; `Expanded books and separate class categories work offline` checks 16 books and the distinct categories. |
| Rule entries preserve complex formatting | Local pass | three `Rule entries preserve complex formatting: …` tests |
| Cross-reference navigation works | Local pass | `Cross-reference navigation works`, plus the deep-link test |
| Back/forward navigation works | Local pass | `Back/forward navigation works` |
| Bookmarks persist across restarts | Local pass | `Bookmarks persist across restarts` (persistent browser profile, relaunched offline) |
| Interrupted installation can recover safely | Local pass | `Interrupted installation can recover safely` |
| Offline launch works after a new browser session | Local pass | `Offline launch works after a new browser session` |
| No application feature requires a runtime server | Local pass | `No application feature requires a runtime server` (zero server requests after install, apart from the browser's own `sw.js` update check) |
| The compiler generates repeatable artifacts | Local pass | Two final full-corpus compiles were byte-identical. CI job `content` compiles twice and compares. |
| Licensing information is preserved and accessible | Local pass | `Licensing information is preserved and accessible` |

Phase 6 also requires that bookmarks survive a content update. `Content update keeps user records` covers this. A separate real-browser check installed the actual published schema-1 POC library in the new app, bookmarked Arcane Archer, reloaded offline, installed schema 2, and confirmed the bookmark and offline reading survived. `scripts/check-library-upgrade.py` also confirmed all 1,937 published IDs and titles are preserved.

## Expanded corpus, locally verified

Content version `v91f4bc8ffd606a17`, schema 2. Classes has 27 entries; Prestige classes has 19, including Technology Guide's Technomancer.

| Measure | Value |
|---|---|
| Entries | 9,112 across 16 publications |
| Download | 58,487,268 bytes raw |
| WASM | 1,212,369 bytes raw; 450,990 bytes gzip |
| Search index | 19,264,125 bytes raw |
| Search latency | p50 2.5 ms, p95 4.8 ms, max 5.4 ms over 20 queries in desktop Chromium |
| Index load | 24.2 ms from the Cache API |
| Links | 1,304 resolved, 128 out of selection, 38 broken upstream |
| Intentional omissions | 432 source occurrences, including duplicated story copies and partial-body omissions; no malformed-record rejections |

These are desktop measurements, not real-phone memory or speed results. Light and dark Prestige classes screens were inspected at 390 px and 1280 px. The original library stayed readable under the new app before the schema-2 update was installed.

| Publication | Entries |
|---|---:|
| Advanced Player's Guide | 768 |
| Advanced Race Guide | 516 |
| Bestiary | 545 |
| Bestiary 2 | 428 |
| Bestiary 3 | 360 |
| Bestiary 4 | 288 |
| Core Rulebook | 1,612 |
| Game Mastery Guide | 146 |
| Monster Codex | 395 |
| Mythic Adventures | 375 |
| NPC Codex | 342 |
| Technology Guide | 166 |
| Ultimate Campaign | 65 |
| Ultimate Combat | 618 |
| Ultimate Equipment | 2,066 |
| Ultimate Magic | 422 |

The selection has 64 typed directories and 101 complete rules trees. Unsupported standalone record families and duplicated section-file representations remain listed in coverage. Later books absent from the pinned snapshot are not included.

## Not verified yet

- GitHub CI and deployment for this expansion are pending. POC PR CI passed on GitHub in runs 41569305309 and 41569305238. The first Pages attempt failed before Pages was enabled. Deployment run 41576342801 then succeeded and the public site passed the live smoke check.
- Firefox: the tests exist, but Playwright's Firefox does not start on the development machine (macOS 27). CI runs Firefox as non-blocking.
- Real devices: Android Chromium, iOS Safari, installed-app (standalone) relaunch.
- Persistent storage: headless Chromium reports "not granted". On real browsers the result depends on browser heuristics.

## Known gaps

- Licensing review is scoped to the 16 selected books. The publication screen covers all 9,112 entries and found no issue, but does not classify every sentence or establish blanket rights to new sources. See `docs/research/licensing-audit.md`.
- No "Referenced by" list (needs a reverse link index). Only "References" exists.
- Facet chips show global counts, not counts that depend on the other filters.
- In spell and creature `spells` stat lines, upstream lists become lines with breaks, so list structure is lost.
- Heading ids are not prefixed, so a content anchor could collide with an app element id such as `main`.
- Firefox, and a single flaky run of `Back/forward navigation works` on Chromium, which could not be reproduced (see `docs/decisions.md`).

## Next (post-POC, spec section 12)

1. Keep the licensing review current when changing the source selection. The POC is public under the scoped OGL and Community Use review.
2. Add standalone support for record families currently represented only in rules trees or excluded, such as archetypes, traits, and animal companions. Newer books require a new source pin or source dataset and licensing review.
3. Run the suite on real devices and record the results here.
