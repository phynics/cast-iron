import { expect, type Page } from "@playwright/test";

/** A count as the app writes it: digits grouped by commas. */
export const count = (n: number) => n.toLocaleString("en-US");

/** The number in text that the app wrote with grouped digits. */
export const num = (text: string | null | undefined) => Number((text ?? "").replace(/,/g, ""));

/** The browse A to Z group of a title: an ASCII letter, else `#` for digits and symbols. */
export function groupLetter(title: string): string {
  const first = [...title].find((char) => /[\p{L}\p{N}]/u.test(char))?.toUpperCase();
  return first && /^[A-Z]$/.test(first) ? first : "#";
}

/** Mirrors `letter_id` in `crates/web/src/views/browse.rs`. */
export function letterId(letter: string): string {
  return letter === "#" ? "letter-other" : `letter-${letter}`;
}

/**
 * Open the app and wait for the reader, downloading the library only when this profile has no
 * active library. Tests that must observe the installation offer use a clean profile and click
 * Download themselves; this helper is safe from an already-installed profile.
 */
let preinstalled = false;
/**
 * Set by the `installedTest` context fixture while its test runs. A copied profile that shows the
 * Download offer has lost its library; the test would silently reinstall and only get slower, so
 * that case fails instead.
 */
export function expectPreinstalled(on: boolean) {
  preinstalled = on;
}

/**
 * `freshBooks: true` keeps the profile new, so the first-run default (Paizo Rulebooks) applies.
 * Without it the helper first records that the profile's book choice is settled, the state of a
 * profile from before the default existed, so tests keep the library with every book active.
 * That is the only way a test changes the default: the app has no switch for it.
 */
export async function installLibrary(page: Page, options: { freshBooks?: boolean } = {}) {
  await page.goto("./");
  const reader = page.getByRole("navigation", { name: "Primary" });
  const download = page.getByRole("button", { name: "Download", exact: true });
  await reader.or(download).waitFor({ state: "visible", timeout: 90_000 });
  if (await download.isVisible()) {
    if (preinstalled) {
      throw new Error("The pre-installed profile copy has no library: the profile copy did not carry the browser's site data.");
    }
    if (!options.freshBooks) await settleBookDefaults(page);
    await download.click();
  }
  await reader.waitFor({ state: "visible", timeout: 90_000 });
  // Home reads the catalog from the library cache after the reader appears. Let that finish:
  // a reload or navigation that interrupts an in-flight Cache read is a page error in WebKit
  // ("Context is stopped"), and a pre-installed profile reaches this point almost at once.
  await page.locator("section[aria-labelledby=by-kind] a.cat").first().waitFor({ state: "visible", timeout: 90_000 });
  // Home also warms the search index in the background. Leaving or reloading while that read is
  // in flight is the same WebKit page error on the next screen, and two specs (journeys and
  // relationships) navigated away quickly enough to hit it.
  await waitHomeIdle(page);
}

/** Writes the `book_defaults` marker as `done`, before the first library is activated. */
async function settleBookDefaults(page: Page) {
  await page.evaluate(() => new Promise<void>((resolve, reject) => {
    const request = indexedDB.open("cast-iron");
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const tx = db.transaction("meta", "readwrite");
      tx.objectStore("meta").put("done", "book_defaults");
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }));
}

export async function waitReady(page: Page) {
  await expect(page.getByRole("navigation", { name: "Primary" })).toBeVisible({ timeout: 90_000 });
}

/**
 * The service worker controls this page (needed before any offline claim).
 *
 * `clients.claim()` can run before this page is ready to observe it, which leaves
 * `controller` null until a navigation. Wait for an active registration, give the
 * claim a short window, and otherwise reload once so the active worker controls
 * the new document.
 */
export async function waitControlled(page: Page) {
  await page.evaluate(() => navigator.serviceWorker.ready);
  const claimed = await page
    .waitForFunction(() => !!navigator.serviceWorker.controller, undefined, { timeout: 10_000 })
    .then(() => true)
    .catch(() => false);
  if (!claimed) {
    await page.reload();
    await page.waitForFunction(() => !!navigator.serviceWorker.controller);
  }
}

