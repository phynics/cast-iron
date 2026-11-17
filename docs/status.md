# Status

Updated 2026-11-17. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`, then `cargo xtask status-index`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.24`, commit `fdb3fc2f24cf06a0c38aedfd92e22a65b78788aa` (`sources/corpus.toml`). Pin entry: [2026-11-17](status/2026-11-17-corpus-pinned-to-v0-1-24-with-the-placement-rules.md) |
| Content version | `CONTENT_VERSION` (`cargo xtask content` on `main` at `f59c34c`, 2026-11-17) |
| Library | 31,800 entries, 439 publications, 439 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 5 (`crates/schema/src/lib.rs`; the client reads 1 to 5) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `6365dba`**, published 2026-11-09: corpus `v0.1.17`, schema 4, index format 2 ([release entry](status/2026-11-09-release-6365dba-corpus-v0-1-17-schema-4-index-format-2.md)). That is what Pages serves.
- **`main` is 211 commits ahead** (2026-11-17). It carries corpus `v0.1.24` and schema 5 (book membership in the catalog stub, [#394](https://github.com/phynics/cast-iron/issues/394)). None of it is published.
- **Promotion is on hold.** Breaking format changes ship together as "format 5" (owner decision 2026-11-16, [decision](decisions/2026-11-16-owner-decisions-from-the-architecture-review.md)). Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.

## Open release gates

[#441](https://github.com/phynics/cast-iron/issues/441) is the checklist; copy of 2026-11-17, the issue is the record:

- [ ] Real schema-4 to 5 upgrade check in a browser, on Chromium and WebKit: the old library keeps working until the new one is verified, stars, recents and active books survive, and membership visibility works after the update.
- [ ] Fresh-install check on the promoted build.
- [ ] Full CI green on the exact `main` SHA that is promoted.
- [ ] iPhone measurement results ([#423](https://github.com/phynics/cast-iron/issues/423)) reviewed, if the session happened.
- [ ] Release notes list the user-visible changes since the last promotion.

## Latest runs

- **Full CI on `main`:** the scheduled run [44077764758](https://github.com/phynics/cast-iron/actions/runs/44077764758) on `c13d001`, 2026-11-17 11:51 UTC, succeeded. `main` has moved 18 commits since, so there is no full run on its head yet. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [42683286107](https://github.com/phynics/cast-iron/actions/runs/42683286107) for `release` at `6365dba`, 2026-11-09, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** recorded with the release entry above. Nothing newer exists.
