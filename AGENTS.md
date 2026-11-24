# Cast Iron: agent guide

Cast Iron is an offline-first PWA reference for the Pathfinder First Edition rules. It is Rust/WASM (Leptos, client-side rendering) on GitHub Pages, with no backend.

- Product source of truth: `docs/spec.md`. Do not rewrite it.
- Decisions that refine the spec: one file each in `docs/decisions/` (rules and front matter: `docs/decisions/README.md`). Add a file `docs/decisions/YYYY-MM-DD-<slug>.md` for each decision that a later reader cannot derive from code. Do not edit another entry's text; when a decision replaces one, set the older entry's `status` and `superseded_by`. No list of entries is committed: `cargo xtask decisions-index` renders the index into `target/docs-index/`.
- Development tracking: [GitHub Issues](https://github.com/phynics/cast-iron/issues). Search before creating an issue. Link each implementation PR to its issue. See `CONTRIBUTING.md`.
- Integration and publication: `main` integrates work; `release` publishes Pages. Follow `docs/releasing.md` for explicit promotion, verification, and recovery. Never auto-promote main or reset release history.
- Current state (pinned corpus, format versions, main versus release, open release gates, latest runs): `docs/status.md`, one screen. The log of pins, releases and acceptance results is one file each in `docs/status/`: a pin or release PR adds `docs/status/YYYY-MM-DD-<slug>.md` and edits only the few lines of `docs/status.md` that its change makes untrue. Distinguish local verification, CI, and deployment.
- Book-order evidence: `docs/research/book-order-evidence.md`. Read it before changing a curated book (`books/<publication-id>.toml`, or `books.toml` before corpus format 3) in the corpus; website menus and dataset order do not establish printed order.

## Commands

Run all commands from the repository root. `rust-toolchain.toml` pins the toolchain. `~/.cargo/bin` can be missing from `PATH`; `cargo xtask` handles that case.

On Nix hosts, run each command inside the dev shell: `devenv shell -- <command>`. It provides the pinned toolchain, Trunk, Node, and Playwright browsers. Do not run `playwright install` or `cargo install trunk` there.

| Task | Command |
|---|---|
| Install Trunk (once) | `cargo install trunk --locked` |
| Get the pinned corpus | `cargo xtask fetch-sources` (downloads `phynics/cast-iron-data`; verifies the tree hash in `sources/corpus.toml`) |
| Compile the library | `cargo xtask content` (reads the pinned corpus; writes `generated/content/`) |
| Build the site into `dist/` | `cargo xtask build-site` (`--public-url /` for a root path) |
| Check the library and WASM sizes against the budget | `cargo xtask size-budget` (after `build-site`; limits in `sources/size-budget.toml`; see `docs/releasing.md`) |
| Regenerate the canonical top-50 ranking golden (every corpus-pin PR, after `cargo xtask content`) | `cargo xtask search-golden` (rewrites `crates/search/tests/golden/canonical-top50.txt`; read its diff for ranking regressions. `cargo test -p cast-iron-search` fails when the golden names another corpus commit than `sources/corpus.toml`) |
| Print a tree hash (to bump the corpus pin) | `cargo xtask corpus-tree-hash <dir>` |
| Add the ids of a published library to the guard | `cargo xtask published-ids` (after `cargo xtask content`; see `docs/releasing.md`) |
| Give new books their share slot | `cargo xtask published-books` (`cargo xtask content` fails until each book of the pinned corpus has a line in `sources/published-books.txt`; commit the added lines, never edit one) |
| Read every entry of a compiled library back with the client's reader | `cargo xtask entries-round-trip [dir]` (default `generated/content`; `cargo xtask content` runs it after the compile; give the directory of a retained released library to prove that an installed library still reads; `docs/decisions/2026-11-22-tagged-enums-read-tag-first.md`) |
| Validate a corpus | `cargo xtask corpus-check <dir>` |
| Put a corpus into canonical form | `cargo xtask corpus-fmt <dir>` (`--check` to only list differences) |
| Split a corpus's `books.toml` into the curated `books/` files (format 3 layout, idempotent) | `cargo xtask corpus-migrate-layout <dir>` |
| Declare corpus format 4 (feature facts, #540): rewrites only `corpus.toml`, idempotent | `cargo xtask corpus-migrate-features <dir>` (`cargo xtask corpus-migrate` runs it after the older steps) |
| Lower a corpus's trades to format 2 | `cargo xtask corpus-migrate-trades <dir>` (idempotent; writes the review queue `generated/trade-review.tsv`; the pinned corpus has had format 2 trades since v0.1.33 and is format 3 since v0.1.36; the command converts an older corpus) |
| Check before you finish | `cargo xtask check` (also fails when the verify skill's feature map and the specs in `tests/e2e/tests/` disagree: a spec or test the map does not reach, a name or `--grep` pattern that matches nothing; and when `docs/research/aon-path-coverage.{md,tsv}` is stale against the pinned corpus, which prints `cargo xtask aon-coverage` as the fix; the step is skipped, with a note, when `.cache/sources/cast-iron-data` is absent) |
| Browser acceptance test (at most one spec; see "Test scope") | `npm --prefix tests/e2e ci`, then `npm --prefix tests/e2e test -- --project=chromium tests/<name>.spec.ts` |
| Full browser suite (CI only) | `gh workflow run ci.yml --ref <branch>` |

To prove a change in the real app, use the `verify` skill in `.agents/skills/verify/SKILL.md`.

The retired upstream import and the PSRD library build are preserved at the git tag `psrd-last-buildable`. `docs/releasing.md` says when to use it.

## Layout

| Path | Contents |
|---|---|
| `crates/schema` | The contract between the compiler and the client: content types, validation |
| `crates/content-compiler` | Native CLI: compiles the corpus into the content library, writes reports |
| `crates/corpus` | The plain-text corpus format: reader, writer, check, formatter (`docs/corpus-format.md`) |
| `crates/search` | Index generation and query engine, for native and WASM. No dependency on `schema` |
| `crates/sort` | One non-generic stable sort. Non-test code in `web`, `schema` and `search` uses it instead of `slice::sort*`, so the WASM holds one copy (`docs/decisions/2026-11-22-one-stable-sort-helper.md`). The same code builds `BTreeMap` and `BTreeSet` with `map_of`, `set_of`, `insert` or `extend`, never `collect()` or `From`, and a unit effect with `views::util::effect` (`docs/decisions/2026-11-22-wasm-cuts-without-a-speed-cost.md`) |
| `crates/web` | Leptos client |
| `crates/xtask` | Build automation |
| `web/` | `index.html`, `sw.js`, manifest, CSS, icons |
| `docs/decisions/`, `docs/status/` | One Markdown file per decision and per status log entry, each with front matter and a static `README.md` (no list of entries). `scripts/split-docs.py` made them from the old single files |
| `sources/corpus.toml` | Pin of the plain-text corpus (`phynics/cast-iron-data`): tag, commit, tree hash. To bump it: tag the corpus, update `commit` and `tree_sha256` (`cargo xtask corpus-tree-hash <extracted dir>`), run `cargo xtask fetch-sources` and `cargo xtask content`, then `cargo xtask search-golden` and commit the golden |
| `scripts/` | Publication screen and release promotion scripts |
| `LICENSE`, `LICENSES/` | MIT code license, OGL text, and the licensing of the corpus |
| `generated/`, `dist/`, `.cache/` | Build output. Never commit these directories |
| `tests/e2e` | Playwright acceptance tests |

## Rules

1. Offline is the product. Nothing at runtime may contact a third-party host: no CDN, no remote font, no telemetry. After installation, ordinary use makes no network request at all. The one exception is the opt-out, same-origin daily update check that fetches only `content/manifest.json` (#206), only when online, and never downloads the library automatically.
2. Client rendering uses typed nodes only. Never add `inner_html` or raw upstream HTML. This is the XSS defense.
3. `EntryId` values are public, permanent identifiers, because bookmarks store them. Never edit an `id` or move an entry file in the corpus. Published ids are `ref/` ids; a legacy id resolves through `aliases.toml` in the corpus and never gets a second meaning. `sources/published-ids.txt` lists every id a published library used; `cargo xtask content` fails when one stops resolving, and the file only grows.
4. A breaking change to serialized content must increase `SCHEMA_VERSION`. A breaking change to the search format must increase `INDEX_FORMAT_VERSION`.
5. Compiler output must stay byte-identical between runs: no timestamps, no hash-map iteration order.
6. Excluded or rejected material must appear in the coverage report. Record it in `exclusions.toml` in the corpus; never drop it silently.
7. A partly installed library must never show as ready. Verify hashes before activation. The active library stays usable until a replacement is verified.
8. JavaScript stays limited to `web/sw.js` and the small bridge in `crates/web/js/`. Put logic that can be tested natively in Rust.
9. All asset URLs are relative to the document base, because the app runs under a project path (`/cast-iron/`).
10. Do not ship artwork. Paizo Product Identity (names, deities, characters, setting prose) ships only under Paizo's Community Use Policy: every entry carries the `legal#community-use` notice path, and the legal page keeps the Community Use notice. Never imply official status (rule 11). Read `docs/research/licensing-audit.md` before adding a source, plus `docs/research/pf-data-licensing.md` for anything from the Pf Data corpus. Keep `legal.toml` in the corpus complete (Section 15 notices, dataset pins) and `LICENSES/README.md` accurate.
11. The app must not use the Pathfinder logo, and must not claim official status or compatibility.
12. Entries enter the library only by editing the corpus repository (`phynics/cast-iron-data`) and bumping the pin in `sources/corpus.toml`. Each new publication needs a row in the corpus `publications.toml` with attribution. OGL sources need their exact Section 15 notice or a `notice_pending` decision. Paizo Blog text covered by the Community Use Policy may use an explicit CUP-only basis in `legal.toml`; mixed sources name both bases. `corpus-check` validates the declared basis. Never copy upstream renderer code or artwork; ORC-licensed PF2 material stays out of scope.

## Working pattern

Start new work from current `origin/main` on an issue-linked branch. Keep outstanding work in issues. Docs retain decisions, limitations, and evidence.

In a worktree that T3 Code created for your thread, the branch already exists (`git worktree list` shows the worktree, and its path does not have to match any fixed location): commit and push on it, and do not create, rename or switch branches. T3 removes a worktree only while it stays on the branch T3 recorded. Link the issue in the PR body instead of the branch name. To start from current `origin/main`, run `git fetch origin && git reset --hard origin/main` before you change any file.

Work in small commits, each with a verifiable state. A task is finished only after the checks for its kind of change pass (see "Test scope"). Report failures with their output.

Open PRs against `main` and use `Closes #N` only for completed acceptance, otherwise `Refs #N`. A merged PR is not a release. Record exact CI and Pages runs separately from local and live checks.

After changing compiler code, schema types, or the corpus pin, run `cargo xtask content` before `cargo xtask build-site`. The site build reuses an existing content manifest; it does not detect stale generated content. For corpus or schema changes, follow the upgrade checks in the `verify` skill.

## Test scope

Owner rules of 2026-11-17 (decision `docs/decisions/2026-11-17-test-scope-owner-rules-of-2026-11-17.md`). Run each check once per pull request, and only the checks that the change can break. Find every row that matches a changed file. Run the checks of all those rows.

- **Locally:** `cargo xtask check` always; `cargo xtask content` as well when the compiler, the schema or the corpus pin changed; and at most ONE e2e spec: the new spec the change adds or, when it adds none, the one spec that the feature map of the `verify` skill selects for the changed feature. No other specs, and no full browser suite. Broader coverage comes from the batch CI run on `main`. A change to docs or xtask needs no browser run.
- **Merge:** a pull request that a reviewer has read merges on the required `Rust checks` status. A reviewer does not run tests or builds.
- **After each batch of merges:** start one full CI run on `main` (`gh workflow run ci.yml --ref main`). It tests the merged result; fix what it finds in a new pull request.
- **Full CI before merge:** only a change that matches a row below that says so: CI configuration, the toolchain, install, storage or offline code, or a format version (`SCHEMA_VERSION`, `INDEX_FORMAT_VERSION`).
- **Promotion** (`docs/releasing.md`) needs a green full run on the exact `main` SHA.

| What changed | Local checks | Full CI run on the branch before merge |
|---|---|---|
| Docs only (`docs/`, `AGENTS.md`, `DESIGN.md`, `.agents/skills/`), or research scripts the build does not run (`scripts/*-nested.py`) | `cargo xtask check` always (it validates `docs/decisions/`, `docs/status/` and the Markdown links). Run a changed script. No build, no e2e | No |
| CI and build configuration: `.github/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `Trunk.toml`, `tests/e2e/package*.json`, `playwright.config.ts`, `server.mjs` | `actionlint` for workflow files. `cargo xtask check` for Cargo or toolchain files | Yes |
| `scripts/promote-release.py`, `scripts/test-promote-release.py` | `python3 scripts/test-promote-release.py` | No |
| `scripts/check-publication.py`, `scripts/publication-license.py`, `scripts/test-check-publication.py` | `python3 scripts/test-check-publication.py`, and the screen on a built `dist/` when `check-publication.py` changed | No |
| `scripts/merge-pr.py`, `scripts/test-merge-pr.py` | `python3 scripts/test-merge-pr.py`, and a preview (no `--merge`) on a real PR | No |
| Rust outside `crates/web`: compiler, corpus, schema, search, xtask | `cargo xtask check`. Also `cargo xtask content` when the compiler, the schema, or the corpus pin changed | Only when the change bumps `SCHEMA_VERSION` or `INDEX_FORMAT_VERSION` |
| UI: `crates/web`, `web/`, `tests/e2e` | `cargo xtask check`, then at most one spec (see Locally): `cargo xtask build-site --public-url /cast-iron/` and `--project=chromium` (add `--project=mobile-chromium` when `playwright.config.ts` lists the spec there) | No |
| Service worker, installer, storage, offline (`web/sw.js`, `install.rs`, `content_cache.rs`, `store.rs`) | The UI checks, plus `--project=webkit` on that one spec | Yes |
| Corpus pin (`sources/corpus.toml`) | `cargo xtask check`, `cargo xtask content` and `cargo xtask search-golden`. At most one spec, the one the feature map selects for the content that changed | No |

Start the one full CI run for a row that needs it (`gh workflow run ci.yml --ref <branch>`) after the review fixes are in, not after each push. Run it again only in two cases. First: a later commit changes a file whose row needs a full run. Second: a pull request merged into `main` after the run, and it changed files that this pull request also changes, or specs that test them. Run WebKit locally only on the affected spec; CI runs the full WebKit suite, which is informational.

- When a test fails only in a run with other load on the host, run its spec again alone before you change code. Report both results.
- A reviewer reads the diff and the code, and cites the implementer's report and the CI run (`gh pr checks`, `gh run view`) for test status.
- Report the exact specs and projects you ran. Do not report a targeted run as a full run.
- `scripts/merge-pr.py` still classifies files by the previous table (it asks for a full run on any UI, Rust or corpus pin change); until its rows follow this table, merge a reviewed pull request that needs only `Rust checks` with `gh pr merge --merge --match-head-commit <sha>`.
