# Status

Updated 2026-11-26. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.61`, commit `2eb078173ac19deee2686dda9570dd9a6e896c6e` (`sources/corpus.toml`), corpus format 4. Pin entry: [2026-11-26](status/2026-11-26-pin-corpus-v0-1-61.md) |
| Content version | `v464ed33579599e94` (`cargo xtask content` on `main` at `844c66cd3` plus #737 and the v0.1.61 pin, 2026-11-26). The release serves the same content (the live manifest reads `vaaeb1e4c7340254b`). |
| Library | 31,766 entries, 445 publications, 445 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 7 (`crates/schema/src/lib.rs`; the client reads 1 to 7). Schema 7 adds the haunt kind ([decision](decisions/2026-11-23-haunts-are-a-kind-schema-7.md)) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `feb3554`**, published 2026-11-26: corpus `v0.1.59`, schema 7, index format 2 ([release entry](status/2026-11-26-release-feb3554-corpus-v0-1-59-schema-7.md)). That is what Pages serves. The app shell is unchanged from the previous release, `edcb421` ([entry](status/2026-11-26-release-edcb421-corpus-v0-1-58-schema-7.md)).
- **`main`** is at the release commit plus the record of this release. Nothing else is unpublished. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **The haunt window and the promotion gate are done.** The haunt kind (schema 7) is published, and the schema upgrade check of [releasing.md](releasing.md) (old library to new shell to new library, an interrupted download, stars including a haunt star made under `rule`) passed before the promotion in Chromium and WebKit.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). [#12](https://github.com/phynics/cast-iron/issues/12) (physical-device and accessibility acceptance) was closed by the owner on 2026-11-25 without a real-device check. The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [45821132156](https://github.com/phynics/cast-iron/actions/runs/45821132156) on `feb3554`, 2026-11-26, succeeded; all 14 jobs passed, including the four Chromium shards and the informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [45829830477](https://github.com/phynics/cast-iron/actions/runs/45829830477) for `release` at `feb3554`, 2026-11-26, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Measurements:** the integrated-browser run of 2026-11-24 on `2b56cc1` (install, cold start, search, Wizard page) is recorded in [its entry](status/2026-11-24-integrated-browser-measurements.md). These are not iPhone numbers.
- **Live verification:** on 2026-11-26 the live manifest (`vaaeb1e4c7340254b`, schema 7, corpus `v0.1.59`, 31,766 entries, 215,286,180 bytes in 276 files) and shell stamp (`a5d3b8e5a478002d`, unchanged) match the release; the shipped WASM is 2,585,921 bytes, unchanged. A Playwright run in Chromium 156 and WebKit 27.2 installed the library from a fresh profile, read the entries of the previous check plus the archetype Invested Regent (2 feature heads, 50 list items), then reloaded and searched offline; all 30 checks passed in each browser, with no console error, page error, response of 400 or more or third-party request (Chromium's first run failed one check on a mistake in the throwaway script, and a repeat before the fix gave the same result; see the release entry). A local upgrade check before the promotion (existing profile, interrupted download and retry, saved stars including a haunt star, the 28 chosen books and Continue Reading) passed 85 of 85 checks in each browser; it did not run the deployed old shell or the daily update check. No real-device check. The iOS Simulator session of 2026-11-23 was on an earlier release ([simulator entry](status/2026-11-23-ios-simulator-acceptance-2b56cc1.md)). The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
