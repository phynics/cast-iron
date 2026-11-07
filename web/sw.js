// Cast Iron service worker. It caches the app shell only. Library content lives in the
// Cache API under cast-iron-content-<version> and is read by the app itself, never here.
//
// `xtask stamp-shell` fills in the constants below in dist/sw.js. Do not repeat their
// placeholder text elsewhere in this file: the stamper replaces every occurrence.
"use strict";

const SHELL_VERSION = "__SHELL_VERSION__";
const SHELL_FILES = "__SHELL_FILES__";
const SHELL_DATE = "__SHELL_DATE__";

// An unstamped copy (trunk serve, or a plain file server) has a string instead of a list.
// It then caches nothing and leaves every request to the network.
const DEV = !Array.isArray(SHELL_FILES);
const VERSION = DEV ? "dev" : SHELL_VERSION;
// The commit date, or null when the build had no git checkout (a tarball build).
const DATE = DEV || !SHELL_DATE ? null : SHELL_DATE;
const SHELL_PREFIX = "cast-iron-shell-";
const SHELL_CACHE = SHELL_PREFIX + VERSION;

// All URLs are relative to the scope, so the app works at / and at /cast-iron/.
const SCOPE = self.registration.scope;
const scoped = (path) => new URL(path, SCOPE).href;
// Dotfiles such as .trunk-lock are build leftovers, not part of the app.
const SHELL_URLS = new Set(DEV ? [] : SHELL_FILES.filter((f) => !/(^|\/)\./.test(f)).map(scoped));
const INDEX_URL = scoped("index.html");

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      if (!DEV) {
        const cache = await caches.open(SHELL_CACHE);
        try {
          // Every shell file must arrive, straight from the network. One failure fails the install,
          // and the previous worker keeps serving.
          await Promise.all(
            [...SHELL_URLS].map(async (url) => {
              const response = await fetch(new Request(url, { cache: "reload" }));
              if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
              await cache.put(url, response);
            }),
          );
        } catch (error) {
          await caches.delete(SHELL_CACHE);
          throw error;
        }
      }
      // The first install takes over at once. An update waits for the app to ask, so that code
      // never changes under a user who is reading.
      if (!self.registration.active) await self.skipWaiting();
    })(),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      // Only shell caches. Content caches (cast-iron-content-*) hold the user's library.
      const names = await caches.keys();
      await Promise.all(
        names
          .filter((name) => name.startsWith(SHELL_PREFIX) && name !== SHELL_CACHE)
          .map((name) => caches.delete(name)),
      );
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("message", (event) => {
  const message = event.data;
  if (!message || typeof message !== "object") return;
  if (message.type === "SKIP_WAITING") {
    self.skipWaiting();
  } else if (message.type === "GET_VERSION" && event.ports[0]) {
    event.ports[0].postMessage({ version: VERSION, date: DATE });
  }
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (DEV || request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin || !request.url.startsWith(SCOPE)) return;
  // Library content is never cached here: manifest.json is network only, and versioned
  // files are read from the Cache API by the app.
  const path = request.url.slice(SCOPE.length).split(/[?#]/)[0];
  if (path.startsWith("content/")) return;

  if (request.mode === "navigate") {
    event.respondWith(serveIndex(request));
  } else if (SHELL_URLS.has(url.origin + url.pathname)) {
    event.respondWith(cacheFirst(request, url.origin + url.pathname));
  }
});

async function serveIndex(request) {
  const cached = await caches.match(INDEX_URL, { cacheName: SHELL_CACHE });
  return cached || fetch(request);
}

async function cacheFirst(request, key) {
  const cached = await caches.match(key, { cacheName: SHELL_CACHE });
  return cached || fetch(request);
}
