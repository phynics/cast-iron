<img src="assets/icon.svg" width="64" height="64" alt="">

# Cast Iron

Cast Iron is an offline rules reference for Pathfinder First Edition (PF1e), delivered as an installable progressive web app (PWA). It is written in Rust and compiled to WebAssembly with Leptos. After installation it works with no network connection. It has no backend. The project is unofficial and is a proof of concept (POC): the content covers only some of the PF1e books.

## What works now

- Install the app to a device, then use it offline.
- Browse rules entries by category and by source book.
- Search titles and full text, with filters for the kinds of entry that are selected.
- Read formatted entries, with a References section that links to the entries they cite.
- Bookmark entries and organize bookmarks into collections.
- See recently viewed entries on the home screen.
- Choose light, dark, or system theme.
- Read the Legal and attribution page, which includes the content coverage report.

## Quick start

Prerequisites:

- [rustup](https://rustup.rs/). `rust-toolchain.toml` pins the toolchain.
- Trunk, which builds the WebAssembly app: `cargo install trunk --locked`.
- Node.js, only to run the browser acceptance tests.
- `curl`, `tar`, and `perl` (used by `cargo xtask fetch-sources` and the Trunk post-build hook).

Run these commands from the repository root:

| Task | Command |
|---|---|
| Download the pinned dataset | `cargo xtask fetch-sources` |
| Compile content into `generated/` | `cargo xtask content` |
| Build the site into `dist/` | `cargo xtask build-site` (use `--public-url /` to serve from a root path) |
| Run all checks | `cargo xtask check` |
| Install the browser test dependencies | `npm --prefix tests/e2e ci` |
| Run the browser acceptance tests | `npm --prefix tests/e2e test` |

`cargo xtask build-site` fetches and compiles the content when `generated/content/manifest.json` is missing. The default public URL is `/cast-iron/`, the GitHub Pages project path.

## Repository layout

See [AGENTS.md](AGENTS.md) for the directory layout, the rules that the code must follow, and the working pattern for changes.

## Content and licensing

The content comes from [PSRD-Data](https://github.com/devonjones/PSRD-Data), pinned to commit `76ea4784604145ba3dea40eb56f83bb904759467` in `sources/psrd-data/source.toml`. PSRD-Data is derived from Paizo's Pathfinder Reference Document, which is published under the Open Game License 1.0a (OGL 1.0a) and Paizo's Community Use Policy. PSRD-Data has no license file of its own. The audit in `docs/research/licensing-audit.md` records the facts and open risks.

The POC does not include every PF1e book. Which publications are included, and which upstream records were excluded or rejected, is listed in the coverage report at `generated/reports/coverage.md`, which `cargo xtask content` writes. The app shows the same report on its Legal and attribution page.

The Legal and attribution page in the app shows the Community Use notice, the Product Identity declaration, the OGL 1.0a text, the Section 15 copyright notices, and the coverage report. The licenses for this repository are listed in [LICENSES/README.md](LICENSES/README.md).

Community Use notice, as the Paizo Community Use Policy requires:

> Cast Iron uses trademarks and/or copyrights owned by Paizo Inc., used under Paizo's Community Use Policy (paizo.com/licenses/communityuse). We are expressly prohibited from charging you to use or access this content. Cast Iron is not published, endorsed, or specifically approved by Paizo. For more information about Paizo Inc. and Paizo products, visit paizo.com.

Cast Iron is not affiliated with or endorsed by Paizo Inc. It does not use the Pathfinder logo.

Contact the maintainer through [GitHub Issues](https://github.com/phynics/cast-iron/issues) for questions, corrections, or licensing concerns.

Created in [T3 Code](https://t3.codes).
