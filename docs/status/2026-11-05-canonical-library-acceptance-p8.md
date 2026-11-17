---
title: "Canonical library acceptance (P8)"
date: 2026-11-05
kind: record
---

# Canonical library acceptance (P8), 2026-11-05

At the time of P8, `cargo xtask build-site --canonical` packaged `generated/canonical/`
into `dist/` through the same Trunk build, and the default still packaged PSRD-Data
from `generated/content/`. The default now packages the canonical library from
`generated/content/`, and the PSRD build is `--psrd` into `generated/psrd/` (top
section). The Playwright suite reads its expectations from the compiled corpus
selected by `CAST_IRON_CONTENT` (`generated/content` by default). Both libraries were
checked against the same full suite, one `dist/` at a time. The command block below
records the P8 commands; the paths moved with the default switch.

| Project | Canonical | PSRD |
|---|---:|---:|
| chromium | 54/54 | 54/54 |
| webkit | 54/54 | 54/54 |
| mobile-chromium | 2/2 | 2/2 |
| Total | 110/110 | 110/110 |

Commands, run from the repository root:

```sh
cargo xtask content --canonical
cargo xtask build-site --canonical --public-url /cast-iron/
CAST_IRON_CONTENT=generated/canonical npm --prefix tests/e2e test -- \
  --project=chromium --project=webkit --project=mobile-chromium

cargo xtask content
cargo xtask build-site --public-url /cast-iron/
npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium
```

The canonical library in these runs is content `vd7596846128e349b`: 29,515 entries,
206,972,160 bytes, 65,725,435 bytes of search index. The PSRD build is
`vdb43853600b8ed8b`: 9,968 entries and 62,123,015 bytes. Two consecutive
`cargo xtask content --canonical` runs produced the same tree hash
5ac59bee1dcec9f72596ba1233446d952dbff8f73b2ae1be73ee3bb26ab830cb. `cargo xtask check` passes.

Measurements on that build, from `tests/e2e/measure-library.mjs` with a fresh profile and
the distribution server on localhost:

| Measure | Chromium | WebKit |
|---|---:|---:|
| Install, Download click to reader (s) | 1.3 | 2.3 |
| Storage usage / quota | 209.1 MB / 10.9 GB | 208.9 MB / 20.6 GB |
| First entry render (ms) | 32 | 36 |
| Search engine p50 / p95 (ms) | 53.5 / 200.0 | 52.0 / 211.0 |
| Search input to results p50 / p95 (ms) | 178 / 324 | 174 / 342 |
| JS heap after search | 5 MB | not available |
| Console errors | 0 | 0 |

The storage figures are `navigator.storage.estimate().usage`: the library plus the app
shell, about 1.0–1.9 % of the reported quota. No quota error occurred. The heap figure is
the Chromium JS heap only; WebAssembly linear memory is a separate allocation, so it is a
floor rather than the whole process. WebKit exposes no equivalent counter.

Acceptance surfaced two canonical-only gaps and two app defects:

- Search facets were empty in the canonical index, so the school, level, CR, and similar
  filters never appeared. `pipeline::build_canonical` now derives them from `TypedFields`
  and the typed archetype edge. Race groups remain absent because the pin stores them only
  in upstream HTML.
- Prestige-class requirements are prose in the canonical records, not typed fields, so the
  prestige pages have no named requirement links there. The requirement-link acceptance
  uses a feat, whose canonical records do carry typed prerequisite edges. Prestige
  category and publication navigation are still asserted.
- Search blocked for about 48 seconds before showing results offline. Every one of the 397
  publication chips rebuilt the whole discovery set for its own active-book total. A memo
  now computes all source totals in one pass per selection revision, and the chip counts
  read the search's `FilterCounts`; the first offline search now reaches results in about
  one second.
- Publication titles longer than a phone screen widened `.entry-list` rows and the page
  itself. `.row-source` and the row link now shrink and wrap, so `#/browse/kind/spell` has
  no horizontal page scroll at 390 px. Light and dark 390 px screenshots of that screen
  show the wrapped row inside the viewport.
- About 35 canonical spells carry upstream reference markup in their `school` attribute,
  such as `‹spelldef/abjuration› &#91&see text&#93&`. Their stat rows and the new school
  facet show that markup. Rendering attribute references is converter work outside this
  acceptance; it is recorded here rather than hidden, and the entries stay readable and
  searchable.

The sixteen PSRD books carry their curated chapters in this build; every other publication
is page-ordered. The canonical library also publishes both `core-rulebook` and a duplicate
`prpg-core-rulebook` book. Tests derive the book tree from `books.json` and attribute an
entry to its first placement, which is what the reader does. Folding the duplicate
publication remains later work.

Two harness flakes on a library this size were fixed while making the suite pass: WebKit's
header-button tab walk uses Option-Tab as the other keyboard tests do, and the mobile
install-button size check rounds device-pixel layout values before comparing with 44 px.

This is local verification. Nothing here is deployed by that build, and PSRD was still
the production default at that time.
