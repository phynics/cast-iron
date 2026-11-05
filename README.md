<img src="assets/icon.svg" width="64" height="64" alt="">

# Cast Iron

Cast Iron is an offline rules reference for Pathfinder First Edition (PF1e), delivered as an installable progressive web app (PWA). It is written in Rust and compiled to WebAssembly with Leptos. After installation it works with no network connection. It has no backend. The project's code is MIT-licensed, code only (see [LICENSE](LICENSE); the full licensing picture, including OGL content, is in [LICENSES/README.md](LICENSES/README.md)). The project is unofficial.

The default build is the canonical Pf Data 1e library: about 29,500 entries from the publications in the pinned corpus, with PSRD-Data entries carried over where the canonical source has no equivalent. The PSRD-Data library remains buildable with `cargo xtask content --psrd` for comparison and carry-over inspection. Neither library contains every published PF1e book or every standalone record family. See [status](docs/status.md) for local verification, the default switch, and deployment state.

## What works now

- Install the app to a device, then use it offline.
- Browse categories with alphabetical lists and facets, with separate Classes and Prestige classes categories.
- Follow races to racial traits, classes to archetypes, and feats to named prerequisites and chains.
- Browse spell lists by class and level, then filter the list by name.
- Open book contents and move between entries in the curated order. Source notes identify verified chapter order and remaining placement questions.
- Search titles and full text, with category, source, and applicable field filters.
- Read formatted entries, with a References section that links to the entries they cite.
- Find every starred entry on Home, open it, or remove its star.
- Activate or disable installed books to control ordinary discovery without deleting downloads or stars.
- See recently viewed entries on the home screen.
- Choose light, dark, or system theme.
- Read the Legal and attribution page, which includes the content coverage report.

The book view aims to follow original chapter and section order. The Core Rulebook's 15 main chapters match the publisher's contents PDF. Other outlines, nested placement, and appendix grouping still need verification. Feat links preserve prerequisite wording and alternatives; they do not determine whether a character qualifies. See [book evidence](docs/research/book-order-evidence.md) and [relationship coverage](docs/research/relationship-data.md).

## Quick start

Prerequisites:

