# Status

Updated 2026-11-27. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.66`, commit `4d635089edcd34ad0c6d5a1e30b61d1e5fe728f1` (`sources/corpus.toml`), corpus format 4. Pin entry: [2026-11-27](status/2026-11-27-pin-corpus-v0-1-66.md) |
| Content version | `vdfca03aaa4a17a38` (`cargo xtask content` on `main` at `5d4bf3a27` plus the v0.1.66 pin, 2026-11-27). `release` serves it too ([entry](status/2026-11-27-release-33af444-corpus-v0-1-66.md)). |
| Library | 31,766 entries, 445 publications, 445 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 8 on `main` (`crates/schema/src/lib.rs`): the library is shipped and stored as gzip files ([decision](decisions/2026-11-27-precompressed-library.md)); `release` is schema 8 since 2026-11-27 (the haunt kind of schema 7 is in it too, [decision](decisions/2026-11-23-haunts-are-a-kind-schema-7.md)) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `33af444`**, published 2026-11-27: corpus `v0.1.66`, schema 8, index format 2, and the installer stall timeout of [#768](https://github.com/phynics/cast-iron/issues/768) ([release entry](status/2026-11-27-release-33af444-corpus-v0-1-66.md)). That is what Pages serves. The previous release was `36cacaa`, the first schema 8 release ([entry](status/2026-11-27-release-36cacaa-corpus-v0-1-65-schema-8.md)).
- **`main` is ahead of `release` only by docs**: `main` was `33af444` when it was promoted, and later commits on `main` are status records. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **The haunt window and the promotion gate are done.** The haunt kind (schema 7) is published, and the schema upgrade check of [releasing.md](releasing.md) (old library to new shell to new library, an interrupted download, stars including a haunt star made under `rule`) passed before the promotion in Chromium and WebKit.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). [#12](https://github.com/phynics/cast-iron/issues/12) (physical-device and accessibility acceptance) was closed by the owner on 2026-11-25 without a real-device check. The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [46065669260](https://github.com/phynics/cast-iron/actions/runs/46065669260) on `33af444`, 2026-11-27, succeeded; all jobs passed. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [46067531840](https://github.com/phynics/cast-iron/actions/runs/46067531840) for `release` at `33af444`, 2026-11-27, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Measurements:** the integrated-browser run of 2026-11-24 on `2b56cc1` (install, cold start, search, Wizard page) is recorded in [its entry](status/2026-11-24-integrated-browser-measurements.md). These are not iPhone numbers.
- **Live verification:** on 2026-11-27 the live manifest reads `vdfca03aaa4a17a38` (corpus `v0.1.66`), schema 8, and the owner's default browser profile updated from v0.1.65 to v0.1.66 ([release record](status/2026-11-27-release-33af444-corpus-v0-1-66.md)). The schema 8 checks, including step 7 in the T3 integrated browser (desktop Chromium, not a phone), are in the [`36cacaa` record](status/2026-11-27-release-36cacaa-corpus-v0-1-65-schema-8.md). Earlier checks are in their own entries, such as the [simulator entry](status/2026-11-23-ios-simulator-acceptance-2b56cc1.md) and the [deferred upgrade acceptance](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
