# Status

Updated 2026-11-21. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.37`, commit `76517ea3cda1169892b8fd32eb297c091d984b9b` (`sources/corpus.toml`), corpus format 3. Pin entry: [2026-11-21](status/2026-11-21-corpus-pinned-to-v0-1-37.md) |
| Content version | `v28481793019cf75c` (`cargo xtask content` on `main` at `80d0cf0c9` plus the v0.1.37 pin, 2026-11-21). The release serves the same version. |
| Library | 31,825 entries, 443 publications, 443 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `9a6fb06`**, published 2026-11-21: corpus `v0.1.37`, schema 6, index format 2 ([release entry](status/2026-11-21-release-9a6fb06-corpus-v0-1-37-schema-6.md)). That is what Pages serves. The previous release was `d200b4d` ([entry](status/2026-11-21-release-d200b4d-corpus-v0-1-34-schema-6.md)).
- **`main`** equals `release` (`9a6fb06`) until the next merge. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [44955036533](https://github.com/phynics/cast-iron/actions/runs/44955036533) on `9a6fb06`, 2026-11-21, succeeded; every job passed, including the informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [44957082851](https://github.com/phynics/cast-iron/actions/runs/44957082851) for `release` at `9a6fb06`, 2026-11-21, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** on 2026-11-21 the live manifest (`v28481793019cf75c`, corpus `v0.1.37`, 224,578,783 bytes in 294 files) and shell stamps (`738b7855d23307b9`) match the release; the shipped WASM is 2,783,498 bytes. A Playwright run in Chromium and WebKit installed the library and checked the class-option power layout (Abjuration, Evil Eye), the Wizard's Also available links, the prestige class Requirements stop, Copy link, Game Mastery Guide's place in the books, the Elephant in the Room `?a=toc-1-1` address and the Transmutation Anointings, then reloaded and searched offline; all passed with no console error, page error or third-party request (see the release entry; three early failures were mistakes in the throwaway script). No real-device check, no existing installed profile and no upgrade from the previous release. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
