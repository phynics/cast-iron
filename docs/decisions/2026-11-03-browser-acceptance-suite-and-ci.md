---
title: "Browser acceptance suite and CI"
date: 2026-11-03
status: current
superseded_by: []
issues: []
prs: []
area: process
---

# Browser acceptance suite and CI

- **The suite tests the real `dist/`.** `tests/e2e/server.mjs` serves an existing `dist/` at `/cast-iron/`, and `playwright.config.ts` refuses to start without one. It also exposes `/__test/*` hooks (corrupt, stall, abort, slow, or refuse a path; overlay an alternative library; read the request log). The hooks exist on this test server only. Each Playwright worker starts its own server on a free port (the `serverOrigin` worker fixture in `tests/e2e/tests/fixtures.ts`), so hooks never leak between workers and parallel runs on one machine never collide. Local runs use 4 workers (`E2E_WORKERS` changes it); CI keeps 1 worker per shard runner. Run by hand, `npm --prefix tests/e2e run serve` still listens on 4318, and `E2E_PORT` changes it.
- **Most tests start from a pre-installed library.** A `preinstalledProfile` worker fixture installs the library once into a persistent browser profile, waits for service-worker control and `active_library`, then closes the profile. The `installedTest` object copies that profile into a fresh temp directory for each test and opens it with `launchPersistentContext` on the worker's own origin. A copy, not a shared profile, keeps tests independent: service-worker caches, IndexedDB, and local storage all belong to one test. macOS clone copies (`cp -c`) and Linux reflinks keep the copy cheap; other systems fall back to a recursive copy. `installLibrary` first waits for either the reader or the Download button, so it downloads only on a clean profile; under `installedTest` it fails instead, because a copy that lost the browser's site data would otherwise reinstall silently and only get slower. Specs that verify installation, the Download offer, request logs, or cache contents, that seed IndexedDB before installing, or that install an overlay or fixture library keep the default `test` and its clean profile.
- **Offline means two things.** Tests call `context.setOffline(true)` and also make the server drop every connection. Chromium's offline emulation does not cover the browser's own check of `sw.js`, which reached the server in the first run, so the allowed exception is exactly `sw.js`.
- **Update test library.** The test builds version 2 in a temp directory (a copy of the library with `legal.json` changed, hashes and manifest recomputed, version by the compiler's rule) and serves it through the overlay. The client does not derive versions, so any `v...` string would do.
- **Browsers.** Chromium (and a 390 px Pixel 7 project for `mobile.spec.ts`) gate CI. WebKit passes all tests locally on macOS; its project turns Playwright traces and screenshots off, because they inject an inline `<style>` into WebKit pages of persistent contexts and the page CSP refuses it, which would be a false console error. WebKit runs in CI with `continue-on-error` until a CI run shows it stable. Firefox is disabled in CI as recorded below.
- **Compiler determinism is a CI check, not a browser test.** The `content` job compiles once; the `determinism` job compiles again and compares all of `generated/` (since 2026-11-09, see "Test scope" below).
- **Pages.** `pages.yml` builds as CI does and deploys with the official Pages actions. The repository must have Pages set to the "GitHub Actions" source, which is a repository setting that no workflow changes.