- [rustup](https://rustup.rs/). `rust-toolchain.toml` pins the toolchain.
- Trunk, which builds the WebAssembly app: `cargo install trunk --locked`.
- Node.js, only to run the browser acceptance tests.
- Python 3.11 or newer for source selection, publication checks, and mapping tests in `cargo xtask check`.
- `curl`, `tar`, and `perl` (used by `cargo xtask fetch-sources` and the Trunk post-build hook).

With [devenv](https://devenv.sh/) (Nix), `devenv shell` provides all of these at the CI versions: the pinned Rust toolchain, Trunk 0.22.1, `wasm-opt`, Node 24, Python, Perl, and Playwright browsers built by Nix. With direnv, `direnv allow` loads the shell on entry. In that shell, skip `playwright install`. If you update `@playwright/test`, also update `devenv.lock` (`devenv update`) so the Nix browser version matches; the shell refuses to start otherwise.

Run these commands from the repository root:

| Task | Command |
|---|---|
| Download the pinned datasets | `cargo xtask fetch-sources` |
| Compile the canonical library into `generated/content/` | `cargo xtask content` |
| Compile the PSRD carry-over library into `generated/psrd/` | `cargo xtask content --psrd` |
| Build the site into `dist/` | `cargo xtask build-site` (canonical default; `--psrd` for the PSRD library; `--public-url /` to serve from a root path) |
| Run all checks | `cargo xtask check` |
| Install the browser test dependencies | `npm --prefix tests/e2e ci` |
| Install the locally verified browsers | `npx --prefix tests/e2e playwright install chromium webkit` |
| Run the locally verified browser projects | `npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium` |

`cargo xtask build-site` fetches both datasets and compiles the canonical library when `generated/content/manifest.json` is missing. The default public URL is `/cast-iron/`, the GitHub Pages project path. For the PSRD library, build `generated/psrd/` with `--psrd` and run the browser suite with `CAST_IRON_CONTENT=generated/psrd`.

After a compiler, schema, or source change, run `cargo xtask content` explicitly before rebuilding. An existing manifest makes the site build reuse the generated library. Browser tests read expectations from `generated/` and run against `dist/`, so both must describe the same version. The [verification guide](.agents/skills/verify/SKILL.md) covers offline checks, screenshots, and library upgrades. The default browser test command includes Firefox, which has a known local startup failure recorded in [status](docs/status.md).

## Development and releases

[GitHub Issues](https://github.com/phynics/cast-iron/issues) tracks new work.
Follow [the contributor guide](CONTRIBUTING.md) from issue to branch and PR.
`main` integrates changes; Pages publication uses explicit promotion to `release`.
See [the release guide](docs/releasing.md) for promotion, verification, recovery,
and the migration state.

## Repository layout

See [AGENTS.md](AGENTS.md) for the directory layout, the rules that the code must follow, and the working pattern for changes.

## Content and licensing

The canonical content comes from [pathfinder-data-1-e](https://github.com/jasontankapps/pathfinder-data-1-e), pinned to commit `08ea71f24375908d0190c822fd1fd080797667a9` in `sources/pathfinder-data-1-e/source.json`, screened by `pi-screen.json`, and registered through `sources/canonical/publications.toml`. PSRD-Data, pinned to commit `76ea4784604145ba3dea40eb56f83bb904759467` in `sources/psrd-data/source.toml`, remains the carry-over source and the previous published library. Both datasets are derived from Paizo's Pathfinder Reference Document material, used under the Open Game License 1.0a (OGL 1.0a) for Open Game Content and Paizo's Community Use Policy for permitted descriptive references. PSRD-Data has no license file of its own. The Pf Data source places its `json/` game data under OGL 1.0a, keeps its non-OGC application code under GPL-3.0-or-later (not copied here), and carries ORC-licensed PF2 material that is explicitly excluded from this project. The audits in `docs/research/licensing-audit.md` and `docs/research/pf-data-licensing.md` record the facts and open risks. The project's own code is MIT-licensed; see [LICENSE](LICENSE) and [LICENSES/README.md](LICENSES/README.md) for how the licenses combine.

The canonical library covers every publication with an imported record in the pinned Pf Data corpus, including the 16 PSRD books through canonical entries or carry-over. The pinned sources still do not contain every published PF1e book; later books such as Occult Adventures or Pathfinder Unchained are absent. Which publications are included, and which upstream records were excluded or rejected, is listed in the coverage report at `generated/reports/coverage.md`, which `cargo xtask content` writes. The app shows the same report on its Legal and attribution page.

The Legal and attribution page in the app shows the Community Use notice, the Product Identity declaration, the OGL 1.0a text, the Section 15 copyright notices, and the coverage report. The licenses for this repository are listed in [LICENSES/README.md](LICENSES/README.md).

Community Use notice, as the Paizo Community Use Policy requires:

> Cast Iron uses trademarks and/or copyrights owned by Paizo Inc., used under Paizo's Community Use Policy (paizo.com/licenses/communityuse). We are expressly prohibited from charging you to use or access this content. Cast Iron is not published, endorsed, or specifically approved by Paizo. For more information about Paizo Inc. and Paizo products, visit paizo.com.

Cast Iron is not affiliated with or endorsed by Paizo Inc. It does not use the Pathfinder logo.

Contact the maintainer through [GitHub Issues](https://github.com/phynics/cast-iron/issues) for questions, corrections, or licensing concerns.

To regenerate the PSRD carry-over selection from its pinned snapshot, run `python3 scripts/expand-sources.py`, then `cargo xtask content --psrd` and build with `--psrd`. This inventories only the reviewed 16 books and refuses a changed book set. Canonical Pf Data records enter through the screened inventory and the reviewed publication registry; the PSRD supplemental record still enters only through `sources/pathfinder-data-1-e/approved.json`. See `docs/data-migration.md`.
