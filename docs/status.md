# Status

Updated 2026-11-23. This page holds the current state and nothing else. The history is a log with one file per entry: [`docs/status/`](status/README.md) (pins, releases, acceptance records). Decisions are in [`docs/decisions/`](decisions/README.md). Open work is in [GitHub Issues](https://github.com/phynics/cast-iron/issues).

Edit this page only where a change makes a line untrue, and add a log file for the event (`docs/status/YYYY-MM-DD-<slug>.md`). Keep local verification, CI and deployment apart.

## Pinned state of `main`

| | |
|---|---|
| Corpus | `phynics/cast-iron-data` tag `v0.1.40`, commit `ca316adfc791b7e090e76184a780496f60dcd861` (`sources/corpus.toml`), corpus format 3. Pin entry: [2026-11-23](status/2026-11-23-pin-corpus-v0-1-40.md) |
| Content version | `v3f3d0b708758820d` (`cargo xtask content` on `main` at `473dc3f57` plus the v0.1.40 pin, 2026-11-23). The release serves the same content version (corpus v0.1.40). |
| Library | 31,742 entries, 443 publications, 443 books; 0 unreachable entries, 0 broken links |
| `SCHEMA_VERSION` | 6 (`crates/schema/src/lib.rs`; the client reads 1 to 6) |
| `INDEX_FORMAT_VERSION` | 2 (`crates/search/src/lib.rs`) |

## Main and release

- **`release` is at `2b56cc1`**, published 2026-11-23: corpus `v0.1.40`, schema 6, index format 2 ([release entry](status/2026-11-23-release-2b56cc1-corpus-v0-1-40-schema-6.md)). That is what Pages serves. The previous release was `9a6fb06` ([entry](status/2026-11-21-release-9a6fb06-corpus-v0-1-37-schema-6.md)).
- **`main`** is ahead of `release`: the merges after `2b56cc1` (#638, #639 and the record of this release) are not published. Promotion follows [releasing.md](releasing.md); `main` never publishes by itself.
- **No open gate** beyond [releasing.md](releasing.md): [#543](https://github.com/phynics/cast-iron/issues/543) (a saved Continue Reading point can name a book that no longer holds the entry) and [#546](https://github.com/phynics/cast-iron/issues/546) (book section anchors followed tree positions) are fixed and released by [#547](https://github.com/phynics/cast-iron/pull/547) and [#549](https://github.com/phynics/cast-iron/pull/549). The class-option power check is an error from the v0.1.35 pin: 0 power errors, 19 powers excepted in `power-exceptions.toml` ([#557](https://github.com/phynics/cast-iron/issues/557)).

## Latest runs

- **Full CI on `main`:** run [45181702594](https://github.com/phynics/cast-iron/actions/runs/45181702594) on `2b56cc1`, 2026-11-23, succeeded; all 14 jobs passed, including the four Chromium shards and the informational WebKit shards. All runs: [ci.yml on `main`](https://github.com/phynics/cast-iron/actions/workflows/ci.yml?query=branch%3Amain).
- **Pages:** the last deployment is [45209601850](https://github.com/phynics/cast-iron/actions/runs/45209601850) for `release` at `2b56cc1`, 2026-11-23, succeeded. All runs: [pages.yml](https://github.com/phynics/cast-iron/actions/workflows/pages.yml).
- **Live verification:** on 2026-11-23 the live manifest (`v3f3d0b708758820d`, corpus `v0.1.40`, 217,014,990 bytes in 280 files) and shell stamp (`114f05ccdf222d2a`) match the release; the shipped WASM is 2,490,876 bytes. A Playwright run in Chromium and WebKit installed the library from a fresh profile and read Goblin, Acolyte (spell block), Longsword (Weapon Group link), Bag of Holding (Construction), a trait and Power Attack (`Sources:` lines with pages) and Fireball, then reloaded and searched offline; all 35 checks passed in each browser with no console error, page error or third-party request (see the release entry; early failures were mistakes in the throwaway script). A local upgrade check before the promotion (old library to new, existing profile) passed 65 of 65 checks in Chromium 156 and WebKit 27.2; the path where the old shell had already run a library check before the app update was not exercised. No real-device check and no Home Screen app. The deferred upgrade acceptance from the published schema 4 release is in [the acceptance entry](status/2026-11-18-deferred-upgrade-acceptance-6365dba-to-1324e1d.md).
