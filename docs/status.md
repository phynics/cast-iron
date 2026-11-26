# Status

Updated 2026-11-25. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.57`, commit `38c4d4ba735bdf636fec3e343476ee8d3c7fb2ef` (`sources/corpus.toml`), corpus format 4. Pin entry: [2026-11-26](status/2026-11-26-pin-corpus-v0-1-57.md) |
| Content version | `vd1dd4aeccdc8a81a` (`cargo xtask content` on `main` at `cea17e1b1` plus the v0.1.57 pin, 2026-11-26). The release serves `vad43d1e1b6e40e61` (corpus v0.1.51, `cargo xtask content` on `main` at `28a96e784`), so it is one pin behind `main`. |
| Library | 31,766 entries, 445 publications, 445 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 7 (`crates/schema/src/lib.rs`; the client reads 1 to 7). Schema 7 adds the haunt kind ([decision](decisions/2026-11-23-haunts-are-a-kind-schema-7.md)) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `70d528f`**, published 2026-11-25: corpus `v0.1.51`, schema 7, index format 2 ([release entry](status/2026-11-25-release-70d528f-corpus-v0-1-51-schema-7.md)). That is what Pages serves. The previous release was `2b56cc1` ([entry](status/2026-11-23-release-2b56cc1-corpus-v0-1-40-schema-6.md)).
- **`main`** is ahead of `release` by the merges after `70d528f`: the v0.1.53 corpus pin ([#719](https://github.com/phynics/cast-iron/pull/719)), the uncited-membership error ([#715](https://github.com/phynics/cast-iron/pull/715)) and the record of this release. They are not published. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **The haunt window and the promotion gate are done.** The haunt kind (schema 7) is published, and the schema upgrade check of [releasing.md](releasing.md) (old library to new shell to new library, an interrupted download, stars including a haunt star made under `rule`) passed before the promotion in Chromium and WebKit.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). The owner's physical-device and accessibility acceptance ([#12](https://github.com/phynics/cast-iron/issues/12)) is still open. The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [45550179604](https://github.com/phynics/cast-iron/actions/runs/45550179604) on `70d528f`, 2026-11-25, succeeded; all 14 jobs passed, including the four Chromium shards and the informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [45580167145](https://github.com/phynics/cast-iron/actions/runs/45580167145) for `release` at `70d528f`, 2026-11-25, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Measurements:** the integrated-browser run of 2026-11-24 on `2b56cc1` (install, cold start, search, Wizard page) is recorded in [its entry](status/2026-11-24-integrated-browser-measurements.md). These are not iPhone numbers.
- **Live verification:** on 2026-11-25 the live manifest (`vad43d1e1b6e40e61`, schema 7, corpus `v0.1.51`, 31,672 entries, 215,677,047 bytes in 277 files) and shell stamp (`2d4770843b89f710`) match the release; the shipped WASM is 2,585,921 bytes. A Playwright run in Chromium 156 and WebKit 27.2 installed the library from a fresh profile, read Goblin, Acolyte, Longsword, Bag of Holding, a trait, Power Attack, Fireball and a haunt (`ref/haunt/arcane-storm`), checked the haunt list (45) and Home's Haunts link, then reloaded and searched offline; all 29 checks passed in each browser on the second run, with no console error, page error or third-party request (the first run failed on a wrong id in the throwaway script, and in Chromium on one transient 503 during the install; see the release entry). A local upgrade check before the promotion (schema 6 library to schema 7, a starred haunt, an interrupted download) passed 73 of 73 and 79 of 79 checks in each browser; the path where the old shell had already run a library check before the app update was not exercised. No real-device check. The iOS Simulator session of 2026-11-23 was on the previous release ([simulator entry](status/2026-11-23-ios-simulator-acceptance-2b56cc1.md)). The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
