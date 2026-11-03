# Status

Updated 2026-11-03, branch `poc`.

## POC acceptance checklist (spec section 11)

"Local" means the Playwright suite in `tests/e2e` passed against a real `dist/` build served at `/cast-iron/` by `tests/e2e/server.mjs`. Last local run: 35 passed (chromium 17, webkit 17, mobile-chromium 1).

| Item | State | Evidence |
|---|---|---|
| Static GitHub Pages deployment works | Not verified | `.github/workflows/pages.yml` exists and passes actionlint. The repository is private and Pages is not enabled. The project-path build (`/cast-iron/`) passes locally. |
| Application is installable where browser support allows | Local pass | `Application is installable where browser support allows` |
| Representative rules dataset installs completely | Local pass | `Representative rules dataset installs completely` |
| Download integrity is verified | Local pass | `Download integrity is verified` (corrupted shard rejected; retry fetches only that file) |
| Search operates entirely offline | Local pass | `Search operates entirely offline` (p95 < 100 ms asserted) |
| Category and sourcebook browsing works | Local pass | `Category and sourcebook browsing works` |
| Rule entries preserve complex formatting | Local pass | three `Rule entries preserve complex formatting: …` tests |
| Cross-reference navigation works | Local pass | `Cross-reference navigation works`, plus the deep-link test |
| Back/forward navigation works | Local pass | `Back/forward navigation works` |
| Bookmarks persist across restarts | Local pass | `Bookmarks persist across restarts` (persistent browser profile, relaunched offline) |
| Interrupted installation can recover safely | Local pass | `Interrupted installation can recover safely` |
| Offline launch works after a new browser session | Local pass | `Offline launch works after a new browser session` |
| No application feature requires a runtime server | Local pass | `No application feature requires a runtime server` (zero server requests after install, apart from the browser's own `sw.js` update check) |
| The compiler generates repeatable artifacts | Local pass | Three compiles gave the same tree hash. CI job `content` compiles twice and compares. |
| Licensing information is preserved and accessible | Local pass | `Licensing information is preserved and accessible` |

Phase 6 also requires that bookmarks survive a content update. `Content update keeps user records` covers this.

## Measurements (POC corpus, local)

| Measure | Value |
|---|---|
| Entries | 1,937 (spell 623, item 590, creature 357, feat 175, affliction 52, rule 48, trap 33, class 26, skill 26, race 7) from Core Rulebook and Bestiary |
| Content download | 11.7 MB raw. About 1.2 MB of that is `search.bin` gzipped, and about 1.2 MB is entries plus catalog gzipped |
| WASM | 1,211,742 bytes raw, 450,217 bytes gzip -9 |
| Search latency, browser main thread | p50 5.2 ms, p95 7.7 ms over 20 queries on desktop Chromium. Mobile viewport: p50 5.4 ms, p95 7.3 ms |
| Search index load | 8.2 ms for the 4.1 MB `search.bin`, from the Cache API |
| Longest entry render (`core-rulebook/classes/cleric`, 420 blocks) | 13 ms warm, 39 ms cold, 60–64 ms with 4x CPU throttling |
| Links | 882 resolved, 124 point outside the POC selection, 31 broken upstream |

The latency figures come from desktop Chromium, with a mobile viewport and CPU throttling. No real mid-range phone has been measured.

## Not verified yet

- GitHub Actions runs and the GitHub Pages deployment. The workflows have not run on GitHub yet.
- Firefox: the tests exist, but Playwright's Firefox does not start on the development machine (macOS 27). CI runs Firefox as non-blocking.
- Real devices: Android Chromium, iOS Safari, installed-app (standalone) relaunch.
- Persistent storage: headless Chromium reports "not granted". On real browsers the result depends on browser heuristics.

## Known gaps

- Licensing: PSRD-Data has no license of its own, and no per-record Open Game Content / Product Identity audit has been done. Resolve this before a public deployment (`docs/research/licensing-audit.md`).
- No "Referenced by" list (needs a reverse link index). Only "References" exists.
- Facet chips show global counts, not counts that depend on the other filters.
- In spell and creature `spells` stat lines, upstream lists become lines with breaks, so list structure is lost.
- Heading ids are not prefixed, so a content anchor could collide with an app element id such as `main`.
- Firefox, and a single flaky run of `Back/forward navigation works` on Chromium, which could not be reproduced (see `docs/decisions.md`).

## Next (post-POC, spec section 12)

1. Resolve the licensing questions, then decide on a public deployment.
2. Add the remaining PSRD-Data books. The compiler already reports them as excluded. Measure index size and memory on the full corpus (25k upstream files), and split `search.bin` if necessary.
3. Run the suite on real devices and record the results here.
