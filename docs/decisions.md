# Decisions

Decisions that refine `docs/spec.md` and that a later reader cannot derive from the code.

## Platform layer

- **CSP and Trunk's inline script.** Trunk writes its start-up module inline, so a strict `script-src` needs its hash. The `post_build` hook `web/stamp-csp.pl` (registered in `Trunk.toml`) hashes the built inline `<script>` and `<style>` and fills the `__CSP_SCRIPT_HASH__` and `__CSP_STYLE_HASH__` placeholders in the policy. Without the hook the page does not start, so a missing hook shows at once. `trunk serve` injects a live-reload script that this policy blocks; develop against `cargo xtask build-site` output.
- **SRI is off (`no_sri = true`).** Chrome warns on the `integrity` attribute that Trunk puts on the wasm preload, and same-origin files already come from the service worker cache.
- **No JavaScript beyond `sw.js` and the bridge, so no pre-render theme script.** `<html>` gets `data-theme` as the first action of `start()`, from a `localStorage` mirror of the IndexedDB choice. The window before the wasm module loads follows `prefers-color-scheme`. A user whose explicit choice differs from the system can see one background change per cold start.
- **Service worker update policy.** The first install skips waiting and claims the page. Later versions wait until the app posts `{type: "SKIP_WAITING"}`; the app shows "Update ready" and reloads on `controllerchange`. The worker never touches `cast-iron-content-*`.
- **Content caches.** One cache per version, `cast-iron-content-<version>`, keyed by the document-relative `content/<path>` URL. The manifest is stored with the version as `<version>/manifest.json`. `active_library` in IndexedDB is the only signal of "ready"; the boot check confirms that the cache holds every manifest file and sends the user back to the installer if not.
- **Install order.** Scan the cache for verified files (resume), check quota against the bytes still missing plus 5%, download with three workers and three tries per file, verify size and SHA-256 before `cache.put`, store the manifest, re-hash every file, write `active_library` (and clear `pending_install`) in one IndexedDB transaction, and only then delete the other content caches.
- **`Library` is `SendWrapper<Rc<_>>`.** Leptos 0.8 context requires `Send + Sync`; the browser runs one thread, so the wrapper never panics.
- **Hash links and `<base>`.** `<base href>` makes `#/x` resolve against the base. `normalize_location()` removes `index.html` and query strings at boot so that plain `#/...` anchors stay same-document navigations. The skip link is handled in code for the same reason.
