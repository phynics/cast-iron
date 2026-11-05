import { expect, type Page } from "@playwright/test";

/** Open the app, accept the offer, and wait for the reader. */
export async function installLibrary(page: Page) {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Install the rules library" })).toBeVisible();
  await page.getByRole("button", { name: "Download" }).click();
  await waitReady(page);
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

/** Cache name -> request paths relative to the app base (document-relative `content/...` keys). */
export async function cacheContents(page: Page): Promise<Record<string, string[]>> {
  return page.evaluate(async () => {
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
import { DIST, distManifest } from "./fixtures";

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

/** Follow the publication directory through its actual disclosure and contents link. */
export async function openBookContents(page: Page, title: string) {
  await page.goto("#/books");
  const row = page.locator(".book-directory-row").filter({ has: page.locator(".cat-name", { hasText: new RegExp(`^${title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }) });
  await row.locator("summary").click();
  await row.getByRole("link", { name: `Open ${title} contents`, exact: true }).click();
}

/** Open a publication choice before following its permanent entry link. */
export async function revealSourceChoice(link: import("@playwright/test").Locator) {
  await expect(link).toBeAttached();
  const disclosure = link.locator("xpath=ancestor::details[1]");
  if (await disclosure.count() && !(await disclosure.evaluate((el: HTMLDetailsElement) => el.open))) {
    await disclosure.locator(":scope > summary").click();
  }
}