export async function activeLibrary(page: Page) {
  return page.evaluate(
    () =>
      new Promise<any>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          if (!db.objectStoreNames.contains("meta")) {
            db.close();
            return resolve(null);
          }
          const get = db.transaction("meta").objectStore("meta").get("active_library");
          get.onsuccess = () => {
            db.close();
            resolve(get.result ?? null);
          };
          get.onerror = () => reject(get.error);
        };
      }),
  );
}

/** One `meta` record of the app's IndexedDB, or null. */
export function storedMeta(page: Page, key: string): Promise<any> {
  return page.evaluate(
    (key) =>
      new Promise((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          if (!db.objectStoreNames.contains("meta")) {
            db.close();
            return resolve(null);
          }
          const get = db.transaction("meta").objectStore("meta").get(key);
          get.onsuccess = () => {
            db.close();
            resolve(get.result ?? null);
          };
          get.onerror = () => {
            db.close();
            reject(get.error);
          };
        };
      }),
    key,
  );
}

/** Writes one `meta` record of the app's IndexedDB. */
export function writeMeta(page: Page, key: string, value: unknown): Promise<void> {
  return page.evaluate(
    ({ key, value }) =>
      new Promise<void>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const tx = db.transaction("meta", "readwrite");
          tx.objectStore("meta").put(value, key);
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
          tx.onerror = () => reject(tx.error);
        };
      }),
    { key, value },
  );
}

/** Removes one `meta` record, so the next app start reads its default. */
export function deleteMeta(page: Page, key: string): Promise<void> {
  return page.evaluate(
    (key) =>
      new Promise<void>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const tx = db.transaction("meta", "readwrite");
          tx.objectStore("meta").delete(key);
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
          tx.onerror = () => reject(tx.error);
        };
      }),
    key,
  );
}

/** Cache name -> request paths relative to the app base (document-relative `content/...` keys). */
export async function cacheContents(page: Page): Promise<Record<string, string[]>> {  return page.evaluate(async () => {
    const out: Record<string, string[]> = {};
    const base = new URL("./", document.baseURI).pathname;
    for (const name of await caches.keys()) {
      const cache = await caches.open(name);
      out[name] = (await cache.keys()).map((r) => new URL(r.url).pathname.slice(base.length));
    }
    return out;
  });
}

export const contentCaches = (all: Record<string, string[]>) =>
  Object.keys(all).filter((n) => n.startsWith("cast-iron-content-"));

/** Run a search through the box and wait until the results for exactly this query are shown. */
export async function searchFor(page: Page, query: string) {
  await page.goto("#/search");
  const input = page.locator("#search-input");
  await input.fill(query);
  await waitResults(page, query);
}

/** The header's search icon: the one control that opens Quick search. */
export const searchIcon = (page: Page) => page.locator("header.app-header").getByRole("button", { name: "Search", exact: true });

/** The Quick search layer. */
export const quickSearch = (page: Page) => page.getByRole("dialog", { name: "Quick search" });

/** The field of the Quick search layer. */
export const quickField = (page: Page) => quickSearch(page).locator("#quick-search-input");

/** The rows of the Quick search list: suggestions, or recent and starter searches. */
export const quickOptions = (page: Page) => quickSearch(page).getByRole("option");

/** The names of the entries that Quick search suggests, without the "See all results" row. */
export async function suggestionTitles(page: Page) {
  return quickSearch(page).locator('[role="option"]:not(.quick-search-all) .quick-search-name').allInnerTexts();
}

/**
 * Open Quick search from the header icon and return the layer. A mouse click makes Playwright
 * scroll the sticky header's natural position into view, which moves a scrolled page. Pass
 * `keyboard` to open with the Enter key and keep the scroll position under test.
 */
