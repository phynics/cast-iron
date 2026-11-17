# Status

Updated 2026-11-18. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

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

- **`release` is at `1324e1d`**, published 2026-11-17: corpus `v0.1.26`, schema 6, index format 2 ([release entry](status/2026-11-17-release-1324e1d-corpus-v0-1-26-schema-6.md)). That is what Pages serves.
- **`main`** equals `release` at the time of promotion. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.

## Latest runs

- **Full CI on `main`:** run [44167404059](https://github.com/phynics/cast-iron/actions/runs/44167404059) on `1324e1d`, 2026-11-17, succeeded with every job green. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [44180026338](https://github.com/phynics/cast-iron/actions/runs/44180026338) for `release` at `1324e1d`, 2026-11-17, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** the live manifest and shell stamps match the release (see the release entry). The owner waived the live browser checks for this release. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
