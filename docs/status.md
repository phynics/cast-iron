# Status

Updated 2026-11-20. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.32`, commit `c265325ece61ef4fde220631a6865beef629472b` (`sources/corpus.toml`). Pin entry: [2026-11-20](status/2026-11-20-corpus-pinned-to-v0-1-32.md) |
| Content version | `vac36ce2cc1ac05cb` (`cargo xtask content` on `main` at `622eaa6e8` plus the v0.1.32 pin, 2026-11-20). The release serves `v4192151b29702eab` (corpus v0.1.31); v0.1.32 is on `main` only. |
| Library | 31,825 entries, 443 publications, 443 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `622eaa6`**, published 2026-11-20: corpus `v0.1.31`, schema 6, index format 2 ([release entry](status/2026-11-20-release-622eaa6-corpus-v0-1-31-schema-6.md)). That is what Pages serves.
- **`main`** equals `release` at the time of promotion. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.

## Latest runs

- **Full CI on `main`:** run [44739978417](https://github.com/phynics/cast-iron/actions/runs/44739978417) on `622eaa6`, 2026-11-20, succeeded; every job passed, including all four informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [44759576175](https://github.com/phynics/cast-iron/actions/runs/44759576175) for `release` at `622eaa6`, 2026-11-20, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** on 2026-11-20 the live manifest (`v4192151b29702eab`, corpus `v0.1.31`) and shell stamps match the release. A Playwright run in Chromium and WebKit installed the library, rendered Summon Monster II without notes, Detect Law with one note, Detect Relations with `Range: N/A` and Torrent of Elemental Rage without a Range line, then reloaded and searched offline (see the release entry). No real-device check, no existing installed profile and no upgrade from the previous release. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