export async function openQuickSearch(page: Page, { keyboard = false } = {}) {
  if (keyboard) {
    // `locator.focus()` scrolls a sticky element's natural position into view as well.
    await searchIcon(page).evaluate((el) => el.focus({ preventScroll: true }));
    await page.keyboard.press("Enter");
  } else {
    await searchIcon(page).click();
  }
  const layer = quickSearch(page);
  await expect(layer).toBeVisible();
  await expect(quickField(page)).toBeFocused();
  return layer;
}

/** Search from any page through Quick search and wait for the results of exactly this query. */
export async function quickSearchFor(page: Page, query: string) {
  // On the results page the icon focuses that page's own field, so leave it first.
  if (page.url().includes("#/search")) await page.goto("#/");
  const layer = await openQuickSearch(page);
  await quickField(page).fill(query);
  await page.keyboard.press("Enter");
  await waitResults(page, query);
}

export async function waitResults(page: Page, query: string) {
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", query);
  await expect(page.locator(".results-region")).toHaveAttribute("data-search-ms", /.+/);
  await expect(page.locator(".results .hit, .empty").first()).toBeVisible();
}

export const hitTitles = (page: Page) => page.locator(".results > .hit > .hit-link > .hit-title, .results > .hit > details > summary .hit-title").allInnerTexts();

/** Bytes of a number like "11.2 MB" are not needed; the installer shows the size as text. */
export const pct = (values: number[], p: number) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(p * sorted.length) - 1)];
};

/** Table blocks of an entry body in document order (lists and quotes nest blocks). */
export function tablesOf(blocks: any[]): any[] {
  const found: any[] = [];
  const walk = (list: any[]) => {
    for (const block of list) {
      if (block.t === "table") found.push(block);
      else if (block.t === "list") block.items.forEach(walk);
      else if (block.t === "quote") walk(block.content);
    }
  };
  walk(blocks);
  return found;
}
export const tableRows = (t: any): any[] => [...(t.head ?? []), ...t.body, ...(t.foot ?? [])];
export const spans = (c: any) => (c.colspan ?? 1) > 1 || (c.rowspan ?? 1) > 1;

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { DIST, GENERATED, distManifest, loadCorpus, type Entry } from "./fixtures";

/**
 * Builds an alternative library in a temp directory: the served library with legal.json changed.
 * The result is laid out as `<dir>/content/manifest.json` and `<dir>/content/<version>/...`,
 * which the test server overlays on dist. The version follows the compiler's documented rule
 * (sha256 of the sorted "path:sha256" lines), but the client must not depend on that.
 */
export function buildUpdatedLibrary(marker: string) {
  const old = distManifest();
  const oldPrefix = `${old.content_version}/`;
  const files = new Map<string, Buffer>();
  for (const f of old.files) files.set(f.path.slice(oldPrefix.length), fs.readFileSync(path.join(DIST, "content", f.path)));
  const legal = JSON.parse(files.get("legal.json")!.toString());
  legal.community_use_notice += ` ${marker}`;
  files.set("legal.json", Buffer.from(JSON.stringify(legal)));

  const sha = (b: Buffer) => crypto.createHash("sha256").update(b).digest("hex");
  const sorted = [...files.keys()].sort();
  const lines = sorted.map((p) => `${p}:${sha(files.get(p)!)}\n`).join("");
  const version = `v${crypto.createHash("sha256").update(lines).digest("hex").slice(0, 16)}`;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cast-iron-e2e-v2-"));
  for (const p of sorted) {
    const target = path.join(dir, "content", version, p);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, files.get(p)!);
  }
  const manifest = {
    ...old,
    content_version: version,
    total_bytes: sorted.reduce((n, p) => n + files.get(p)!.length, 0),
    files: sorted.map((p) => ({ path: `${version}/${p}`, sha256: sha(files.get(p)!), bytes: files.get(p)!.length })),
  };
  fs.writeFileSync(path.join(dir, "content/manifest.json"), JSON.stringify(manifest));
  return { dir, version, manifest, cleanup: () => fs.rmSync(dir, { recursive: true, force: true }) };
}

