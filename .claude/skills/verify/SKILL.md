---
name: verify
description: Use after changing the UI, the installer, the service worker, search, or the content, to prove the change works in the real app built into dist/.
---

# Verify a change in the real app

Tests alone do not prove a user-visible change. Build the site, run the acceptance suite, and look at the app.

## 1. Build

From the repository root:

```sh
cargo xtask build-site --public-url /cast-iron/
```

This compiles the content when `generated/content/manifest.json` is missing, builds the WebAssembly app with Trunk, and writes `dist/`. Run `cargo xtask check` as well before you finish.

## 2. Run the acceptance suite

```sh
npm --prefix tests/e2e ci
npx --prefix tests/e2e playwright install chromium
npm --prefix tests/e2e test
```

To run one test, pass a name pattern:

```sh
npm --prefix tests/e2e test -- --grep "<name>"
```

Report any failure with its output. Do not call a change done while a test fails.

## 3. Drive the app by hand (new features)

A new feature needs a check that a user can follow. Serve `dist/` and open it:

```sh
node tests/e2e/server.mjs
```

Then open `http://localhost:4318/cast-iron/`. Set `E2E_PORT` to use another port.

Use a real browser. Click through the feature the way a user does, and read the page after each step.

## 4. What to check

- The browser console shows no errors.
- After the app is installed, it works offline. Set the browser to offline and reload.
- Take screenshots in both themes (light and dark) at 390 px and 1280 px wide. Open each screenshot and look at it. Check layout, text overflow, and contrast.

## 5. Feature map

Find the row for the change. Run the e2e tests in that area with `--grep`, and read the listed source files.

| Feature | `--grep` pattern (spec file) | Source files |
|---|---|---|
| Search and filters | `Search operates` (`search.spec.ts`) | `crates/search`, `crates/web/src/views/search.rs` |
| Installer, integrity, recovery | `installs completely\|integrity\|Interrupted` (`install.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/content_cache.rs`, `web/sw.js` |
| Offline launch, no runtime server | `Offline launch\|runtime server` | `web/sw.js`, `crates/web/src/app.rs` |
| Entry rendering | `complex formatting` (`reference.spec.ts`) | `crates/web/src/views/rich.rs`, `crates/web/src/views/entry.rs` |
| Cross-references and history | `Cross-reference\|Back/forward` (`reference.spec.ts`) | `crates/web/src/route.rs`, `crates/web/src/views/rich.rs` |
| Browsing | `browsing` | `crates/web/src/views/browse.rs` |
| Bookmarks and collections | `Bookmarks persist` | `crates/web/src/store.rs`, `crates/web/src/views/bookmarks.rs` |
| Content updates | `Content update` (`update.spec.ts`) | `crates/web/src/install.rs`, `crates/web/src/views/settings.rs` |
| PWA install metadata | `installable` | `web/manifest.webmanifest`, `web/index.html` |
| Mobile layout | `Mobile viewport` (`mobile.spec.ts`) | `web/styles.css` |
| Legal and attribution | `Licensing` | `crates/web/src/views/legal.rs`, `sources/psrd-data/legal-notices.json` |
| Content compiler | none: `cargo test -p cast-iron-content-compiler`, CI determinism job | `crates/content-compiler` |

## Keep this file current

When you add a feature, add its row to the feature map, with its e2e test area and source files. When you rename or move a file listed here, update the row. A feature that is missing from this table cannot be verified by the next agent.
