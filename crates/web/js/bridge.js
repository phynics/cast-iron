// The only JavaScript that the app owns besides sw.js. It wraps browser callbacks
// as promises and holds service worker glue. Rust owns every decision.

/** Opens an IndexedDB database. `onUpgrade(db, oldVersion, transaction)` runs in the upgrade transaction. */
export function idbOpen(name, version, onUpgrade) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(name, version);
    request.onupgradeneeded = (event) => {
      try {
        onUpgrade(request.result, event.oldVersion, request.transaction);
      } catch (error) {
        request.transaction.abort();
        reject(error);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/** Resolves with the result of an IDBRequest. */
export function idbRequest(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/**
 * Resolves when the transaction commits and rejects when it aborts or fails. Rust creates this
 * promise before the first request, so a request that fails first can return before anyone
 * awaits it. The no-op `catch` marks the outcome as observed: such a return reports the failure
 * from the request, and the promise must not also leave an unhandled rejection (a page error).
 * A caller that awaits the promise still gets the rejection.
 */
export function idbTxDone(transaction) {
  const done = new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    // A request's error event reaches the transaction before the abort, while `transaction.error`
    // is still null (after `abort()` it stays null): take the reason from the request.
    transaction.onerror = (event) =>
      reject(transaction.error || event.target?.error || new Error("transaction failed"));
    transaction.onabort = () => reject(transaction.error || new Error("transaction aborted"));
  });
  done.catch(() => {});
  return done;
}

/** Lowercase hex SHA-256 of an ArrayBuffer. */
export async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

/** True when the browser can gunzip a stream (Safari 16.4, Chrome 80, Firefox 113). */
export function gunzipSupported() {
  return typeof DecompressionStream === "function";
}

/**
 * Fetches `url` past the HTTP cache and streams the body, so that Rust can see whether bytes are
 * still arriving. `onProgress()` runs when the response starts and after every chunk. Rust owns
 * `controller` and aborts it when no bytes arrive (the stall timeout of the installer).
 * Resolves the body as a Uint8Array, or the HTTP status as a number when the answer is not ok.
 */
export async function fetchBytes(url, controller, onProgress) {
  const signal = controller.signal;
  // Rust drops `onProgress` once it aborts: never call it after that.
  const progress = () => {
    if (!signal.aborted) onProgress();
  };
  const response = await fetch(url, { cache: "no-store", signal });
  if (!response.ok) {
    response.body?.cancel().catch(() => {});
    return response.status;
  }
  progress();
  if (!response.body) return new Uint8Array(await response.arrayBuffer());
  const reader = response.body.getReader();
  const chunks = [];
  let length = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      length += value.length;
      progress();
    }
  } finally {
    // Both the end of the body and an abort release the lock.
    reader.releaseLock();
  }
  const bytes = new Uint8Array(length);
  let at = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, at);
    at += chunk.length;
  }
  return bytes;
}

/**
 * Reads a stored gzip file from the cache and decodes it. Resolves the decoded bytes as an
 * ArrayBuffer, or null when the cache has no such file.
 */
export async function cacheReadGunzip(cache, url) {
  const response = await cache.match(url);
  if (!response) return null;
  return new Response(response.body.pipeThrough(new DecompressionStream("gzip"))).arrayBuffer();
}

/** Asks for persistent storage. Resolves false when unsupported or refused. */
export async function storagePersist() {
  return navigator.storage?.persist ? navigator.storage.persist() : false;
}

export async function storagePersisted() {
  return navigator.storage?.persisted ? navigator.storage.persisted() : false;
}

/** Resolves `{usage, quota}` in bytes, or null when the browser does not say. */
export async function storageEstimate() {
  if (!navigator.storage?.estimate) return null;
  const { usage, quota } = await navigator.storage.estimate();
  return { usage: usage ?? null, quota: quota ?? null };
}

let registration = null;

/**
 * Registers the service worker. `onWaiting()` fires when a new shell is installed and waiting.
 * Resolves true when a worker is registered.
 */
export async function registerServiceWorker(url, scope, onWaiting) {
  if (!("serviceWorker" in navigator)) return false;
  registration = await navigator.serviceWorker.register(url, { scope });
  // The first worker of a profile takes over by itself (`skipWaiting`, then `clients.claim()`), so
  // it is never an update. WebKit can deliver its `installed` event after the page already has a
  // controller, while `registration.waiting` still names it: the first install then looks like an
  // update and the header shows "update ready" (#683). Remember that worker and skip it.
  const firstWorker =
    !registration.active && !navigator.serviceWorker.controller
      ? registration.installing || registration.waiting
      : null;
  const announce = () => {
    const waiting = registration.waiting;
    // A waiting worker is an update only when it is installed, is not the first one, and a
    // controller exists for it to replace.
    if (waiting && waiting !== firstWorker && waiting.state === "installed" && navigator.serviceWorker.controller) {
      onWaiting();
    }
  };
  announce();
  registration.addEventListener("updatefound", () => {
    const worker = registration.installing;
    if (worker && worker !== firstWorker) worker.addEventListener("statechange", () => worker.state === "installed" && announce());
  });
  return true;
}

/** Asks the browser to look for a newer sw.js now. Rejects when offline. */
export async function checkShellUpdate() {
  if (registration) await registration.update();
}

/** Tells the waiting worker to take over and reloads once it controls the page. */
export function applyShellUpdate() {
  if (!registration?.waiting) return;
  navigator.serviceWorker.addEventListener("controllerchange", () => location.reload(), { once: true });
  registration.waiting.postMessage({ type: "SKIP_WAITING" });
}

/** Resolves `{ version, date }` that the controlling worker reports, or null. */
export function shellInfo() {
  const worker = navigator.serviceWorker?.controller;
  if (!worker) return Promise.resolve(null);
  return new Promise((resolve) => {
    const channel = new MessageChannel();
    const timer = setTimeout(() => resolve(null), 1500);
    channel.port1.onmessage = (event) => {
      clearTimeout(timer);
      resolve(event.data ?? null);
    };
    worker.postMessage({ type: "GET_VERSION" }, [channel.port2]);
  });
}

// ---- Install prompt ------------------------------------------------------------------------
// Chromium fires `beforeinstallprompt` once, possibly before the app has started. This module
// loads first, so it holds the event and tells the app through a window event.

/** Window event for "the install prompt appeared or went away". `js.rs` listens for the same name. */
const INSTALL_CHANGE_EVENT = "cast-iron:install-change";

let installPrompt = null;

function installChanged() {
  window.dispatchEvent(new Event(INSTALL_CHANGE_EVENT));
}

window.addEventListener("beforeinstallprompt", (event) => {
  // Stop the browser's own mini-infobar; the app decides where to offer installing.
  event.preventDefault();
  installPrompt = event;
  installChanged();
});

window.addEventListener("appinstalled", () => {
  installPrompt = null;
  installChanged();
});

/** True while a captured install prompt can still be opened. */
export function installAvailable() {
  return installPrompt !== null;
}

/**
 * Opens the browser's install prompt. The event works once, so it is cleared at once.
 * Resolves the user's choice, "accepted" or "dismissed", or null when no prompt is held.
 */
export async function promptInstall() {
  const event = installPrompt;
  if (!event) return null;
  installPrompt = null;
  installChanged();
  await event.prompt();
  const choice = await event.userChoice;
  return choice?.outcome ?? null;
}
