# Status

Updated 2026-11-19. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.29`, commit `d9bff95d097b0bef28df9cd7fd7071d1a816d964` (`sources/corpus.toml`). Pin entry: [2026-11-19](status/2026-11-19-corpus-pinned-to-v0-1-29.md) |
| Content version | `vb23c2dd0a96f4c21` (`cargo xtask content` on `main` at `0852e9c` plus the v0.1.29 pin, 2026-11-19) |
| Library | 31,825 entries, 443 publications, 443 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `58ca785`**, published 2026-11-19: corpus `v0.1.29`, schema 6, index format 2 ([release entry](status/2026-11-19-release-58ca785-corpus-v0-1-29-schema-6.md)). That is what Pages serves.
- **`main`** equals `release` at the time of promotion. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.

## Latest runs

- **Full CI on `main`:** run [44606423367](https://github.com/phynics/cast-iron/actions/runs/44606423367) on `58ca785`, 2026-11-19, succeeded with every job green. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [44610016618](https://github.com/phynics/cast-iron/actions/runs/44610016618) for `release` at `58ca785`, 2026-11-19, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** on 2026-11-19 the live manifest (`vb23c2dd0a96f4c21`) and shell stamps match the release. A Playwright run in Chromium and WebKit installed the library and rendered a feat, a domain, Manage books with the share section, and an entry's Notes group (see the release entry). No real-device check and no upgrade from the previous release. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
