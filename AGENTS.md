# Cast Iron: agent guide

Cast Iron is an offline-first PWA reference for the Pathfinder First Edition rules. It is Rust/WASM (Leptos, client-side rendering) on GitHub Pages, with no backend.

- Product source of truth: `docs/spec.md`. Do not rewrite it.
- Decisions that refine the spec: `docs/decisions.md`. Add an entry for each decision that a later reader cannot derive from code.
- Development tracking: [GitHub Issues](https://github.com/phynics/cast-iron/issues). Search before creating an issue. Link each implementation PR to its issue. See `CONTRIBUTING.md`.
- Integration and publication: `main` integrates work; `release` publishes Pages. Follow `docs/releasing.md` for explicit promotion, verification, and recovery. Never auto-promote main or reset release history.
- Current state, corpus counts, and acceptance results: `docs/status.md`. Distinguish local verification, CI, and deployment.
- Book-order evidence: `docs/research/book-order-evidence.md`. Read it before changing `sources/psrd-data/book-order.toml`; website menus and dataset order do not establish printed order.

## Commands

Run all commands from the repository root. `rust-toolchain.toml` pins the toolchain. `~/.cargo/bin` can be missing from `PATH`; `cargo xtask` handles that case.

On Nix hosts, run each command inside the dev shell: `devenv shell -- <command>`. It provides the pinned toolchain, Trunk, Node, and Playwright browsers. Do not run `playwright install` or `cargo install trunk` there.

| Task | Command |
|---|---|
| Install Trunk (once) | `cargo install trunk --locked` |
| Get the pinned datasets | `cargo xtask fetch-sources` |
| Compile the canonical library (default) | `cargo xtask content` (writes `generated/content/`) |
| Compile the PSRD carry-over library | `cargo xtask content --psrd` (writes `generated/psrd/`) |
| Build the site into `dist/` | `cargo xtask build-site` (canonical default; `--psrd` for the PSRD library; `--public-url /` for a root path) |
| Test the PSRD build | `CAST_IRON_CONTENT=generated/psrd npm --prefix tests/e2e test -- ...` |
| Check before you finish | `cargo xtask check` |
| Browser acceptance tests | `npm --prefix tests/e2e ci`, then `npm --prefix tests/e2e test -- --project=chromium --project=webkit --project=mobile-chromium` |

To prove a change in the real app, use the `verify` skill in `.agents/skills/verify/SKILL.md`.

## Layout

| Path | Contents |
|---|---|
| `crates/schema` | The contract between the compiler and the client: content types, validation |
| `crates/content-compiler` | Native CLI: PSRD-Data adapter, HTML to rich content, reports |
| `crates/search` | Index generation and query engine, for native and WASM. No dependency on `schema` |
| `crates/web` | Leptos client |
| `crates/xtask` | Build automation |
| `web/` | `index.html`, `sw.js`, manifest, CSS, icons |
| `sources/psrd-data/` | Dataset pin, reviewed selection, book order, id overrides, legal notices |
| `sources/pathfinder-data-1-e/` | Supplemental pin, approval gate (`approved.json`), out-of-scope list |
| `scripts/` | Source selection, publication, upgrade, and audit checks |
| `LICENSE`, `LICENSES/` | MIT code license and content/upstream notices |
| `generated/`, `dist/`, `.cache/` | Build output. Never commit these directories |
| `tests/e2e` | Playwright acceptance tests |

## Rules

1. Offline is the product. Nothing at runtime may contact a third-party host: no CDN, no remote font, no telemetry. After installation, ordinary use makes no network request at all.
2. Client rendering uses typed nodes only. Never add `inner_html` or raw upstream HTML. This is the XSS defense.
3. `EntryId` values are public, permanent identifiers, because bookmarks store them. If an upstream change alters a PSRD id, add a mapping in `sources/psrd-data/id-overrides.toml`. Published canonical ids are `ref/` ids; a legacy id resolves through `sources/canonical/id-map.toml` and never gets a second meaning.
4. A breaking change to serialized content must increase `SCHEMA_VERSION`. A breaking change to the search format must increase `INDEX_FORMAT_VERSION`.
5. Compiler output must stay byte-identical between runs: no timestamps, no hash-map iteration order.
6. Excluded or rejected material must appear in the coverage report. Never drop it silently.
7. A partly installed library must never show as ready. Verify hashes before activation. The active library stays usable until a replacement is verified.
8. JavaScript stays limited to `web/sw.js` and the small bridge in `crates/web/js/`. Put logic that can be tested natively in Rust.
9. All asset URLs are relative to the document base, because the app runs under a project path (`/cast-iron/`).
10. Do not ship artwork. Paizo Product Identity (names, deities, characters, setting prose) ships only under Paizo's Community Use Policy: every entry carries the `legal#community-use` notice path, and the legal page keeps the Community Use notice. Never imply official status (rule 11). Read `docs/research/licensing-audit.md` before adding a source, plus `docs/research/pf-data-licensing.md` for anything from the canonical Pf Data corpus. Keep the legal page data complete for both dataset pins and `LICENSES/README.md` accurate.
11. The app must not use the Pathfinder logo, and must not claim official status or compatibility.
12. PSRD supplemental records enter its library only through `sources/pathfinder-data-1-e/approved.json`. Canonical Pf Data records enter through the screened inventory and the reviewed `sources/canonical/publications.toml` registry. Never copy upstream renderer code or artwork; ORC-licensed PF2 material stays out of scope via `out-of-scope.json`.

## Working pattern

Start new work from current `origin/main` on an issue-linked branch. Keep outstanding work in issues. Docs retain decisions, limitations, and evidence.

In a T3 Code worktree (a path under `~/.t3/worktrees/`), the branch already exists: commit and push on it, and do not create, rename or switch branches. T3 removes a worktree only while it stays on the branch T3 recorded. Link the issue in the PR body instead of the branch name. To start from current `origin/main`, run `git fetch origin && git reset --hard origin/main` before you change any file.

Work in small commits, each with a verifiable state. A task is finished only after `cargo xtask check` passes and, for user-visible behavior, the affected e2e test passes on a real `dist/` build. Report failures with their output.

Open PRs against `main` and use `Closes #N` only for completed acceptance, otherwise `Refs #N`. A merged PR is not a release. Record exact CI and Pages runs separately from local and live checks.

After changing compiler code, schema types, or source configuration, run `cargo xtask content` (canonical default) before `cargo xtask build-site`. Run `cargo xtask content --psrd` as well when the PSRD carry-over input or the previous published library is affected. The site build reuses an existing content manifest; it does not detect stale generated content. For corpus or schema changes, follow the upgrade checks in the `verify` skill and retain the last published PSRD library outside build-output directories.
