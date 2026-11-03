# Cast Iron: agent guide

Cast Iron is an offline-first PWA reference for the Pathfinder First Edition rules. It is Rust/WASM (Leptos, client-side rendering) on GitHub Pages, with no backend.

- Product source of truth: `docs/spec.md`. Do not rewrite it.
- Decisions that refine the spec: `docs/decisions.md`. Add an entry for each decision that a later reader cannot derive from code.
- Current state and acceptance results: `docs/status.md`. Update it when an acceptance item changes.

## Commands

Run all commands from the repository root. `rust-toolchain.toml` pins the toolchain. `~/.cargo/bin` can be missing from `PATH`; `cargo xtask` handles that case.

| Task | Command |
|---|---|
| Install Trunk (once) | `cargo install trunk --locked` |
| Get the pinned dataset | `cargo xtask fetch-sources` |
| Compile content | `cargo xtask content` |
| Build the site into `dist/` | `cargo xtask build-site` (`--public-url /` for a root path) |
| Check before you finish | `cargo xtask check` |
| Browser acceptance tests | `npm --prefix tests/e2e ci`, then `npm --prefix tests/e2e test` |

To prove a change in the real app, use the `verify` skill in `.claude/skills/verify/SKILL.md`.

## Layout

| Path | Contents |
|---|---|
| `crates/schema` | The contract between the compiler and the client: content types, validation |
| `crates/content-compiler` | Native CLI: PSRD-Data adapter, HTML to rich content, reports |
| `crates/search` | Index generation and query engine, for native and WASM. No dependency on `schema` |
| `crates/web` | Leptos client |
| `crates/xtask` | Build automation |
| `web/` | `index.html`, `sw.js`, manifest, CSS, icons |
| `sources/psrd-data/` | Dataset pin, POC selection, id overrides, legal notices |
| `LICENSES/` | Code, content, and upstream notices |
| `generated/`, `dist/`, `.cache/` | Build output. Never commit these directories |
| `tests/e2e` | Playwright acceptance tests |

## Rules

1. Offline is the product. Nothing at runtime may contact a third-party host: no CDN, no remote font, no telemetry. After installation, ordinary use makes no network request at all.
2. Client rendering uses typed nodes only. Never add `inner_html` or raw upstream HTML. This is the XSS defense.
3. `EntryId` values are public, permanent identifiers, because bookmarks store them. If an upstream change alters an id, add a mapping in `sources/psrd-data/id-overrides.toml`.
4. A breaking change to serialized content must increase `SCHEMA_VERSION`. A breaking change to the search format must increase `INDEX_FORMAT_VERSION`.
5. Compiler output must stay byte-identical between runs: no timestamps, no hash-map iteration order.
6. Excluded or rejected material must appear in the coverage report. Never drop it silently.
7. A partly installed library must never show as ready. Verify hashes before activation. The active library stays usable until a replacement is verified.
8. JavaScript stays limited to `web/sw.js` and the small bridge in `crates/web/js/`. Put logic that can be tested natively in Rust.
9. All asset URLs are relative to the document base, because the app runs under a project path (`/cast-iron/`).
10. Do not ship artwork or other Product Identity. Read `docs/research/licensing-audit.md` before adding a source. Keep the legal page data complete.
11. The app must not use the Pathfinder logo, and must not claim official status or compatibility.

## Working pattern

Work in small commits, each with a verifiable state. A task is finished only after `cargo xtask check` passes and, for user-visible behavior, the affected e2e test passes on a real `dist/` build. Report failures with their output.
