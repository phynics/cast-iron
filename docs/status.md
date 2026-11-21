# Status

Updated 2026-11-21. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.36`, commit `8344c87b2093a6fcfe69e2fff4a4a923074d4deb` (`sources/corpus.toml`), corpus format 3. Pin entry: [2026-11-21](status/2026-11-21-corpus-pinned-to-v0-1-36.md) |
| Content version | `v8f589fddba1bb56a` (`cargo xtask content` on `main` at `27e3209f5` plus the v0.1.36 pin, 2026-11-21). The release still serves `v24010454fb1d951c` (corpus v0.1.34). |
| Library | 31,825 entries, 443 publications, 443 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `d200b4d`**, published 2026-11-21: corpus `v0.1.34`, schema 6, index format 2 ([release entry](status/2026-11-21-release-d200b4d-corpus-v0-1-34-schema-6.md)). That is what Pages serves.
- **`main`** equals `release` at the time of promotion. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [44894283691](https://github.com/phynics/cast-iron/actions/runs/44894283691) on `d200b4d`, 2026-11-21, succeeded; every job passed, including all four informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [44898321278](https://github.com/phynics/cast-iron/actions/runs/44898321278) for `release` at `d200b4d`, 2026-11-21, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** on 2026-11-21 the live manifest (`v24010454fb1d951c`, corpus `v0.1.34`) and shell stamps (`8009aeb778ed3216`) match the release. A Playwright run in Chromium and WebKit installed the library and checked the wordmark, old `?a=toc-…` links redirecting to Ranger and Rogue, `?c=` addresses, a table-cell footnote link, the Abjuration power text, the Goblin defense rows and a stale Continue Reading point, then reloaded and searched offline (see the release entry). One transient 503 for a library file in an exploratory run and one WebKit `Context is stopped` page error in one of two runs are recorded there. No real-device check, no existing installed profile and no upgrade from the previous release. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