/**
 * A manifest overlay that leaves out `books.json`, so the client installs a library with no
 * book order. The legacy `#/browse/source/<id>` route then renders its kind groups instead of
 * the book doorway, which lets the acceptance test compare the header against those links.
 * The content version stays the same, so the unchanged files keep coming from dist.
 */
export function buildLibraryWithoutBooks() {
  const old = distManifest();
  const books = old.files.filter((f) => f.path.endsWith("/books.json"));
  const files = old.files.filter((f) => !f.path.endsWith("/books.json"));
  const total_bytes = old.total_bytes - books.reduce((n, f) => n + f.bytes, 0);
  const manifest = { ...old, total_bytes, files };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cast-iron-e2e-no-books-"));
  fs.mkdirSync(path.join(dir, "content"), { recursive: true });
  fs.writeFileSync(path.join(dir, "content/manifest.json"), JSON.stringify(manifest));
  return { dir, manifest, cleanup: () => fs.rmSync(dir, { recursive: true, force: true }) };
}

/**
 * A manifest overlay that leaves out `backlinks.json` and its shards and presents the library
 * as schema 3, so the client sees a library built before the reverse index existed. The
 * content version stays the same, so every other file keeps coming from dist.
 */
export function buildLibraryWithoutBacklinks() {
  const old = distManifest();
  const isBacklink = (p: string) => /\/backlinks\.json$/.test(p) || /\/backlinks\//.test(p);
  const removed = old.files.filter((f) => isBacklink(f.path));
  const files = old.files.filter((f) => !isBacklink(f.path));
  const total_bytes = old.total_bytes - removed.reduce((n, f) => n + f.bytes, 0);
  const manifest = { ...old, schema_version: 3, total_bytes, files };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cast-iron-e2e-no-backlinks-"));
  fs.mkdirSync(path.join(dir, "content"), { recursive: true });
  fs.writeFileSync(path.join(dir, "content/manifest.json"), JSON.stringify(manifest));
  return { dir, manifest, cleanup: () => fs.rmSync(dir, { recursive: true, force: true }) };
}

/**
 * A manifest-only overlay with a different content version, for the automatic update check.
 * The files are the served library's with the new version prefix, so the client accepts the
 * manifest. None of those files exists: the check must never download anything, and the test
 * never taps Download.
 */
export function buildNewerManifest() {
  const old = distManifest();
  const version = `vnext${Date.now().toString(36)}`;
  const prefix = `${old.content_version}/`;
  const files = old.files.map((file) => ({
    ...file,
    path: `${version}/${file.path.slice(prefix.length)}`,
  }));
  const manifest = { ...old, content_version: version, files };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cast-iron-e2e-manifest-"));
  fs.mkdirSync(path.join(dir, "content"), { recursive: true });
  fs.writeFileSync(path.join(dir, "content/manifest.json"), JSON.stringify(manifest));
  return { dir, version, manifest, cleanup: () => fs.rmSync(dir, { recursive: true, force: true }) };
}

/**
 * Open every priority-tier group on the Books page. Only Rulebooks start open, so a test that
 * reaches a book by title opens the groups first. Waits for the groups to be in the document.
 */
export async function openBookGroups(page: Page) {
  await expect(page.locator(".book-tier").first()).toBeVisible();
  const closed = page.locator('.tier-toggle[aria-expanded="false"]');
  // Each click removes one match, so always take the first.
  for (let left = await closed.count(); left > 0; left--) await closed.first().click();
  await expect(page.locator('.tier-toggle[aria-expanded="false"]')).toHaveCount(0);
}

