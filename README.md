<img src="assets/icon.svg" width="64" height="64" alt="">

# Cast Iron

Cast Iron is an offline rules reference for Pathfinder First Edition (PF1e), delivered as an installable progressive web app (PWA). It is written in Rust and compiled to WebAssembly with Leptos. After installation it works with no network connection. It has no backend. The project is unofficial. This branch contains 9,967 entries across the 16 books available in the pinned dataset. It does not contain every published PF1e book or every standalone record family. See [status](docs/status.md) for local verification and deployment state.

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

Run these commands from the repository root:

| Task | Command |
|---|---|
| Download the pinned dataset | `cargo xtask fetch-sources` |
| Compile content into `generated/` | `cargo xtask content` |
| Build the site into `dist/` | `cargo xtask build-site` (use `--public-url /` to serve from a root path) |
| Run all checks | `cargo xtask check` |
| Install the browser test dependencies | `npm --prefix tests/e2e ci` |
| Install the locally verified browsers | `npx --prefix tests/e2e playwright install chromium webkit` |
| Run the locally verified browser projects | `npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium` |

`cargo xtask build-site` fetches and compiles the content when `generated/content/manifest.json` is missing. The default public URL is `/cast-iron/`, the GitHub Pages project path.

After a compiler, schema, or source change, run `cargo xtask content` explicitly before rebuilding. An existing manifest makes the site build reuse the generated library. Browser tests read expectations from `generated/` and run against `dist/`, so both must describe the same version. The [verification guide](.claude/skills/verify/SKILL.md) covers offline checks, screenshots, and library upgrades. The default browser test command includes Firefox, which has a known local startup failure recorded in [status](docs/status.md).

## Development and releases

[GitHub Issues](https://github.com/phynics/cast-iron/issues) tracks new work.
Follow [the contributor guide](CONTRIBUTING.md) from issue to branch and PR.
`main` integrates changes; Pages publication uses explicit promotion to `release`.
See [the release guide](docs/releasing.md) for promotion, verification, recovery,
and the migration state.

## Repository layout

See [AGENTS.md](AGENTS.md) for the directory layout, the rules that the code must follow, and the working pattern for changes.

## Content and licensing

The content comes from [PSRD-Data](https://github.com/devonjones/PSRD-Data), pinned to commit `76ea4784604145ba3dea40eb56f83bb904759467` in `sources/psrd-data/source.toml`. PSRD-Data is derived from Paizo's Pathfinder Reference Document, which is published under the Open Game License 1.0a (OGL 1.0a) and Paizo's Community Use Policy. PSRD-Data has no license file of its own. The audit in `docs/research/licensing-audit.md` records the facts and open risks.

The library covers Core Rulebook, Advanced Player's Guide, Advanced Race Guide, Bestiary 1 through 4, Game Mastery Guide, Monster Codex, Mythic Adventures, NPC Codex, Technology Guide, Ultimate Campaign, Ultimate Combat, Ultimate Equipment, and Ultimate Magic. The pinned snapshot does not contain later books such as Occult Adventures or Pathfinder Unchained. Which publications are included, and which upstream records were excluded or rejected, is listed in the coverage report at `generated/reports/coverage.md`, which `cargo xtask content` writes. The app shows the same report on its Legal and attribution page.

The Legal and attribution page in the app shows the Community Use notice, the Product Identity declaration, the OGL 1.0a text, the Section 15 copyright notices, and the coverage report. The licenses for this repository are listed in [LICENSES/README.md](LICENSES/README.md).

Community Use notice, as the Paizo Community Use Policy requires:

> Cast Iron uses trademarks and/or copyrights owned by Paizo Inc., used under Paizo's Community Use Policy (paizo.com/licenses/communityuse). We are expressly prohibited from charging you to use or access this content. Cast Iron is not published, endorsed, or specifically approved by Paizo. For more information about Paizo Inc. and Paizo products, visit paizo.com.

Cast Iron is not affiliated with or endorsed by Paizo Inc. It does not use the Pathfinder logo.

Contact the maintainer through [GitHub Issues](https://github.com/phynics/cast-iron/issues) for questions, corrections, or licensing concerns.

Created in [T3 Code](https://t3.codes).

To regenerate the reviewed selection from the pinned snapshot, run `python3 scripts/expand-sources.py`, then `cargo xtask content` and rebuild the site. This inventories only the reviewed 16 books and refuses a changed book set.
