# Status

Updated 2026-11-17. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.26`, commit `a4b3ef61952098358e62274be1d0e17da99ea2c8` (`sources/corpus.toml`). Pin entry: [2026-11-17](status/2026-11-17-corpus-pinned-to-v0-1-26.md) |
| Content version | `vb4ec72f397a03202` (`cargo xtask content` on `main` at `2f1d4ca` plus the v0.1.26 pin, 2026-11-17) |
| Library | 31,800 entries, 439 publications, 439 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `6365dba`**, published 2026-11-09: corpus `v0.1.17`, schema 4, index format 2 ([release entry](status/2026-11-09-release-6365dba-corpus-v0-1-17-schema-4-index-format-2.md)). That is what Pages serves.
- **`main` is 219 commits ahead** (2026-11-17). It carries corpus `v0.1.26` and schema 6 (book membership in the catalog stub, [#394](https://github.com/phynics/cast-iron/issues/394); typed trades in `trades.json`, [#450](https://github.com/phynics/cast-iron/issues/450)). None of it is published.
- **Promotion is on hold.** Breaking format changes ship together as "format 5" (owner decision 2026-11-16, [decision](decisions/2026-11-16-owner-decisions-from-the-architecture-review.md)). Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.

## Open release gates

[#441](https://github.com/phynics/cast-iron/issues/441) is the checklist; copy of 2026-11-17, the issue is the record:

- [ ] Real schema-4 to 5 upgrade check in a browser, on Chromium and WebKit: the old library keeps working until the new one is verified, stars, recents and active books survive, and membership visibility works after the update.
- [ ] Fresh-install check on the promoted build.
- [ ] Full CI green on the exact `main` SHA that is promoted.
- [ ] iPhone measurement results ([#423](https://github.com/phynics/cast-iron/issues/423)) reviewed, if the session happened.
- [ ] Release notes list the user-visible changes since the last promotion.

## Latest runs

- **Full CI on `main`:** the dispatched run [44096836868](https://github.com/phynics/cast-iron/actions/runs/44096836868) on `c14d7f0`, 2026-11-17 13:50 UTC, succeeded. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [42683286107](https://github.com/phynics/cast-iron/actions/runs/42683286107) for `release` at `6365dba`, 2026-11-09, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** recorded with the release entry above. Nothing newer exists.