/** Follow the publication directory through its row link to the book page. */
export async function openBookContents(page: Page, title: string) {
  await page.goto("#/books");
  await openBookGroups(page);
  const row = page.locator("a.book-directory-row").filter({ has: page.locator(".cat-name", { hasText: new RegExp(`^${title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }) });
  await row.click();
  // A route change moves focus to <main> on the next animation frame so screen readers announce the
  // page. A key pressed on a contents control before that move lands on <main> instead, and the
  // control never reacts. Wait for the move rather than racing it.
  await expect(page.getByRole("heading", { level: 1, name: title, exact: true })).toBeVisible();
  await expect(page.locator("main#main")).toBeFocused();
}

/**
 * Let a pending route change finish its focus move. The app moves focus to <main> one animation
 * frame after a route change; typing before that frame can lose keys to <main>. Two frames
 * guarantee the move has run, without depending on whether the route actually changed.
 */
export async function afterRouteFrame(page: Page) {
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
}

/** Click "Show more results" until the whole grouped result list is on screen. */
export async function showAllResults(page: Page) {
  await expect(page.locator(".results .hit").first()).toBeVisible();
  const more = page.getByRole("button", { name: "Show more results" });
  for (let i = 0; i < 60; i++) {
    if (!(await more.isVisible().catch(() => false))) return;
    const before = await page.locator(".results .hit").count();
    await more.click();
    await expect.poll(() => page.locator(".results .hit").count()).toBeGreaterThan(before);
  }
}

/** Open a publication choice before following its permanent entry link. */
export async function revealSourceChoice(link: import("@playwright/test").Locator) {
  await expect(link).toBeAttached();
  const disclosure = link.locator("xpath=ancestor::details[1]");
  if (await disclosure.count() && !(await disclosure.evaluate((el: HTMLDetailsElement) => el.open))) {
    await disclosure.locator(":scope > summary").click();
  }
}

/**
 * Corpus selection. Expectations must come from the library that was built, not from one
 * dataset's ids or titles, so a test that needs "a spell with a stat block" or "a class"
 * asks the compiled corpus and uses what it returns.
 */
export function corpusEntry(kind: string, title: string): Entry {
  const found = [...loadCorpus().entries.values()].find((e) => e.kind === kind && e.title === title);
  if (!found) throw new Error(`The compiled corpus has no ${kind} titled ${JSON.stringify(title)}.`);
  return found;
}

export function firstEntry(what: string, predicate: (entry: Entry) => boolean): Entry {
  const found = [...loadCorpus().entries.values()].find(predicate);
  if (!found) throw new Error(`The compiled corpus has no entry for ${what}.`);
  return found;
}

/** Plain text of an inline content array. */
export function inlineText(nodes: any[]): string {
  return nodes
    .map((node) => (typeof node.v === "string" ? node.v : inlineText(node.c ?? [])))
    .join("");
}

/** Entry links in a body, in document order: the target id and its optional heading anchor. */
export function bodyLinks(body: any[]): { target: string; anchor?: string; text: string }[] {
  const found: { target: string; anchor?: string; text: string }[] = [];
  const walk = (value: any) => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (!value || typeof value !== "object") return;
    if (value.t === "link" && typeof value.target === "string") {
      found.push({ target: value.target, anchor: value.anchor, text: inlineText(value.c ?? []) });
    } else if (value.t === "entry_link" && typeof value.target === "string") {
      // Rules entries keep cross-references inline as entry links.
      found.push({ target: value.target, text: value.title ?? "" });
    }
    Object.values(value).forEach(walk);
  };
  walk(body);
  return found;
}

/** All text of an entry body, joined with spaces. */
export function bodyText(entry: Entry): string {
  const parts: string[] = [];
  const walk = (value: any) => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (!value || typeof value !== "object") return;
    if (typeof value.v === "string") parts.push(value.v);
    Object.values(value).forEach(walk);
  };
  walk(entry.body);
  return parts.join(" ");
}

/** True when the body contains a block of this type. */
export function hasBlock(entry: Entry, type: string): boolean {
  let found = false;
  const walk = (value: any) => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (!value || typeof value !== "object") return;
    if (value.t === type) found = true;
    Object.values(value).forEach(walk);
  };
  walk(entry.body);
  return found;
}

/** Every heading block of an entry body, in document order, with or without an anchor. */
export function bodyHeadings(entry: Entry): { anchor?: string; text: string; level: number; content: any[] }[] {
  const found: { anchor?: string; text: string; level: number; content: any[] }[] = [];
  const walk = (value: any) => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (!value || typeof value !== "object") return;
    if (value.t === "heading") {
      found.push({
        anchor: value.anchor,
        // Same text as Rust `plain_text`: a line break counts as a space.
        text: headingPlainText(value.content ?? []),
        level: value.level,
        content: value.content ?? [],
      });
    }
    Object.values(value).forEach(walk);
  };
  walk(entry.body);
  return found;
}

/** Anchored headings of an entry body, in document order. */
export function headingAnchors(entry: Entry): { anchor: string; text: string; level: number }[] {
  return bodyHeadings(entry).filter(
    (heading): heading is { anchor: string; text: string; level: number } => typeof heading.anchor === "string",
  );
}

/** Plain text as the reader derives a heading slug: a line break counts as a space. */
function headingPlainText(nodes: any[]): string {
  return nodes
    .map((node) =>
      node.t === "break" ? " " : typeof node.v === "string" ? node.v : headingPlainText(node.c ?? []),
    )
    .join("");
}

/** Lowercase `a-z0-9` words joined by `-`, as `rich::slugify` does. */
function slugify(text: string): string {
  let out = "";
  let pendingDash = false;
  for (const ch of text) {
    if (/^[A-Za-z0-9]$/.test(ch)) {
      if (pendingDash && out) out += "-";
      pendingDash = false;
      out += ch.toLowerCase();
    } else {
      pendingDash = true;
    }
  }
  return out;
}

/** Route anchors that target app elements on an entry page; content headings never take them. */
export const RESERVED_ANCHOR_KEYS = ["entry-related"];

/**
 * The URL anchor key of every heading of an entry body, in document order, or null for a heading
 * with no key. Mirrors `rich::keyed_headings`: the compiler anchor, else `sec-<slug>`; a repeated
 * key gets `-2`, `-3`, ..., skipping any literal key in the body and the reserved app keys.
 * The DOM id is `h-<key>`.
 */
export function headingKeys(entry: Entry): { key: string | null; text: string; level: number }[] {
  const headings = bodyHeadings(entry).map((heading) => {
    const slug = slugify(headingPlainText(heading.content));
    const base = heading.anchor ? heading.anchor : slug ? `sec-${slug}` : null;
    return { base, text: heading.text, level: heading.level };
  });
  const literal = new Set(headings.map((heading) => heading.base).filter((base): base is string => base !== null));
  const used = new Set(RESERVED_ANCHOR_KEYS);
  return headings.map(({ base, text, level }) => {
    if (base === null) return { key: null, text, level };
    let key = base;
    for (let suffix = 2; used.has(key); suffix++) {
      const candidate = `${base}-${suffix}`;
      if (!literal.has(candidate) && !used.has(candidate)) key = candidate;
    }
    used.add(key);
    return { key, text, level };
  });
}

export interface TocNode {
  title: string;
  entry?: string;
  anchor?: string;
  review?: string;
  children?: TocNode[];
}
export interface BookTree {
  source: string;
  title: string;
  basis: string;
  review: string[];
  chapters: TocNode[];
}

let cachedBooks: BookTree[] | undefined;
/** `books.json` of the compiled library, parsed once per worker. */
export function bookTrees(): BookTree[] {
  if (!cachedBooks) {
    const { version } = loadCorpus();
    const file = JSON.parse(fs.readFileSync(path.join(GENERATED, version, "books.json"), "utf8"));
    cachedBooks = file.books;
  }
  return cachedBooks!;
}

/**
 * The kinds that the `In this book` links reach: every kind with its own list page, which is
 * every kind except Rules (`EntryKind` in the schema).
 */
export const LINKED_KINDS = new Set([
  "spell",
  "feat",
  "creature",
  "item",
  "class",
  "prestige_class",
  "race",
  "skill",
  "affliction",
  "trap",
  "trait",
  "racial_trait",
  "archetype",
  "class_option",
]);
/**
 * Whether Contents leaves those kinds out. On since #158 shipped the links (`LEAVE_OUT_LINKED_KINDS`
 * in `views/book.rs`); the tests follow this one switch.
 */
export const CONTENTS_LEAVES_OUT_LINKED_KINDS = true;
/** The kinds Contents leaves out right now. */
export const CONTENTS_HIDDEN_KINDS: Set<string> = CONTENTS_LEAVES_OUT_LINKED_KINDS ? LINKED_KINDS : new Set();

/** The titles the top level of Contents shows, as the page writes them. */
export function listedChapterTitles(book: BookTree): string[] {
  const kinds = new Map(loadCorpus().catalog.entries.map((stub) => [stub.id, stub.kind]));
  const ids = (node: TocNode): string[] => [...(node.entry ? [node.entry] : []), ...(node.children ?? []).flatMap(ids)];
  const shown = (node: TocNode) => {
    const all = ids(node);
    return all.length === 0 || all.some((id) => !CONTENTS_HIDDEN_KINDS.has(kinds.get(id) as string));
  };
  const listed = book.chapters.filter(shown);
  const rename = (title: string) => (title === "New entries (page order)" ? "More from this book" : title);
  // A book whose only listed chapter is the catch-all shows that chapter's rows at the top.
  if (listed.length === 1 && listed[0].title === "New entries (page order)" && !listed[0].entry) {
    return (listed[0].children ?? []).filter(shown).map((node) => rename(node.title));
  }
  return listed.map((chapter) => rename(chapter.title));
}

/** The chapter path of an entry in a book tree, or null when the book does not place it. */
export function pathToEntry(nodes: TocNode[], id: string, base: number[] = []): number[] | null {
  for (const [index, node] of nodes.entries()) {
    const path = [...base, index];
    if (node.entry === id && !node.children?.length) return path;
    if (node.children?.length) {
      const found = pathToEntry(node.children, id, path);
      if (found) return found;
    }
    if (node.entry === id) return path;
  }
  return null;
}

let cachedOwners: Map<string, string> | undefined;
/**
 * The book the client shows an entry under: its first placement in `books.json` order.
 * A canonical entry can appear in several publications, and the reader takes the first.
 */
export function bookOwners(): Map<string, string> {
  if (!cachedOwners) {
    const owners = new Map<string, string>();
    for (const book of bookTrees()) {
      for (const id of bookEntryOrder(book)) if (!owners.has(id)) owners.set(id, book.source);
    }
    cachedOwners = owners;
  }
  return cachedOwners;
}

export function firstBookPlacement(entryId: string): BookTree | undefined {
  for (const book of bookTrees()) {
    if (pathToEntry(book.chapters, entryId)) return book;
  }
  return undefined;
}

/** Entries of a book in reading order: a node's own entry, then its children. */
export function bookEntryOrder(book: BookTree): string[] {
  const order: string[] = [];
  const walk = (nodes: TocNode[]) => {
    for (const node of nodes) {
      if (node.entry) order.push(node.entry);
      walk(node.children ?? []);
    }
  };
  walk(book.chapters);
  return order;
}

const tocId = (path: number[]) => `toc-${path.map((index) => index + 1).join("-")}`;

/** Follow the expanded contents row of an entry. */
export async function openBookEntry(
  page: Page,
  book: BookTree,
  entryId: string,
  { keys = false }: { keys?: boolean } = {},
) {
  const path = pathToEntry(book.chapters, entryId);
  if (!path) throw new Error(`${book.title} does not place ${entryId}.`);
  const activate = (locator: import("@playwright/test").Locator) =>
    keys ? locator.press("Enter") : locator.click();
  await openBookContents(page, book.title);
  const id = tocId(path);
  await activate(page.locator(`a#${id}`));
}

/**
 * Replaces the stored recent history and stars with the given entries, then opens Home on a
 * fresh load. Writes straight to IndexedDB so a test need not browse to fill the lists.
 */
export async function seedHome(
  page: Page,
  { recents = [], stars = [] }: { recents?: { id: string; title: string }[]; stars?: { id: string; title: string; kind: string }[] },
) {
  // Stage on Legal, not Settings: Settings has its own storage and service worker reads in flight
  // for a moment after it renders, and a reload that stops the page under them is reported by
  // WebKit as the page error "Context is stopped". Legal reads nothing after it renders.
  await page.goto("#/legal");
  await page.locator(".legal-text").first().waitFor({ state: "visible", timeout: 90_000 });
  await page.evaluate(
    ({ recents, stars }) =>
      new Promise<void>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const tx = db.transaction(["bookmarks", "meta"], "readwrite");
          tx.objectStore("bookmarks").clear();
          stars.forEach((s, i) =>
            tx.objectStore("bookmarks").put({
              entry_id: s.id, title_snapshot: s.title, kind: s.kind, added_at: 100 + i, collection_ids: [],
            }));
          tx.objectStore("meta").put(recents.map((r) => ({ id: r.id, title: r.title })), "recent");
          tx.oncomplete = () => { db.close(); resolve(); };
          tx.onerror = () => reject(tx.error);
        };
      }),
    { recents, stars },
  );
  await page.reload();
  await page.locator(".legal-text").first().waitFor({ state: "visible", timeout: 90_000 });
  await page.goto("#/");
  await waitReady(page);
  await page.locator("section[aria-labelledby=by-kind] a.cat").first().waitFor({ state: "visible", timeout: 90_000 });
  // Home warms the search index after it renders. A caller that leaves or reloads while that
  // Cache read is in flight is the WebKit page error "Context is stopped" (#121, #273).
  await waitHomeIdle(page);
}

/** How many recently viewed entries are in storage right now, read straight from IndexedDB. */
export function storedRecentCount(page: Page): Promise<number> {
  return page.evaluate(
    () =>
      new Promise<number>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const get = db.transaction("meta").objectStore("meta").get("recent");
          get.onsuccess = () => { db.close(); resolve(Array.isArray(get.result) ? get.result.length : 0); };
          get.onerror = () => { db.close(); reject(get.error); };
        };
      }),
  );
}

