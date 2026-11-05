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

/** Resolves when the transaction commits and rejects when it aborts or fails. */
export function idbTxDone(transaction) {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () => reject(transaction.error || new Error("transaction aborted"));
  });
}

/** Lowercase hex SHA-256 of an ArrayBuffer. */
export async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

export function randomUUID() {
  return crypto.randomUUID();
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
  const announce = () => {
    // On the first install there is no controller yet, and nothing is "waiting" for the user.
    if (registration.waiting && navigator.serviceWorker.controller) onWaiting();
  };
  announce();
  registration.addEventListener("updatefound", () => {
    const worker = registration.installing;
    if (worker) worker.addEventListener("statechange", () => worker.state === "installed" && announce());
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

/** Resolves the shell version that the controlling worker reports, or null. */
export function shellVersion() {
  const worker = navigator.serviceWorker?.controller;
  if (!worker) return Promise.resolve(null);
  return new Promise((resolve) => {
    const channel = new MessageChannel();
    const timer = setTimeout(() => resolve(null), 1500);
    channel.port1.onmessage = (event) => {
      clearTimeout(timer);
      resolve(event.data?.version ?? null);
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