/**
 * Waits until Home has finished warming the search index. The read of `search.bin` runs for about
 * a second after Home renders; a reload that stops the page under it is reported by WebKit as the
 * page error "Context is stopped".
 */
export async function waitHomeIdle(page: Page) {
  await page.locator(".home-page[data-index-ready=true]").waitFor({ state: "attached", timeout: 90_000 });
}

/**
 * How many feats each feat leads to, directly or not, from the compiled relations: the number
 * the chain marker states. Only feats that lead to more than `moreThan` are returned.
 */
export function featLeadsTo(moreThan = 5): Map<string, number> {
  const dir = path.join(GENERATED, loadCorpus().version);
  const index = JSON.parse(fs.readFileSync(path.join(dir, "relations.json"), "utf8"));
  const leads = new Map<string, string[]>();
  for (const [source, kind, target] of index.edges as [string, string, string][]) {
    if (kind !== "prerequisite") continue;
    leads.set(target, [...(leads.get(target) ?? []), source]);
  }
  const out = new Map<string, number>();
  for (const start of leads.keys()) {
    const seen = new Set([start]);
    const stack = [start];
    while (stack.length) {
      for (const next of leads.get(stack.pop()!) ?? []) {
        if (!seen.has(next)) {
          seen.add(next);
          stack.push(next);
        }
      }
    }
    if (seen.size - 1 > moreThan) out.set(start, seen.size - 1);
  }
  return out;
}
